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
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut added_to_modules = false;
    let mut added_definition = false;

    for line in &mut lines {
        if !added_to_modules
            && (line.contains("\"modules-right\"") || line.contains("\"modules-center\""))
        {
            // Find '[' and get the byte position right after it
            if let Some(bracket_pos) = line.find('[') {
                // Safe: '[' is ASCII (1 byte), so bracket_pos + 1 is always a valid char boundary
                let (before, after) = line.split_at(bracket_pos + 1);
                *line = format!("{}\"custom/wisprarch\", {}", before, after.trim_start());
                added_to_modules = true;
            }
        }
    }

    let mut result = lines.join("\n");

    // Find last '}' - safe because '}' is ASCII and rfind returns byte position of ASCII char
    if let Some(last_brace) = result.rfind('}') {
        let before = &result[..last_brace];
        let before = before.trim_end();
        let before = before.trim_end_matches(',');
        result = format!("{},{}\n}}\n", before, WISPRARCH_MODULE);
        added_definition = true;
    }

    if !added_to_modules || !added_definition {
        anyhow::bail!("Could not parse Waybar config. Please add manually.");
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
        let mut end_byte_pos = 0;

        for (byte_pos, c) in after_start.char_indices() {
            if c == '{' {
                brace_depth += 1;
            } else if c == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    end_byte_pos = byte_pos + c.len_utf8();
                    break;
                }
            }
        }

        if end_byte_pos > 0 {
            let before = &result[..start];
            let after = &result[start + end_byte_pos..];

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
