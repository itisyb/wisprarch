//! Recording control endpoints.
//!
//! Provides HTTP endpoints for:
//! - Toggling recording (POST /toggle)
//! - Getting recording status (GET /status)
//! - Switching input method (POST /input-method, GET /input-method)

use crate::audio::{JobOptions, RecordingPhase, RecordingStatus, RecordingStatusHandle, NUM_BANDS};
use crate::config::WaybarConfig;
use crate::text_io::{InjectionMethod, TextIoService};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Request body for the toggle recording endpoint.
/// All fields are optional - if not provided, defaults are used from config.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ToggleRequest {
    /// Whether to copy the transcription to clipboard (default: true)
    #[serde(default)]
    pub copy_to_clipboard: Option<bool>,
    /// Whether to auto-paste/inject text into the focused app (default: from config)
    #[serde(default)]
    pub auto_paste: Option<bool>,
}

#[derive(Clone)]
pub enum ApiCommand {
    /// Toggle recording with optional per-job options
    ToggleRecording(Option<JobOptions>),
}

#[derive(Clone)]
pub struct RecordingState {
    pub tx: mpsc::Sender<ApiCommand>,
    pub status: RecordingStatusHandle,
    pub waybar_config: WaybarConfig,
    pub text_io: TextIoService,
}

pub fn router(state: RecordingState) -> Router {
    Router::new()
        .route("/toggle", post(toggle_recording))
        .route("/status", get(recording_status))
        .route("/input-method", get(get_input_method))
        .route("/input-method", post(set_input_method))
        .route("/input-method/cycle", post(cycle_input_method))
        .with_state(state)
}

/// Toggles recording on or off with optional per-job options.
///
/// # Request Body
/// Optional JSON with fields:
/// - `copy_to_clipboard`: bool - Copy transcription to clipboard
/// - `auto_paste`: bool - Auto-paste/inject text into focused app
///
/// # Response
/// Returns JSON with recording status and current job information.
async fn toggle_recording(
    State(state): State<RecordingState>,
    body: Option<Json<ToggleRequest>>,
) -> Result<Json<Value>, StatusCode> {
    // Parse optional job options from request body
    let job_options = body.and_then(|Json(req)| {
        // Only create JobOptions if at least one field was specified
        if req.copy_to_clipboard.is_some() || req.auto_paste.is_some() {
            Some(JobOptions {
                copy_to_clipboard: req.copy_to_clipboard.unwrap_or(true),
                auto_paste: req.auto_paste.unwrap_or(true),
            })
        } else {
            None
        }
    });

    info!(
        "Toggle recording command received via API with options: {:?}",
        job_options
    );

    match state
        .tx
        .send(ApiCommand::ToggleRecording(job_options))
        .await
    {
        Ok(_) => {
            // Small delay to allow the status to be updated
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // Get the current status to return job information
            let status = state.status.get().await;

            Ok(Json(json!({
                "success": true,
                "phase": status.phase.as_str(),
                "job_id": status.current_job_id,
                "message": format!("Recording {}", status.phase.as_str())
            })))
        }
        Err(e) => {
            error!("Failed to send toggle command: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Gets the current recording status.
///
/// # Query Parameters
/// - `style=waybar` - Returns response formatted for Waybar integration
///
/// # Response
/// Returns JSON with current recording phase, job ID, and last completed job info.
/// When `style=waybar` is specified, returns Waybar-formatted response with text, class, and tooltip.
async fn recording_status(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<RecordingState>,
) -> Json<Value> {
    let status = state.status.get().await;

    // Check if waybar style is requested
    if params.get("style") == Some(&"waybar".to_string()) {
        return Json(generate_waybar_response(&status, &state.waybar_config));
    }

    // Build last_completed_job object if available
    let last_completed_job = status.last_completed_job.as_ref().map(|job| {
        json!({
            "job_id": job.job_id,
            "history_id": job.history_id,
            "text": job.text,
            "created_at": job.created_at
        })
    });

    // Default JSON response with full job context
    Json(json!({
        "recording": status.phase == RecordingPhase::Recording,
        "phase": status.phase.as_str(),
        "job_id": status.current_job_id,
        "last_completed_job": last_completed_job,
        "last_error": status.last_error,
        "audio_level": status.audio_level,
        "frequency_bands": status.frequency_bands,
    }))
}

fn generate_waybar_response(status: &RecordingStatus, _config: &WaybarConfig) -> Value {
    let (text, class, tooltip) = match status.phase {
        RecordingPhase::Idle => (
            String::new(), // Nothing when idle
            "wisprarch-idle".to_string(),
            "Press Super+R to record".to_string(),
        ),
        RecordingPhase::Recording => {
            let visualizer = generate_visualizer(&status.frequency_bands);
            (
                visualizer,
                "wisprarch-recording".to_string(),
                "Recording... Press Super+R to stop".to_string(),
            )
        }
        RecordingPhase::Processing => (
            "⏳".to_string(),
            "wisprarch-processing".to_string(),
            "Processing...".to_string(),
        ),
        RecordingPhase::Error => (
            "❌".to_string(),
            "wisprarch-error".to_string(),
            status
                .last_error
                .clone()
                .unwrap_or_else(|| "Error".to_string()),
        ),
    };

    json!({
        "text": text,
        "class": class,
        "tooltip": tooltip
    })
}

/// Generate a compact audio visualizer from frequency bands.
///
/// Uses braille dots for a slim, btop-style look.
fn generate_visualizer(bands: &[f32; NUM_BANDS]) -> String {
    // Braille-style vertical bars (4 heights)
    const BARS: [&str; 4] = ["⡀", "⡄", "⡆", "⡇"];

    let mut visualizer = String::new();

    // Use all 8 bands for more detail
    for &level in bands.iter() {
        let bar_idx = ((level * 4.0) as usize).min(3);
        visualizer.push_str(BARS[bar_idx]);
    }

    visualizer
}

#[derive(Debug, serde::Deserialize)]
pub struct SetInputMethodRequest {
    pub method: String,
}

async fn get_input_method(State(state): State<RecordingState>) -> Json<Value> {
    let method = state.text_io.injection_method().await;
    Json(json!({
        "method": method.as_str(),
        "available": get_available_methods()
    }))
}

async fn set_input_method(
    State(state): State<RecordingState>,
    Json(req): Json<SetInputMethodRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match InjectionMethod::parse(&req.method) {
        Some(method) => {
            state.text_io.set_injection_method(method).await;
            info!("Input method changed to: {}", method.as_str());
            Ok(Json(json!({
                "method": method.as_str(),
                "message": format!("Switched to {} mode", method.as_str())
            })))
        }
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": format!("Invalid method '{}'. Use: clipboard, wtype, or ydotool", req.method),
                "available": get_available_methods()
            })),
        )),
    }
}

async fn cycle_input_method(State(state): State<RecordingState>) -> Json<Value> {
    let new_method = state.text_io.cycle_injection_method().await;
    info!("Input method cycled to: {}", new_method.as_str());
    Json(json!({
        "method": new_method.as_str(),
        "message": format!("Switched to {} mode", new_method.as_str())
    }))
}

fn get_available_methods() -> Vec<&'static str> {
    let mut methods = vec!["clipboard"];
    if which::which("wtype").is_ok() {
        methods.push("wtype");
    }
    if which::which("ydotool").is_ok() {
        methods.push("ydotool");
    }
    methods
}
