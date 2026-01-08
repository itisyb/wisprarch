use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};
use which::which;

#[derive(Clone)]
pub struct TextIoService {
    inner: Arc<TextIoInner>,
}

struct TextIoInner {
    clipboard: Mutex<Option<Clipboard>>,
    injection_method: RwLock<InjectionMethod>,
}

impl TextIoService {
    pub fn new(preferred_method: Option<&str>) -> Result<Self> {
        let clipboard = match Clipboard::new() {
            Ok(cb) => Some(cb),
            Err(err) => {
                warn!(
                    "System clipboard backend unavailable ({}); falling back to CLI-only mode",
                    err
                );
                None
            }
        };
        let injection_method = InjectionMethod::detect(preferred_method);

        Ok(Self {
            inner: Arc::new(TextIoInner {
                clipboard: Mutex::new(clipboard),
                injection_method: RwLock::new(injection_method),
            }),
        })
    }

    pub async fn injection_method(&self) -> InjectionMethod {
        *self.inner.injection_method.read().await
    }

    pub async fn set_injection_method(&self, method: InjectionMethod) {
        info!("Switching injection method to {:?}", method);
        *self.inner.injection_method.write().await = method;
    }

    pub async fn cycle_injection_method(&self) -> InjectionMethod {
        let mut method = self.inner.injection_method.write().await;
        *method = method.next_available();
        info!("Cycled injection method to {:?}", *method);
        *method
    }

    pub async fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("Copying {} chars to clipboard", text.len());
        debug!("Text to copy: {}", text);

        let mut used_native = false;

        {
            let mut clipboard_guard = self.inner.clipboard.lock().await;
            if let Some(clipboard) = clipboard_guard.as_mut() {
                match clipboard.set_text(text) {
                    Ok(_) => {
                        used_native = true;
                    }
                    Err(err) => {
                        warn!(
                            "Primary clipboard backend failed ({}), disabling until restart",
                            err
                        );
                        *clipboard_guard = None;
                    }
                }
            } else {
                debug!("Native clipboard backend unavailable; using system clipboard tools");
            }
        }

        if !used_native {
            self.copy_with_system_backends(text).await?;
        }

