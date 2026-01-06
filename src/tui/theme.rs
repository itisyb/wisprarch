use ratatui::style::Color;
use std::process::Command;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub selection: Color,
    pub muted: Color,
}

impl Theme {
    pub fn catppuccin_mocha() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            accent: Color::Rgb(137, 180, 250),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            border: Color::Rgb(69, 71, 90),
            selection: Color::Rgb(49, 50, 68),
            muted: Color::Rgb(166, 173, 200),
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),
            fg: Color::Rgb(76, 79, 105),
            accent: Color::Rgb(30, 102, 245),
            success: Color::Rgb(64, 160, 43),
            warning: Color::Rgb(223, 142, 29),
            error: Color::Rgb(210, 15, 57),
            border: Color::Rgb(172, 176, 190),
            selection: Color::Rgb(204, 208, 218),
            muted: Color::Rgb(140, 143, 161),
        }
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::catppuccin_mocha(),
            ThemeMode::Light => Self::catppuccin_latte(),
            ThemeMode::System => {
                if detect_system_dark_mode() {
                    Self::catppuccin_mocha()
                } else {
                    Self::catppuccin_latte()
                }
            }
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_mode(ThemeMode::System)
    }
}

fn detect_system_dark_mode() -> bool {
    if let Ok(output) = Command::new("dbus-send")
        .args([
            "--reply-timeout=100",
            "--print-reply=literal",
            "--dest=org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings.Read",
            "string:org.freedesktop.appearance",
            "string:color-scheme",
        ])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(value) = stdout.split_whitespace().last() {
                if let Ok(scheme) = value.parse::<u32>() {
                    return scheme == 1;
                }
            }
        }
    }

    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("dark");
        }
    }

    true
}
