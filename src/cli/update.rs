use crate::update::{UpdateConfig, UpdateEngine, UpdateOptions};
use anyhow::{anyhow, Result};
use std::io;
use std::process::Command;

use super::args::UpdateCliArgs;

pub async fn handle_update_command(args: UpdateCliArgs) -> Result<()> {
    if args.enable && args.disable {
        return Err(anyhow!(
            "Cannot enable and disable auto-update at the same time"
        ));
    }

    let mut config = UpdateConfig::detect(args.channel.clone())?;
    config.restart_on_success = false;
    let engine = UpdateEngine::new(config)?;
    let report = engine
        .run_manual(UpdateOptions {
            channel: args.channel,
            check_only: args.check,
            force: args.force,
            enable_auto_update: args.enable,
            disable_auto_update: args.disable,
        })
        .await?;

    println!("{}", report.message);
    if let Some(remote) = report.remote_version.as_deref() {
        println!("Current: {} | Remote: {}", report.current_version, remote);
    } else {
        println!("Current: {}", report.current_version);
    }

    if report.restart_required {
        let remote = report
            .remote_version
            .as_deref()
            .unwrap_or("the newly installed version");
        match restart_user_service() {
            Ok(()) => {
                println!("wisprarch service restarted via systemd user service.");
            }
            Err(err) => {
                eprintln!("Failed to restart wisprarch automatically: {err}");
                println!(
                    "Please restart the wisprarch service manually (e.g. `systemctl --user restart wisprarch.service`) to begin running {}.",
                    remote
                );
            }
        }
    }

    Ok(())
}

fn restart_user_service() -> Result<()> {
    match Command::new("systemctl")
        .arg("--user")
        .arg("restart")
        .arg("wisprarch.service")
        .status()
    {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(anyhow!(
            "systemctl reported failure restarting wisprarch.service (exit status: {status})"
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(anyhow!(
            "systemctl binary not found in PATH, cannot restart wisprarch.service automatically"
        )),
        Err(err) => Err(anyhow!(
            "Failed to invoke systemctl --user restart wisprarch.service: {err}"
        )),
    }
}
