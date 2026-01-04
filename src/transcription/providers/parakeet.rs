use anyhow::{Context, Result};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tracing::{debug, info};

use super::TranscriptionProvider;
use crate::global;
use crate::normalizer::TranscriptionNormalizer;

/// Parakeet model variants
pub enum ParakeetModel {
    /// TDT v2 - English only, optimized for speed
    V2English,
    /// TDT v3 - Multilingual support
    V3Multilingual,
}

impl ParakeetModel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "parakeet-v2" | "v2" | "english" => Some(Self::V2English),
            "parakeet-v3" | "v3" | "multilingual" => Some(Self::V3Multilingual),
            _ => None,
        }
    }

    /// Get the directory name for this model
    pub fn model_dir_name(&self) -> &'static str {
        match self {
            Self::V2English => "parakeet-v2",
            Self::V3Multilingual => "parakeet-v3",
        }
    }

    /// Get human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::V2English => "Parakeet v2 (English)",
            Self::V3Multilingual => "Parakeet v3 (Multilingual)",
        }
    }
}

/// Parakeet speech-to-text provider using parakeet-rs
///
/// Uses NVIDIA's Parakeet TDT models via ONNX runtime for fast,
/// accurate transcription. Supports both English-only (v2) and
/// multilingual (v3) variants.
pub struct ParakeetProvider {
    model_path: PathBuf,
    model_type: ParakeetModel,
}

impl ParakeetProvider {
    /// Create a new Parakeet provider
    ///
    /// # Arguments
    /// * `model_variant` - Model variant: "parakeet-v2" or "parakeet-v3"
    ///
    /// # Errors
    /// Returns error if model files are not found. Models must be downloaded
    /// first using the model manager.
    pub fn new(model_variant: &str) -> Result<Self> {
        let model_type = ParakeetModel::parse(model_variant).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown Parakeet model variant '{}'. Use 'parakeet-v2' or 'parakeet-v3'",
                model_variant
            )
        })?;

        let models_dir = global::models_dir()?;
        let model_path = models_dir.join(model_type.model_dir_name());

        // Check if model directory exists
        if !model_path.exists() {
            return Err(anyhow::anyhow!(
                "Parakeet model not found at {:?}. Run 'wisprarch models download {}' first.",
                model_path,
                model_type.model_dir_name()
            ));
        }

        // Verify required files exist (check for both FP32 and INT8 variants)
        let required_onnx_files = ["encoder-model", "decoder_joint-model"];
        for file_base in required_onnx_files {
            let fp32_path = model_path.join(format!("{}.onnx", file_base));
            let int8_path = model_path.join(format!("{}.int8.onnx", file_base));

            if !fp32_path.exists() && !int8_path.exists() {
                return Err(anyhow::anyhow!(
                    "Required model file '{}.onnx' (or INT8 variant) not found in {:?}. Model may be corrupted or incomplete.",
                    file_base,
                    model_path
                ));
            }
        }

        // Check for vocab file
        let vocab_path = model_path.join("vocab.txt");
        if !vocab_path.exists() {
            return Err(anyhow::anyhow!(
                "Required vocab.txt not found in {:?}. Model may be corrupted or incomplete.",
                model_path
            ));
        }

        info!(
            "Initialized {} provider from {:?}",
            model_type.display_name(),
            model_path
        );

        Ok(Self {
            model_path,
            model_type,
        })
    }
}

impl TranscriptionProvider for ParakeetProvider {
    fn name(&self) -> &'static str {
        match self.model_type {
            ParakeetModel::V2English => "Parakeet v2 (English)",
            ParakeetModel::V3Multilingual => "Parakeet v3 (Multilingual)",
        }
    }

    fn is_available(&self) -> bool {
        self.model_path.exists()
    }

    fn transcribe<'a>(
        &'a self,
        audio_path: &'a Path,
        _language: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            info!("Transcribing with {}: {:?}", self.name(), audio_path);

            // parakeet-rs is synchronous, so we spawn_blocking to avoid blocking the async runtime
            let model_path = self.model_path.clone();
            let audio_path = audio_path.to_path_buf();

            let result = tokio::task::spawn_blocking(move || {
                use parakeet_rs::{ParakeetTDT, Transcriber};

                let mut parakeet = ParakeetTDT::from_pretrained(&model_path, None)
                    .context("Failed to load Parakeet model")?;

                let result = parakeet
                    .transcribe_file(&audio_path, None)
                    .context("Failed to transcribe audio")?;

                Ok::<String, anyhow::Error>(result.text)
            })
            .await
            .context("Parakeet transcription task panicked")??;

            info!("Transcription complete: {} chars", result.len());
            debug!("Transcription: {}", result);

            Ok(result)
        })
    }

    fn normalizer(&self) -> Result<Box<dyn TranscriptionNormalizer>> {
        Ok(Box::new(ParakeetNormalizer::new()))
    }
}

/// Normalizer for Parakeet transcription output
struct ParakeetNormalizer;

impl ParakeetNormalizer {
    fn new() -> Self {
        Self
    }
}

impl TranscriptionNormalizer for ParakeetNormalizer {
    fn normalize(&self, raw_output: &str) -> String {
        // Parakeet output is generally clean, just trim whitespace
        raw_output.trim().to_string()
    }

    fn name(&self) -> &'static str {
        "ParakeetNormalizer"
    }
}
