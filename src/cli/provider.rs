use crate::cli::{ProviderCliArgs, ProviderCommand};
use crate::config::{Config, WhisperConfig};
use crate::transcription::{ProviderConfig, Transcriber};
use anyhow::{anyhow, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use tracing::info;
use which::which;

pub fn handle_provider_command(args: ProviderCliArgs) -> Result<()> {
    match args.command {
        ProviderCommand::Show => handle_show(),
        ProviderCommand::Configure => handle_configure(),
        ProviderCommand::Test => handle_test(),
    }
}

fn handle_show() -> Result<()> {
    let config = Config::load()?;
    let whisper = &config.whisper;

    println!("Current transcription provider configuration:");
    println!("-------------------------------------------");
    println!(
        "Provider: {}",
        whisper.provider.as_deref().unwrap_or("<not set>")
    );
    println!("Model: {}", whisper.model.as_deref().unwrap_or("<default>"));
    println!(
        "Language: {}",
        whisper.language.as_deref().unwrap_or("<default>")
    );
    println!("API key: {}", mask_secret(&whisper.api_key));
    println!("API endpoint: {}", display_value(&whisper.api_endpoint));
    println!("Command path: {}", display_value(&whisper.command_path));
    println!("Model path: {}", display_value(&whisper.model_path));
    Ok(())
}

fn handle_configure() -> Result<()> {
    if !io::stdin().is_terminal() {
        info!("Non-interactive session detected. Please edit ~/.config/audetic/config.toml manually to change providers.");
        return Ok(());
    }

    let theme = ColorfulTheme::default();
    let mut config = Config::load()?;
    println!("Audetic provider configuration wizard");
    println!("--------------------------------------");
    println!(
        "Current provider: {}",
        config.whisper.provider.as_deref().unwrap_or("<not set>")
    );

    let selection = prompt_provider_selection(&theme, config.whisper.provider.as_deref())?;
    config.whisper.provider = Some(selection.as_str().to_string());

    match selection {
        ProviderSelection::AudeticApi => configure_audetic_api(&theme, &mut config.whisper)?,
        ProviderSelection::OpenAiApi => configure_openai_api(&theme, &mut config.whisper)?,
        ProviderSelection::OpenAiCli => configure_openai_cli(&theme, &mut config.whisper)?,
        ProviderSelection::WhisperCpp => configure_whisper_cpp(&theme, &mut config.whisper)?,
    }

    config.save()?;
    println!(
        "✓ Updated transcription provider to '{}'. Restart the Audetic service to apply changes.",
        config.whisper.provider.as_deref().unwrap_or_default()
    );
    Ok(())
}

fn handle_test() -> Result<()> {
    let config = Config::load()?;
    let provider_name = config.whisper.provider.as_deref().ok_or_else(|| {
        anyhow!("No transcription provider configured. Run `audetic provider configure` first.")
    })?;

    let provider_config = provider_config_from_whisper(&config.whisper);
    Transcriber::with_provider(provider_name, provider_config)?;

    println!(
        "✓ Provider '{}' initialized successfully. Ready for recording sessions.",
        provider_name
    );
    Ok(())
}

fn configure_audetic_api(theme: &ColorfulTheme, whisper: &mut WhisperConfig) -> Result<()> {
    whisper.command_path = None;
    whisper.model_path = None;

    let api_key = prompt_secret(theme, "Audetic API key (sk-...)", whisper.api_key.as_ref())?;
    whisper.api_key = Some(api_key);

    let endpoint_default = whisper
        .api_endpoint
        .clone()
        .unwrap_or_else(|| "https://api.audetic.dev/v1/transcribe".to_string());
    whisper.api_endpoint = Some(prompt_string_with_default(
        theme,
        "API endpoint",
        &endpoint_default,
    )?);

    let model_default = whisper.model.clone().unwrap_or_else(|| "base".to_string());
    whisper.model = Some(prompt_string_with_default(
        theme,
        "Model (base, small, medium, large-v3, ...)",
        &model_default,
    )?);

    prompt_language_choice(theme, whisper, "en")?;

    Ok(())
}

fn configure_openai_api(theme: &ColorfulTheme, whisper: &mut WhisperConfig) -> Result<()> {
    whisper.command_path = None;
    whisper.model_path = None;

    let api_key = prompt_secret(theme, "OpenAI API key (sk-...)", whisper.api_key.as_ref())?;
    whisper.api_key = Some(api_key);

    let endpoint_default = whisper
        .api_endpoint
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1/audio/transcriptions".to_string());
    whisper.api_endpoint = Some(prompt_string_with_default(
        theme,
        "API endpoint",
        &endpoint_default,
    )?);

    let model_default = whisper
        .model
        .clone()
        .unwrap_or_else(|| "whisper-1".to_string());
    whisper.model = Some(prompt_string_with_default(
        theme,
        "Model (whisper-1)",
        &model_default,
    )?);

    prompt_language_choice(theme, whisper, "en")?;

    Ok(())
}

fn configure_openai_cli(theme: &ColorfulTheme, whisper: &mut WhisperConfig) -> Result<()> {
    whisper.api_key = None;
    whisper.api_endpoint = None;
    whisper.model_path = None;

    let default_path = whisper
        .command_path
        .clone()
        .or_else(|| detect_default_binary("whisper"));
    whisper.command_path = Some(prompt_required_path(
        theme,
        "Path to `whisper` CLI binary",
        default_path,
        true,
    )?);

    let model_default = whisper.model.clone().unwrap_or_else(|| "base".to_string());
    whisper.model = Some(prompt_string_with_default(
        theme,
        "Model (tiny, base, small, medium, large-v3, ...)",
        &model_default,
    )?);

    prompt_language_choice(theme, whisper, "en")?;

    Ok(())
}

fn configure_whisper_cpp(theme: &ColorfulTheme, whisper: &mut WhisperConfig) -> Result<()> {
    whisper.api_key = None;
    whisper.api_endpoint = None;

    let command_default = whisper.command_path.clone();
    whisper.command_path = Some(prompt_required_path(
        theme,
        "Path to whisper.cpp binary",
        command_default,
        true,
    )?);

    let model_path_default = whisper.model_path.clone();
    whisper.model_path = Some(prompt_required_path(
        theme,
        "Path to GGML/GGUF model file",
        model_path_default,
        true,
    )?);

    let model_default = whisper.model.clone().unwrap_or_else(|| "base".to_string());
    whisper.model = Some(prompt_string_with_default(
        theme,
        "Model size label (tiny, base, small, medium, large)",
        &model_default,
    )?);

    prompt_language_choice(theme, whisper, "en")?;

    Ok(())
}

fn prompt_provider_selection(
    theme: &ColorfulTheme,
    current: Option<&str>,
) -> Result<ProviderSelection> {
    const OPTIONS: &[(&str, &str)] = &[
        ("audetic-api", "Audetic Cloud API (default)"),
        ("openai-api", "OpenAI Whisper API"),
        ("openai-cli", "Local OpenAI Whisper CLI"),
        ("whisper-cpp", "Local whisper.cpp binary"),
    ];

    let items: Vec<String> = OPTIONS
        .iter()
        .map(|(name, desc)| format!("{:<12} — {}", name, desc))
        .collect();

    let default_index = current
        .and_then(|value| OPTIONS.iter().position(|(name, _)| *name == value))
        .unwrap_or(0);

    let selection = Select::with_theme(theme)
        .with_prompt("Select a transcription provider")
        .items(&items)
        .default(default_index)
        .interact()?;

    Ok(ProviderSelection::from_index(selection))
}

fn prompt_secret(theme: &ColorfulTheme, prompt: &str, current: Option<&String>) -> Result<String> {
    if let Some(existing) = current {
        let keep = Confirm::with_theme(theme)
            .with_prompt(format!("Keep existing {}?", prompt))
            .default(true)
            .interact()?;
        if keep {
            return Ok(existing.clone());
        }
    }

    loop {
        let value = Password::new().with_prompt(prompt).interact()?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            println!("{} cannot be empty.", prompt);
            continue;
        }
        return Ok(trimmed.to_string());
    }
}

