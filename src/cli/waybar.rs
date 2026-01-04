use super::args::{WaybarCliArgs, WaybarCommand};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn handle_waybar_command(args: WaybarCliArgs) -> Result<()> {
    match args.command {
        Some(WaybarCommand::Install) => run_install(),
        Some(WaybarCommand::Uninstall) => run_uninstall(),
        Some(WaybarCommand::Status) | None => run_status(),
    }
}

const WISPRARCH_MODULE: &str = r#"
  "custom/wisprarch": {
    "exec": "while true; do curl -s 'http://127.0.0.1:3737/status?style=waybar' 2>/dev/null || echo '{\"text\":\"󰍬\",\"class\":\"wisprarch-offline\"}'; sleep 0.2; done",
    "exec-on-event": false,
    "return-type": "json",
    "on-click": "curl -X POST http://127.0.0.1:3737/toggle",
    "on-click-right": "curl -X POST http://127.0.0.1:3737/input-method/cycle",
    "tooltip": true
  }"#;

pub fn run_install() -> Result<()> {
    let config_path = find_waybar_config()?;
    println!("Found Waybar config: {}", config_path.display());

    let content = fs::read_to_string(&config_path).context("Failed to read Waybar config")?;

    if content.contains("custom/wisprarch") {
        println!("✓ WisprArch module already installed in Waybar config");
        return Ok(());
    }

    let new_content = inject_module(&content)?;

    fs::write(&config_path, &new_content).context("Failed to write Waybar config")?;

    println!("✓ Added WisprArch module to Waybar config");

    restart_waybar();

    println!("\n✓ Waybar integration complete!");
    println!("  • Click the 󰍬 icon to toggle recording");
    println!("  • Right-click to cycle input method (paste/type)");

    Ok(())
}

pub fn run_uninstall() -> Result<()> {
    let config_path = find_waybar_config()?;
    let content = fs::read_to_string(&config_path).context("Failed to read Waybar config")?;

    if !content.contains("custom/wisprarch") {
        println!("WisprArch module not found in Waybar config");
        return Ok(());
    }

    let new_content = remove_module(&content);

    fs::write(&config_path, &new_content).context("Failed to write Waybar config")?;

    println!("✓ Removed WisprArch module from Waybar config");
    restart_waybar();

    Ok(())
}

pub fn run_status() -> Result<()> {
    let config_path = find_waybar_config();

    match config_path {
        Ok(path) => {
            let content = fs::read_to_string(&path).unwrap_or_default();
            if content.contains("custom/wisprarch") {
                println!("✓ WisprArch Waybar module is installed");
                println!("  Config: {}", path.display());
            } else {
                println!("✗ WisprArch Waybar module not installed");
                println!("  Run: wisprarch waybar install");
            }
        }
        Err(_) => {
            println!("✗ Waybar config not found");
            println!("  Expected: ~/.config/waybar/config.jsonc");
        }
    }

    Ok(())
}

fn find_waybar_config() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let candidates = [
        format!("{home}/.config/waybar/config.jsonc"),
        format!("{home}/.config/waybar/config.json"),
        format!("{home}/.config/waybar/config"),
    ];

    for path in &candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }

    Err(anyhow::anyhow!(
        "Waybar config not found. Checked:\n  {}",
        candidates.join("\n  ")
    ))
}

fn inject_module(content: &str) -> Result<String> {
    let mut result = String::new();
    let mut added_to_modules = false;
    let mut added_definition = false;
    let mut brace_depth = 0;
    let mut in_modules_right = false;
    let mut i = 0;

    while i < content.len() {
        let remaining = &content[i..];

        if remaining.starts_with("\"modules-right\"") || remaining.starts_with("\"modules-center\"")
        {
            in_modules_right = true;
        }

        if in_modules_right && remaining.starts_with('[') && !added_to_modules {
            result.push('[');
            result.push_str("\n    \"custom/wisprarch\",");
            added_to_modules = true;
            in_modules_right = false;
            i += 1;
            continue;
        }

        let c = content.chars().nth(i).unwrap();

        if c == '{' {
            brace_depth += 1;
        } else if c == '}' {
            brace_depth -= 1;

            if brace_depth == 0 && !added_definition {
                result.push(',');
                result.push_str(WISPRARCH_MODULE);
                result.push('\n');
                added_definition = true;
            }
        }

        result.push(c);
        i += 1;
    }

    Ok(result)
}

fn remove_module(content: &str) -> String {
    let mut result = content.to_string();

    result = result.replace("\"custom/wisprarch\",", "");
    result = result.replace(",\"custom/wisprarch\"", "");
    result = result.replace("\"custom/wisprarch\"", "");

    let start_marker = "\"custom/wisprarch\":";
    if let Some(start) = result.find(start_marker) {
        let after_start = &result[start..];
        let mut brace_depth = 0;
        let mut end_pos = 0;

        for (i, c) in after_start.chars().enumerate() {
            if c == '{' {
                brace_depth += 1;
            } else if c == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    end_pos = i + 1;
                    break;
                }
            }
        }

        if end_pos > 0 {
            let before = &result[..start];
            let after = &result[start + end_pos..];

            let before = before.trim_end_matches(',').trim_end();
            let after = after.trim_start_matches(',');

            result = format!("{}{}", before, after);
        }
    }

    result
}

fn restart_waybar() {
    println!("Restarting Waybar...");

    let _ = Command::new("pkill").arg("waybar").status();

    std::thread::sleep(std::time::Duration::from_millis(200));

    if Command::new("hyprctl")
        .args(["dispatch", "exec", "waybar"])
        .status()
        .is_err()
    {
        let _ = Command::new("waybar").spawn();
    }
}
