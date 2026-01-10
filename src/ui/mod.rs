pub mod sounds;

use anyhow::Result;
use tracing::{info, warn};

#[derive(Clone)]
pub struct Indicator {
    audio_feedback_enabled: bool,
    start_sound_path: Option<String>,
    complete_sound_path: Option<String>,
}

impl Default for Indicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator {
    pub fn new() -> Self {
        Self {
            audio_feedback_enabled: true,
            start_sound_path: None,
            complete_sound_path: None,
        }
    }

    pub fn with_audio_feedback(mut self, enabled: bool) -> Self {
        self.audio_feedback_enabled = enabled;
        self
    }

    pub fn with_custom_sounds(mut self, start: Option<String>, complete: Option<String>) -> Self {
        self.start_sound_path = start;
        self.complete_sound_path = complete;
        self
    }

    pub async fn show_recording(&self) -> Result<()> {
        info!("Recording started");
        if self.audio_feedback_enabled {
            let path = self.start_sound_path.clone();
            tokio::task::spawn_blocking(move || {
                sounds::play_start_sound(path.as_deref());
            });
        }
        Ok(())
    }

    pub async fn show_processing(&self) -> Result<()> {
        info!("Processing started");
        // Ensure processing animation is visible for at least 500ms
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Ok(())
    }

    pub async fn show_complete(&self, text: &str) -> Result<()> {
        info!("Transcription complete: {} chars", text.len());
        if self.audio_feedback_enabled {
            let path = self.complete_sound_path.clone();
            tokio::task::spawn_blocking(move || {
                sounds::play_complete_sound(path.as_deref());
            });
        }
        Ok(())
    }

    pub async fn show_error(&self, error: &str) -> Result<()> {
        warn!("Error: {}", error);
        Ok(())
    }
}