fn prompt_string_with_default(theme: &ColorfulTheme, label: &str, current: &str) -> Result<String> {
    let prompt = format!("{label} [{current}]");
    let value: String = Input::with_theme(theme)
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;

    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(current.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_language_choice(
    theme: &ColorfulTheme,
    whisper: &mut WhisperConfig,
    fallback: &str,
) -> Result<()> {
    let current = whisper
        .language
        .clone()
        .unwrap_or_else(|| fallback.to_string());

    let prompt = format!("Language code (ISO 639-1, e.g. en, es, auto) [{current}]");
    let value: String = Input::with_theme(theme)
        .with_prompt(prompt)
        .allow_empty(true)
        .interact_text()?;

    let trimmed = value.trim();
    if trimmed.is_empty() {
        whisper.language = Some(current);
    } else {
        whisper.language = Some(trimmed.to_string());
    }
    Ok(())
}

fn prompt_required_path(
    theme: &ColorfulTheme,
    label: &str,
    default: Option<String>,
    require_file: bool,
) -> Result<String> {
    loop {
        let prompt = match &default {
            Some(value) => format!("{label} [{value}]"),
            None => label.to_string(),
        };

        let value: String = Input::with_theme(theme)
            .with_prompt(prompt)
            .allow_empty(default.is_some())
            .interact_text()?;

        let candidate = if value.trim().is_empty() {
            if let Some(def) = &default {
                def.clone()
            } else {
                println!("Value cannot be empty.");
                continue;
            }
        } else {
            value.trim().to_string()
        };

        if validate_path(&candidate, require_file) {
            return Ok(candidate);
        } else {
            println!(
                "Path '{}' does not exist or is not accessible. Please try again.",
                candidate
            );
        }
    }
}

fn validate_path(path: &str, require_file: bool) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => {
            if require_file {
                metadata.is_file()
            } else {
                true
            }
        }
        Err(_) => Path::new(path).exists(),
    }
}

