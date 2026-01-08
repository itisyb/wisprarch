# wisprarch Installation Guide

Complete installation instructions for different operating systems and environments.

## Quick Install (Recommended)

wisprarch now ships verified binaries for Linux and macOS. Install or reinstall the service with one command—no Rust toolchain, git clone, or manual builds required:

```bash
curl -fsSL https://install.wisprarch.dev/cli/latest.sh | bash
```

The installer:

- Detects your OS/architecture and selects the matching artifact.
- Verifies SHA-256 (and optional signatures) before extracting.
- Installs the `wisprarch` binary into `/usr/local/bin` (or a custom prefix).
- Drops the systemd user unit plus config scaffolding under `~/.config/wisprarch`.
- Seeds `update_state.json` so the built-in auto-updater can take over.
- Is idempotent—rerun anytime to repair, reinstall, or switch channels.

### Useful flags

```
latest.sh --channel beta            # jump to another release channel
latest.sh --clean                   # remove previous binary/service before reinstalling
latest.sh --dry-run                 # fetch & verify artifacts without touching the system
```

After install:
1. The installer automatically enables/starts the systemd **user** service (unless `--no-start` was set). Use `systemctl --user status wisprarch.service` to confirm.
2. Add a keybind in Hyprland (or your compositor) that calls `curl -X POST http://127.0.0.1:3737/toggle`.
3. Edit `~/.config/wisprarch/config.toml` if you need custom providers, models, or behavior tweaks.

## Manual Installation

> **When should I use this?**  
> Only when you need to hack on wisprarch itself or build for a platform that doesn't have pre-built binaries yet. Everyone else should stick with the `latest.sh` installer above.

### Prerequisites

