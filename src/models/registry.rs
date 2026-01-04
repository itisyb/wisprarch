use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::global;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub name: String,
    pub url: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub size_bytes: u64,
    pub languages: Vec<String>,
    pub speed_rating: u8,
    pub accuracy_rating: u8,
    pub files: Vec<ModelFile>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelStatus {
    NotDownloaded,
    Downloading { progress: f64 },
    Downloaded,
    Corrupted,
}

pub struct ModelRegistry {
    pub models: Vec<ModelDefinition>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: vec![
                ModelDefinition {
                    id: "parakeet-v3".into(),
                    name: "Parakeet v3 (Multilingual)".into(),
                    provider_type: "parakeet".into(),
                    size_bytes: 2_560_000_000,
                    languages: vec!["multilingual".into()],
                    speed_rating: 5,
                    accuracy_rating: 5,
                    files: vec![
                        ModelFile {
                            name: "encoder-model.onnx".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/encoder-model.onnx".into(),
                            size_bytes: 41_800_000,
                        },
                        ModelFile {
                            name: "encoder-model.onnx.data".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/encoder-model.onnx.data".into(),
                            size_bytes: 2_440_000_000,
                        },
                        ModelFile {
                            name: "decoder_joint-model.onnx".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/decoder_joint-model.onnx".into(),
                            size_bytes: 72_500_000,
                        },
                        ModelFile {
                            name: "vocab.txt".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/vocab.txt".into(),
                            size_bytes: 93_900,
                        },
                    ],
                },
                ModelDefinition {
                    id: "parakeet-v2".into(),
                    name: "Parakeet v2 (English)".into(),
                    provider_type: "parakeet".into(),
                    size_bytes: 2_520_000_000,
                    languages: vec!["en".into()],
                    speed_rating: 5,
                    accuracy_rating: 5,
                    files: vec![
                        ModelFile {
                            name: "encoder-model.onnx".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/encoder-model.onnx".into(),
                            size_bytes: 41_800_000,
                        },
                        ModelFile {
                            name: "encoder-model.onnx.data".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/encoder-model.onnx.data".into(),
                            size_bytes: 2_440_000_000,
                        },
                        ModelFile {
                            name: "decoder_joint-model.onnx".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/decoder_joint-model.onnx".into(),
                            size_bytes: 35_800_000,
                        },
                        ModelFile {
                            name: "vocab.txt".into(),
                            url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main/vocab.txt".into(),
                            size_bytes: 9_380,
                        },
                    ],
                },
            ],
        }
    }

    pub fn get_model(&self, id: &str) -> Option<&ModelDefinition> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn get_model_status(&self, id: &str) -> ModelStatus {
        let Ok(models_dir) = global::models_dir() else {
            return ModelStatus::NotDownloaded;
        };

        let model_path = models_dir.join(id);
        if !model_path.exists() {
            return ModelStatus::NotDownloaded;
        }

        let Some(model) = self.get_model(id) else {
            return ModelStatus::NotDownloaded;
        };

        for file in &model.files {
            let file_path = model_path.join(&file.name);
            if !file_path.exists() {
                return ModelStatus::Corrupted;
            }
        }

        ModelStatus::Downloaded
    }

    pub fn get_model_path(&self, id: &str) -> Option<PathBuf> {
        let models_dir = global::models_dir().ok()?;
        Some(models_dir.join(id))
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