fn detect_default_binary(program: &str) -> Option<String> {
    which(program)
        .ok()
        .map(|path| path.to_string_lossy().to_string())
}

fn provider_config_from_whisper(whisper: &WhisperConfig) -> ProviderConfig {
    ProviderConfig {
        model: whisper.model.clone(),
        model_path: whisper.model_path.clone(),
        language: whisper.language.clone(),
        command_path: whisper.command_path.clone(),
        api_endpoint: whisper.api_endpoint.clone(),
        api_key: whisper.api_key.clone(),
    }
}

fn display_value(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<not set>".to_string())
}

fn mask_secret(value: &Option<String>) -> String {
    match value {
        Some(secret) if secret.len() > 8 => {
            let prefix = &secret[..4];
            let suffix = &secret[secret.len() - 2..];
            format!("{prefix}****{suffix}")
        }
        Some(secret) if !secret.is_empty() => "*".repeat(secret.len()),
        _ => "<not set>".to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
enum ProviderSelection {
    AudeticApi,
    OpenAiApi,
    OpenAiCli,
    WhisperCpp,
}

impl ProviderSelection {
    fn as_str(&self) -> &'static str {
        match self {
            ProviderSelection::AudeticApi => "audetic-api",
            ProviderSelection::OpenAiApi => "openai-api",
            ProviderSelection::OpenAiCli => "openai-cli",
            ProviderSelection::WhisperCpp => "whisper-cpp",
        }
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => ProviderSelection::AudeticApi,
            1 => ProviderSelection::OpenAiApi,
            2 => ProviderSelection::OpenAiCli,
            _ => ProviderSelection::WhisperCpp,
        }
    }
}
