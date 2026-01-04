use anyhow::{Context, Result};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tracing::{debug, error, info, warn};

use super::TranscriptionProvider;
use crate::normalizer::TranscriptionNormalizer;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
    r#type: Option<String>,
    code: Option<String>,
}

pub struct GroqProvider {
    client: reqwest::Client,
    api_key: String,
    endpoint: String,
    model: String,
}

impl GroqProvider {
    pub fn new(api_key: String, model: Option<String>) -> Result<Self> {
        let client = reqwest::Client::new();
        let endpoint = "https://api.groq.com/openai/v1/audio/transcriptions".to_string();
        let model = model.unwrap_or_else(|| "whisper-large-v3-turbo".to_string());

        info!("Initialized Groq provider with model: {}", model);

        Ok(Self {
            client,
            api_key,
            endpoint,
            model,
        })
    }
}

impl TranscriptionProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "Groq Cloud"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn transcribe<'a>(
        &'a self,
        audio_path: &'a Path,
        language: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            info!("Transcribing audio file via Groq API: {:?}", audio_path);

            let audio_data = tokio::fs::read(audio_path)
                .await
                .context("Failed to read audio file")?;

            let filename = audio_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("audio.wav");

            let audio_part = Part::bytes(audio_data)
                .file_name(filename.to_string())
                .mime_str("audio/wav")
                .context("Failed to set MIME type")?;

            let mut form = Form::new()
                .part("file", audio_part)
                .text("model", self.model.clone())
                .text("response_format", "json");

            if !language.is_empty() && language != "auto" {
                form = form.text("language", language.to_string());
            }

            debug!(
                "Sending request to Groq API with model: {}, language: {}",
                self.model, language
            );

            let response = self
                .client
                .post(&self.endpoint)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .multipart(form)
                .send()
                .await
                .context("Failed to send request to Groq API")?;

            let status = response.status();

            // Handle rate limiting
            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);

                warn!(
                    "Groq API rate limit hit. Retry after {} seconds",
                    retry_after
                );
                return Err(anyhow::anyhow!(
                    "Rate limit exceeded. Please retry after {} seconds",
                    retry_after
                ));
            }

            let response_text = response
                .text()
                .await
                .context("Failed to read response body")?;

            if !status.is_success() {
                error!(
                    "Groq API request failed with status {}: {}",
                    status, response_text
                );

                if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
                    return Err(anyhow::anyhow!(
                        "Groq API error: {} (type: {:?}, code: {:?})",
                        error_response.error.message,
                        error_response.error.r#type,
                        error_response.error.code
                    ));
                }

                return Err(anyhow::anyhow!(
                    "Groq API request failed with status {}: {}",
                    status,
                    response_text
                ));
            }

            let transcription: TranscriptionResponse = serde_json::from_str(&response_text)
                .context("Failed to parse transcription response")?;

            let text = transcription.text.trim().to_string();
            info!("Transcription complete: {} chars", text.len());
            debug!("Raw transcription: {}", text);

            Ok(text)
        })
    }

    fn normalizer(&self) -> Result<Box<dyn TranscriptionNormalizer>> {
        Ok(Box::new(GroqNormalizer::new()))
    }
}

struct GroqNormalizer;

impl GroqNormalizer {
    fn new() -> Self {
        Self
    }
}

impl TranscriptionNormalizer for GroqNormalizer {
    fn normalize(&self, raw_output: &str) -> String {
        raw_output.trim().to_string()
    }

    fn name(&self) -> &'static str {
        "GroqNormalizer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_normalizer() {
        let normalizer = GroqNormalizer::new();

        let input = "  This is clean text  ";
        let expected = "This is clean text";

        assert_eq!(normalizer.normalize(input), expected);
    }

    #[test]
    fn test_groq_normalizer_empty() {
        let normalizer = GroqNormalizer::new();

        let input = "   ";
        let expected = "";

        assert_eq!(normalizer.normalize(input), expected);
    }

    #[test]
    fn test_groq_normalizer_name() {
        let normalizer = GroqNormalizer::new();
        assert_eq!(normalizer.name(), "GroqNormalizer");
    }

    #[test]
    fn test_groq_provider_new_default_model() {
        let provider = GroqProvider::new("test_key".to_string(), None).unwrap();
        assert_eq!(provider.model, "whisper-large-v3-turbo");
        assert_eq!(provider.name(), "Groq Cloud");
    }

    #[test]
    fn test_groq_provider_new_custom_model() {
        let provider =
            GroqProvider::new("test_key".to_string(), Some("whisper-large-v3".to_string()))
                .unwrap();
        assert_eq!(provider.model, "whisper-large-v3");
    }

    #[test]
    fn test_groq_provider_is_available() {
        let provider = GroqProvider::new("test_key".to_string(), None).unwrap();
        assert!(provider.is_available());

        let provider_empty = GroqProvider::new("".to_string(), None).unwrap();
        assert!(!provider_empty.is_available());
    }
}
