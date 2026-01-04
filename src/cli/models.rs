use anyhow::Result;
use tracing::info;

use crate::cli::args::{ModelsCliArgs, ModelsCommand};

pub async fn handle_models_command(args: ModelsCliArgs) -> Result<()> {
    match args.command {
        Some(ModelsCommand::List) => {
            list_models().await?;
        }
        Some(ModelsCommand::Download { model_id }) => {
            download_model(&model_id).await?;
        }
        Some(ModelsCommand::Delete { model_id, force }) => {
            delete_model(&model_id, force).await?;
        }
        Some(ModelsCommand::Info { model_id }) => {
            show_model_info(&model_id).await?;
        }
        None => {
            list_models().await?;
        }
    }
    Ok(())
}

async fn list_models() -> Result<()> {
    println!("\n󰓃 WisprArch Models\n");
    println!("┌─ Local Models ────────────────────────────────────────────┐");
    println!("│                                                           │");
    println!("│  parakeet-v3    Multilingual (25 langs)   ~2.5 GB        │");
    println!("│  parakeet-v2    English only              ~2.5 GB        │");
    println!("│                                                           │");
    println!("└───────────────────────────────────────────────────────────┘");
    println!();
    println!("┌─ Cloud Providers ───────────────────────────────────────────┐");
    println!("│                                                             │");
    println!("│  groq           Groq Cloud (whisper-large-v3-turbo)        │");
    println!("│  openai-api     OpenAI Whisper API                         │");
    println!("│                                                             │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();
    println!("Use 'wisprarch models download <model_id>' to download a model.");
    println!("Use 'wisprarch provider set <provider>' to switch providers.");
    Ok(())
}

async fn download_model(model_id: &str) -> Result<()> {
    info!("Downloading model: {}", model_id);

    match model_id {
        "parakeet-v3" => {
            println!("Downloading Parakeet v3 (Multilingual)...");
            println!("This will download ~2.5 GB of model files.");
            println!();
            println!("Files needed:");
            println!("  - encoder-model.onnx (~42 MB)");
            println!("  - encoder-model.onnx.data (~2.4 GB)");
            println!("  - decoder_joint-model.onnx (~72 MB)");
            println!("  - vocab.txt (~94 KB)");
            println!();
            println!("Model manager with progress bars coming in next update!");
            println!("For now, download manually from:");
            println!("  https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx");
        }
        "parakeet-v2" => {
            println!("Downloading Parakeet v2 (English)...");
            println!("This will download ~2.5 GB of model files.");
            println!();
            println!("Model manager with progress bars coming in next update!");
            println!("For now, download manually from:");
            println!("  https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx");
        }
        _ => {
            println!("Unknown model: {}", model_id);
            println!("Available models: parakeet-v3, parakeet-v2");
        }
    }

    Ok(())
}

async fn delete_model(model_id: &str, force: bool) -> Result<()> {
    info!("Deleting model: {} (force: {})", model_id, force);

    let models_dir = crate::global::models_dir()?;
    let model_path = models_dir.join(model_id);

    if !model_path.exists() {
        println!("Model '{}' is not installed.", model_id);
        return Ok(());
    }

    if !force {
        println!(
            "Are you sure you want to delete '{}'? Use --force to confirm.",
            model_id
        );
        return Ok(());
    }

    std::fs::remove_dir_all(&model_path)?;
    println!("Deleted model: {}", model_id);

    Ok(())
}

async fn show_model_info(model_id: &str) -> Result<()> {
    match model_id {
        "parakeet-v3" => {
            println!("\n󰓃 Parakeet v3 (Multilingual)\n");
            println!("Provider:    NVIDIA NeMo");
            println!("Languages:   25 (auto-detection)");
            println!("Size:        ~2.5 GB (full) / ~652 MB (INT8)");
            println!("Speed:       ⚡⚡⚡⚡⚡ (very fast on CPU)");
            println!("Accuracy:    🎯🎯🎯🎯🎯 (WER ~4.9%)");
            println!();
            println!("Supported languages:");
            println!("  Bulgarian, Croatian, Czech, Danish, Dutch, English,");
            println!("  Estonian, Finnish, French, German, Greek, Hungarian,");
            println!("  Italian, Latvian, Lithuanian, Maltese, Polish,");
            println!("  Portuguese, Romanian, Slovak, Slovenian, Spanish,");
            println!("  Swedish, Russian, Ukrainian");
        }
        "parakeet-v2" => {
            println!("\n󰓃 Parakeet v2 (English)\n");
            println!("Provider:    NVIDIA NeMo");
            println!("Languages:   English only");
            println!("Size:        ~2.5 GB (full) / ~652 MB (INT8)");
            println!("Speed:       ⚡⚡⚡⚡⚡⚡ (fastest on CPU)");
            println!("Accuracy:    🎯🎯🎯🎯🎯 (WER ~6.05%)");
        }
        "groq" => {
            println!("\n☁️ Groq Cloud\n");
            println!("Provider:    Groq (cloud)");
            println!("Models:      whisper-large-v3-turbo, whisper-large-v3");
            println!("Speed:       ⚡⚡⚡⚡⚡⚡ (216x realtime)");
            println!("Cost:        $0.04/hour (turbo) / $0.111/hour (v3)");
            println!("Requires:    API key from console.groq.com");
        }
        _ => {
            println!("Unknown model: {}", model_id);
            println!("Available: parakeet-v3, parakeet-v2, groq");
        }
    }
    Ok(())
}
