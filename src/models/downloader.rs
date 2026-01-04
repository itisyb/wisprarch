use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

use super::registry::ModelRegistry;
use crate::global;

pub struct DownloadProgress {
    pub file_name: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: f64,
}

pub struct ModelDownloader {
    client: reqwest::Client,
    registry: ModelRegistry,
}

impl ModelDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            registry: ModelRegistry::new(),
        }
    }

    pub async fn download_model<F>(&self, model_id: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(DownloadProgress) + Send + Sync,
    {
        let model = self
            .registry
            .get_model(model_id)
            .context(format!("Model '{}' not found in registry", model_id))?;

        let models_dir = global::models_dir()?;
        let model_dir = models_dir.join(model_id);

        tokio::fs::create_dir_all(&model_dir)
            .await
            .context("Failed to create model directory")?;

        info!("Downloading {} to {:?}", model.name, model_dir);

        for file in &model.files {
            self.download_file(
                &file.url,
                &model_dir.join(&file.name),
                &file.name,
                file.size_bytes,
                &progress_callback,
            )
            .await?;
        }

        info!("Model {} downloaded successfully", model_id);
        Ok(())
    }

    async fn download_file<F>(
        &self,
        url: &str,
        path: &Path,
        file_name: &str,
        expected_size: u64,
        progress_callback: &F,
    ) -> Result<()>
    where
        F: Fn(DownloadProgress),
    {
        info!("Downloading {} from {}", file_name, url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context(format!("Failed to fetch {}", url))?;

        let total = response.content_length().unwrap_or(expected_size);

        let mut file = tokio::fs::File::create(path)
            .await
            .context(format!("Failed to create file {:?}", path))?;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let start = std::time::Instant::now();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading response chunk")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write chunk to file")?;

            downloaded += chunk.len() as u64;
            let elapsed = start.elapsed().as_secs_f64();
            let speed_bps = if elapsed > 0.0 {
                downloaded as f64 / elapsed
            } else {
                0.0
            };

            progress_callback(DownloadProgress {
                file_name: file_name.to_string(),
                downloaded,
                total,
                speed_bps,
            });
        }

        file.flush().await?;
        debug!("Downloaded {} ({} bytes)", file_name, downloaded);

        Ok(())
    }

    pub async fn delete_model(&self, model_id: &str) -> Result<()> {
        let models_dir = global::models_dir()?;
        let model_dir = models_dir.join(model_id);

        if model_dir.exists() {
            tokio::fs::remove_dir_all(&model_dir)
                .await
                .context(format!("Failed to delete model directory {:?}", model_dir))?;
            info!("Deleted model: {}", model_id);
        }

        Ok(())
    }
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new()
    }
}