        Ok(())
    }

    pub async fn inject_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("Injecting text: {} chars", text.len());
        debug!("Text to inject: {}", text);

        let method = *self.inner.injection_method.read().await;
        match method {
            InjectionMethod::Wtype => {
                self.try_with_clipboard_fallback(text, Self::inject_with_wtype)
                    .await
            }
            InjectionMethod::Ydotool => {
                self.try_with_clipboard_fallback(text, Self::inject_with_ydotool)
                    .await
            }
            InjectionMethod::Clipboard => self.simulate_paste().await,
        }
    }

    pub async fn paste_from_clipboard(&self) -> Result<()> {
        self.simulate_paste().await
    }

    async fn try_with_clipboard_fallback<F>(&self, text: &str, inject_fn: F) -> Result<()>
    where
        F: Fn(&str) -> Result<()>,
    {
        if let Err(err) = inject_fn(text) {
            warn!(
                "Direct text injection failed with {} – falling back to clipboard paste",
                err
            );
            self.copy_to_clipboard(text).await?;
            self.simulate_paste().await
        } else {
            Ok(())
        }
    }

    async fn copy_with_system_backends(&self, text: &str) -> Result<()> {
        for backend in CLIPBOARD_BACKENDS {
            if which(backend.copy_cmd).is_err() {
                continue;
            }

            let mut cmd = Command::new(backend.copy_cmd);
            cmd.args(backend.copy_args);

            if backend.use_stdin {
                cmd.stdin(Stdio::piped());
            }

            if let Ok(mut child) = cmd.spawn() {
                if backend.use_stdin {
                    if let Some(stdin) = child.stdin.as_mut() {
                        if stdin.write_all(text.as_bytes()).is_err() {
                            continue;
                        }
                    }
                }

                if let Ok(status) = child.wait() {
                    if status.success() {
                        debug!("Text copied to clipboard with {}", backend.name);
                        return Ok(());
                    }
                }
            }
        }

        Err(anyhow!(
            "No clipboard tool (wl-copy/xclip/xsel) available for fallback"
        ))
    }

    fn inject_with_wtype(text: &str) -> Result<()> {
        let output = Command::new("wtype")
            .arg(text)
            .output()
            .context("Failed to execute wtype")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("wtype failed: {}", stderr));
        }

        Ok(())
    }

    fn inject_with_ydotool(text: &str) -> Result<()> {
        let output = Command::new("ydotool")
            .arg("type")
            .arg(text)
            .output()
            .context("Failed to execute ydotool")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("ydotool failed: {}", stderr);
            return Err(anyhow!(
                "ydotool failed: {}. Make sure ydotoold is running",
                stderr
            ));
        }

        Ok(())
    }

    async fn simulate_paste(&self) -> Result<()> {
        info!("Simulating paste from clipboard");

        // Small delay to let modifier keys from the trigger shortcut be released
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Best approach: read clipboard and type it directly with wtype
        // This avoids all the modifier key issues with simulating paste shortcuts
        if which("wtype").is_ok() && which("wl-paste").is_ok() {
            if let Ok(clip_output) = Command::new("wl-paste").arg("--no-newline").output() {
                if clip_output.status.success() {
                    let text = String::from_utf8_lossy(&clip_output.stdout);
                    if !text.is_empty() {
                        if let Ok(output) =
                            Command::new("wtype").arg("--").arg(text.as_ref()).output()
                        {
                            if output.status.success() {
                                debug!("Successfully typed clipboard content with wtype");
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        // ydotool fallback: Super(125) + V(47)
        if which("ydotool").is_ok() {
            if let Ok(output) = Command::new("ydotool")
                .args(["key", "125:1", "47:1", "47:0", "125:0"])
                .output()
            {
                if output.status.success() {
                    debug!("Successfully pasted with ydotool (Super+V)");
                    return Ok(());
                }
            }
        }

        if which("xdotool").is_ok() {
            if let Ok(output) = Command::new("xdotool").args(["key", "super+v"]).output() {
                if output.status.success() {
                    debug!("Successfully pasted with xdotool (Super+V)");
                    return Ok(());
                }
            }
        }

        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            if desktop == "KDE" {
                if let Ok(output) = Command::new("qdbus")
                    .args([
                        "org.kde.klipper",
                        "/klipper",
                        "org.kde.klipper.klipper.invokeAction",
                        "paste",
                    ])
                    .output()
                {
                    if output.status.success() {
                        debug!("Successfully pasted with KDE Klipper");
                        return Ok(());
                    }
                }
            }
        }

        warn!("All paste methods failed - text remains in clipboard for manual paste");
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMethod {
    Wtype,
    Ydotool,
    Clipboard,
}

impl InjectionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            InjectionMethod::Wtype => "wtype",
            InjectionMethod::Ydotool => "ydotool",
            InjectionMethod::Clipboard => "clipboard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wtype" => Some(InjectionMethod::Wtype),
            "ydotool" => Some(InjectionMethod::Ydotool),
            "clipboard" | "paste" => Some(InjectionMethod::Clipboard),
            _ => None,
        }
    }

    pub fn next_available(&self) -> Self {
        let order = [
            InjectionMethod::Clipboard,
            InjectionMethod::Wtype,
            InjectionMethod::Ydotool,
        ];

        let current_idx = order.iter().position(|m| m == self).unwrap_or(0);

        for i in 1..=order.len() {
            let next = order[(current_idx + i) % order.len()];
            if next.is_available() {
                return next;
            }
        }

        InjectionMethod::Clipboard
    }

    fn is_available(&self) -> bool {
        match self {
            InjectionMethod::Clipboard => true,
            InjectionMethod::Wtype => which("wtype").is_ok(),
            InjectionMethod::Ydotool => which("ydotool").is_ok(),
        }
    }

    fn detect(preferred: Option<&str>) -> Self {
        if let Some(choice) = preferred {
            match choice {
                "clipboard" | "paste" => {
                    info!("Using clipboard-based injection (per config)");
                    return InjectionMethod::Clipboard;
                }
                "ydotool" if which("ydotool").is_ok() => {
                    info!("Using ydotool for text injection (per config)");
                    return InjectionMethod::Ydotool;
                }
                "wtype" if which("wtype").is_ok() => {
                    info!("Using wtype for text injection (per config)");
                    return InjectionMethod::Wtype;
                }
                other => {
                    warn!(
                        "Unknown or unavailable input_method '{}', falling back to auto-detect",
                        other
                    );
                }
            }
        }

        if std::env::var("WAYLAND_DISPLAY").is_ok() && which("wl-copy").is_ok() {
            info!("Using clipboard-based injection (Wayland detected)");
            return InjectionMethod::Clipboard;
        }

        if which("ydotool").is_ok() {
            info!("Using ydotool for text injection (auto-detected)");
            return InjectionMethod::Ydotool;
        }

        if which("wtype").is_ok() {
            info!("Using wtype for text injection (auto-detected)");
            return InjectionMethod::Wtype;
        }

        info!("Falling back to clipboard-based injection");
        InjectionMethod::Clipboard
    }
}

struct ClipboardBackend {
    name: &'static str,
    copy_cmd: &'static str,
    copy_args: &'static [&'static str],
    use_stdin: bool,
}

const CLIPBOARD_BACKENDS: &[ClipboardBackend] = &[
    ClipboardBackend {
        name: "wl-copy",
        copy_cmd: "wl-copy",
        copy_args: &[],
        use_stdin: true,
    },
    ClipboardBackend {
        name: "xclip",
        copy_cmd: "xclip",
        copy_args: &["-selection", "clipboard"],
        use_stdin: true,
    },
    ClipboardBackend {
        name: "xsel",
        copy_cmd: "xsel",
        copy_args: &["--clipboard", "--input"],
        use_stdin: true,
    },
];

/// Copy text to clipboard using system clipboard tools (synchronous version).
///
/// Uses wl-copy (Wayland), xclip, or xsel (X11) for persistent clipboard
/// storage that survives after the process exits.
///
/// This is a standalone function for use in synchronous contexts (e.g., CLI commands).
pub fn copy_to_clipboard_sync(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    for backend in CLIPBOARD_BACKENDS {
        if which(backend.copy_cmd).is_err() {
            continue;
        }

        let mut child = match Command::new(backend.copy_cmd)
            .args(backend.copy_args)
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => continue,
        };

        if let Some(stdin) = child.stdin.as_mut() {
            if stdin.write_all(text.as_bytes()).is_err() {
                continue;
            }
        }

        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    Err(anyhow!(
        "No clipboard tool available. Please install wl-copy (Wayland), xclip, or xsel (X11)."
    ))
}