All systems require:
- **Rust toolchain** (1.70+)
- **Whisper implementation** (see [Whisper Installation Options](#whisper-installation-options))
- **Text injection tool**: `ydotool` (recommended) or `wtype`
- **Clipboard tools**: `wl-clipboard` (Wayland) or `xclip`/`xsel` (X11)
- **Audio dependencies**: ALSA libraries
- **curl** for API communication

### System Dependencies

#### Arch Linux

```bash
sudo pacman -S rust ydotool wtype wl-clipboard alsa-lib curl cmake make gcc
```

#### Ubuntu/Debian

```bash
sudo apt update
sudo apt install cargo libasound2-dev wl-clipboard curl cmake build-essential

# Install ydotool (may need to compile from source)
sudo apt install ydotool || {
    git clone https://github.com/ReimuNotMoe/ydotool.git
    cd ydotool && mkdir build && cd build
    cmake .. && make -j$(nproc)
    sudo make install
}
```

#### Fedora

```bash
sudo dnf install rust cargo ydotool cmake gcc-c++ alsa-lib-devel curl openssl-devel
```

### Text Injection Setup

wisprarch requires a text injection method. See the [Text Injection Setup Guide](./text-injection-setup.md) for detailed configuration.

**Quick setup for ydotool (recommended):**

```bash
# Enable ydotool user service
systemctl --user enable --now ydotool.service

# Add to shell profile
echo 'export YDOTOOL_SOCKET="/run/user/$(id -u)/.ydotool_socket"' >> ~/.bashrc
source ~/.bashrc
```

## Whisper Installation Options

wisprarch supports multiple Whisper implementations:

### Option 1: Optimized whisper.cpp (Recommended)

Use the optimized fork with automatic build:

```bash
git clone https://github.com/matsilva/whisper.git ~/.local/share/wisprarch/whisper
cd ~/.local/share/wisprarch/whisper
./build.sh
```

This downloads and quantizes the large-v3-turbo model automatically.

### Option 2: OpenAI Whisper (Python)

```bash
pip install -U openai-whisper
```

### Option 3: Standard whisper.cpp

```bash
git clone https://github.com/ggerganov/whisper.cpp.git
cd whisper.cpp
make
./models/download-ggml-model.sh base
```

## Building wisprarch

```bash
# Clone the repository
git clone https://github.com/silvabyte/wisprarch.git
cd wisprarch

# Build release version
cargo build --release

# Install binary
sudo cp target/release/wisprarch /usr/local/bin/
sudo chmod +x /usr/local/bin/wisprarch
```

## Configuration

Create the configuration directory and file:

```bash
mkdir -p ~/.config/wisprarch
```

wisprarch will create a default config on first run, or you can create one manually:

### Quick Start (wisprarch API - Recommended)

Zero-config cloud transcription - no API key or local setup required:

```toml
[whisper]
provider = "wisprarch-api"  # Default: hosted service, no setup needed
language = "en"

[wayland]
input_method = "ydotool"  # Recommended (auto-detected first)

[behavior]
auto_paste = true
delete_audio_files = true
audio_feedback = true
```

### Advanced: Local Processing

#### For OpenAI Whisper (CLI)

```toml
[whisper]
provider = "openai-cli"
model = "base"
language = "en"
# command_path is auto-detected if whisper is in PATH

[wayland]
input_method = "ydotool"  # Recommended (auto-detected first)

[behavior]
auto_paste = true
delete_audio_files = true
audio_feedback = true
```

#### For Optimized Whisper.cpp

```toml
[whisper]
provider = "whisper-cpp"
model = "large-v3-turbo"
language = "en"
command_path = "/home/user/.local/share/wisprarch/whisper/build/bin/whisper-cli"
model_path = "/home/user/.local/share/wisprarch/whisper/models/ggml-large-v3-turbo-q5_1.bin"

[wayland]
input_method = "ydotool"  # Recommended (auto-detected first)

[behavior]
auto_paste = true
delete_audio_files = true
audio_feedback = true
```

## Systemd Service Setup

Create a user service for automatic startup:

```bash
mkdir -p ~/.config/systemd/user
```

Create `~/.config/systemd/user/wisprarch.service`:

```ini
[Unit]
Description=wisprarch Voice Transcription Service
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/local/bin/wisprarch
Restart=always
RestartSec=5
Environment="RUST_LOG=info"
MemoryMax=6G
CPUQuota=80%

[Install]
WantedBy=default.target
```

Enable and start the service:

```bash
systemctl --user daemon-reload
systemctl --user enable --now wisprarch.service
```

> **Audio groups:** User services cannot add supplemental groups the account does not already have. Most setups that use PipeWire/ALSA through the desktop stack work without any extra privileges. If you need direct ALSA device access, add yourself to the `audio` group (followed by a re-login) or, for `latest.sh --system`, add `SupplementaryGroups=audio` via a systemd drop-in.

## Hyprland Integration

Add to your Hyprland config (`~/.config/hypr/hyprland.conf`):

```
bindd = SUPER, R, wisprarch, exec, curl -X POST http://127.0.0.1:3737/toggle
```

For Omarchy users:
```
bindd = SUPER, R, wisprarch, exec, $terminal -e curl -X POST http://127.0.0.1:3737/toggle
```

## GNOME + Wayland Setup

GNOME requires special setup due to security restrictions:

### 1. Install ydotool and setup daemon

```bash
sudo pacman -S ydotool  # or appropriate package manager

# Create user service
mkdir -p ~/.config/systemd/user
```

Create `~/.config/systemd/user/ydotoold.service`:

```ini
[Unit]
Description=ydotoold user daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart=/usr/bin/ydotoold -P 660

[Install]
WantedBy=default.target
```

```bash
# Add environment variable
echo 'export YDOTOOL_SOCKET="/run/user/$(id -u)/.ydotool_socket"' >> ~/.bashrc
source ~/.bashrc

# Enable services
systemctl --user daemon-reload
systemctl --user enable --now ydotoold.service
systemctl --user enable --now wisprarch.service
```

### 2. Configure wisprarch for GNOME

```toml
[wayland]
input_method = "ydotool"  # Recommended (auto-detected first)
```

### 3. Create GNOME Keyboard Shortcut

1. Open GNOME Settings
2. Go to Keyboard → Keyboard Shortcuts → View and Customize Shortcuts
3. Go to Custom Shortcuts
4. Add new shortcut with command: `curl -X POST http://127.0.0.1:3737/toggle`
5. Set your preferred key combination (e.g., Super+R)

## Testing Installation

1. **Test service**: `systemctl --user status wisprarch.service`
2. **Test API**: `curl -X POST http://127.0.0.1:3737/toggle`
3. **Test provider**: `wisprarch provider test` (validates transcription setup)
4. **Test recording**: Press your configured keybind
5. **Check logs**: `make logs` or `journalctl --user -u wisprarch.service -f`

## Troubleshooting

### Service fails to start
- Check logs: `make logs` or `journalctl --user -u wisprarch.service -e`
- Check status: `make status`
- Verify binary path: `which wisprarch`
- Test config: `wisprarch --verbose`

### Recording doesn't work
- Check microphone permissions
- Verify audio device: `arecord -l`
- Ensure the desired input device is set as the system default (wisprarch uses whatever CPAL reports as default)

### Text injection fails
- Verify ydotool service: `systemctl --user status ydotool.service`
- Check socket: `ls -la /run/user/$(id -u)/.ydotool_socket`
- See [Text Injection Setup](./text-injection-setup.md)

### Memory issues
- Large Whisper models need 3-5GB RAM
- Adjust `MemoryMax` in the service file (or remove it entirely)
- Use smaller models if needed

### GNOME-specific issues
- Ensure ydotoold is running as user service (not system)
- Verify YDOTOOL_SOCKET environment variable
- wtype will NOT work on GNOME - use ydotool only

## Updating

wisprarch now includes two parallel update paths:

1. **Background auto-updater**: runs inside the daemon, checks `https://install.wisprarch.dev/cli/version` every few hours, downloads new binaries into `~/.local/share/wisprarch/updates`, swaps them atomically, and restarts the service (unless `WISPRARCH_DISABLE_AUTO_RESTART=1` is set). Auto-updates respect `~/.config/wisprarch/update_state.json` and can be disabled.

2. **Manual CLI control** via the built-in subcommand:

```bash
# Show current vs remote version without installing
wisprarch update --check

# Force an immediate install (even if versions appear equal)
wisprarch update --force

# Switch channels for subsequent checks
wisprarch update --channel beta

# Toggle background updates
wisprarch update --disable
wisprarch update --enable
```

Because `latest.sh` is idempotent, you can also rerun it at any time to jump to a specific channel or repair a broken install:

```bash
curl -fsSL https://install.wisprarch.dev/cli/latest.sh | bash -s -- --channel beta --clean
```

## Uninstalling

Remove wisprarch with the dedicated uninstall script:

```bash
curl -fsSL https://install.wisprarch.dev/cli/uninstall.sh | bash
```

### Uninstall options

```bash
# Preview what will be removed (no changes made)
curl -fsSL https://install.wisprarch.dev/cli/uninstall.sh | bash -s -- --dry-run

# Skip confirmation prompt
curl -fsSL https://install.wisprarch.dev/cli/uninstall.sh | bash -s -- --yes

# Keep your config and transcription history
curl -fsSL https://install.wisprarch.dev/cli/uninstall.sh | bash -s -- --keep-config --keep-database

# Also remove temp audio files from /tmp
curl -fsSL https://install.wisprarch.dev/cli/uninstall.sh | bash -s -- --remove-temp
```

### What gets removed

By default, the uninstaller removes:
- `/usr/local/bin/wisprarch` (CLI binary)
- `/usr/local/bin/wisprarch-*.bak` (backup binaries from auto-updates)
- `~/.config/systemd/user/wisprarch.service` (systemd unit)
- `~/.config/wisprarch/` (config and update state)
- `~/.local/share/wisprarch/` (database and update cache)

Use `--keep-config`, `--keep-database`, or `--keep-updates` to preserve specific artifacts.