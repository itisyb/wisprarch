# 󰓃 WisprArch

Fast speech-to-text for Arch Linux / Hyprland with Groq Cloud, Parakeet, and Whisper models.

## Features

- **Groq Cloud** - Blazing fast transcription (216x realtime) via Whisper API
- **Parakeet v2/v3** - Local NVIDIA models for offline use (25 languages)
- **Auto-paste** - Transcribed text automatically pasted at cursor
- **TUI** - Beautiful terminal interface with Catppuccin theme
- **Hyprland Integration** - Global hotkey support, Waybar module

## Quick Install

Run the interactive installer:

```bash
bunx github:itisyb/create-wisprarch
```

The installer will guide you through:
- 🎙️ Choosing a transcription provider (Groq, OpenAI, Parakeet, Whisper.cpp)
- 🔑 Setting up API keys (if using cloud providers)
- ⌨️ Configuring your keyboard shortcut
- 🔊 Audio feedback preferences
- 📊 Waybar integration

<details>
<summary><strong>Manual Installation</strong></summary>

### Requirements

- **Arch Linux** with Hyprland/Wayland
- **wtype** or **ydotool** for auto-paste:

```bash
sudo pacman -S wtype wl-clipboard
```

### Build from Source

```bash
git clone https://github.com/itisyb/wisprarch
cd wisprarch
./install.sh
```

### Configure

```bash
wisprarch provider configure
wisprarch keybind
```

### Start Service

```bash
systemctl --user enable --now wisprarch
```

</details>

## Usage

Press your configured hotkey (default: **Super + R**) to:
1. **Start recording** - Speak your text
2. **Stop recording** - Press the hotkey again
3. **Auto-paste** - Text appears at your cursor

## TUI Model Manager

```bash
wisprarch tui
```

## Providers

| Provider | Speed | Offline | Cost |
|----------|-------|---------|------|
| `groq` | ⚡⚡⚡⚡⚡⚡ | No | $0.04/hr |
| `parakeet-v3` | ⚡⚡⚡⚡⚡ | Yes | Free |
| `parakeet-v2` | ⚡⚡⚡⚡⚡ | Yes | Free |
| `openai-api` | ⚡⚡⚡ | No | $0.36/hr |
| `whisper-cpp` | ⚡⚡ | Yes | Free |

## Commands

```bash
wisprarch                    # Run daemon
wisprarch tui                # Model manager TUI
wisprarch models list        # List models
wisprarch models download <id>  # Download model
wisprarch provider configure # Configure provider
wisprarch provider show      # Show current config
wisprarch keybind            # Set up hotkey
wisprarch history            # View transcriptions
```

## Configuration

Config file: `~/.config/wisprarch/config.toml`

```toml
[whisper]
provider = "groq"
model = "whisper-large-v3-turbo"
language = "en"
api_key = "gsk_..."

[behavior]
auto_paste = true
delete_audio_files = true

[ui.waybar]
idle_text = "󰓃"
recording_text = "󰻃"
```

## Docker

```bash
docker-compose up -d
```

## Waybar Integration

Add to `~/.config/waybar/config`:

```json
"custom/wisprarch": {
    "exec": "curl -s http://127.0.0.1:3737/waybar",
    "return-type": "json",
    "interval": 1,
    "on-click": "curl -X POST http://127.0.0.1:3737/toggle"
}
```

## License

MIT
