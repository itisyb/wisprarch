#!/bin/bash
set -e

echo "󰓃 WisprArch Installer"
echo "========================"
echo ""

INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="$HOME/.config/wisprarch"
MODELS_DIR="$HOME/.local/share/wisprarch/models"

if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

# Check for text injection tools (required for auto-paste)
echo "Checking text injection tools..."
HAS_WTYPE=false
HAS_YDOTOOL=false

if command -v wtype &> /dev/null; then
    HAS_WTYPE=true
    echo "   ✓ wtype found"
fi

if command -v ydotool &> /dev/null; then
    HAS_YDOTOOL=true
    echo "   ✓ ydotool found"
fi

if [ "$HAS_WTYPE" = false ] && [ "$HAS_YDOTOOL" = false ]; then
    echo ""
    echo "   ⚠ WARNING: No text injection tool found!"
    echo "   Auto-paste won't work without wtype or ydotool."
    echo ""
    echo "   Install one of these (Arch Linux):"
    echo "     sudo pacman -S wtype      # Recommended for Hyprland/Wayland"
    echo "     sudo pacman -S ydotool    # Universal (requires ydotoold service)"
    echo ""
    read -p "   Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "[1/4] Building WisprArch..."
cargo build --release

echo "[2/4] Installing binary..."
sudo cp target/release/wisprarch "$INSTALL_DIR/"
sudo chmod +x "$INSTALL_DIR/wisprarch"

echo "[3/4] Creating config directories..."
mkdir -p "$CONFIG_DIR"
mkdir -p "$MODELS_DIR"

if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    cat > "$CONFIG_DIR/config.toml" << 'EOF'
[whisper]
provider = "groq"
model = "whisper-large-v3-turbo"
language = "en"

[behavior]
auto_paste = true
delete_audio_files = true
audio_feedback = true

[ui.waybar]
idle_text = "󰓃"
recording_text = "󰻃"
EOF
    echo "   Created default config at $CONFIG_DIR/config.toml"
fi

echo "[4/4] Installing systemd service..."
mkdir -p "$HOME/.config/systemd/user"
cp wisprarch.service "$HOME/.config/systemd/user/"
systemctl --user daemon-reload

echo ""
echo "Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Add your Groq API key: wisprarch provider configure"
echo "  2. Start the service: systemctl --user start wisprarch"
echo "  3. Enable on boot: systemctl --user enable wisprarch"
echo "  4. Set up keybind: wisprarch keybind"
echo ""
echo "Commands:"
echo "  wisprarch tui           - Launch model manager TUI"
echo "  wisprarch models list   - List available models"
echo "  wisprarch provider show - Show current provider"
echo ""
