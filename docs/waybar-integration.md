# Waybar Integration

Add a modern waveform visualizer to your Waybar that shows real-time audio levels while recording.

## Setup

### 1. Add WisprArch Module to Waybar Config

Edit `~/.config/waybar/config.jsonc`:

```jsonc
{
  "modules-right": ["custom/wisprarch", "pulseaudio", "clock"],
  
  "custom/wisprarch": {
    "exec": "curl -s 'http://127.0.0.1:3737/status?style=waybar'",
    "interval": 1,
    "return-type": "json", 
    "on-click": "curl -X POST http://127.0.0.1:3737/toggle",
    "tooltip": true
  }
}
```

**For smoother waveform animation**, reduce the interval:

```jsonc
"custom/wisprarch": {
  "exec": "while true; do curl -s 'http://127.0.0.1:3737/status?style=waybar'; sleep 0.1; done",
  "exec-on-event": false,
  "return-type": "json",
  "on-click": "curl -X POST http://127.0.0.1:3737/toggle"
}
```

### 2. Add Modern CSS Styling

Edit `~/.config/waybar/style.css`:

```css
/* WisprArch Module */
#custom-wisprarch {
  padding: 0 12px;
  margin: 4px 2px;
  border-radius: 20px;
  font-family: "JetBrainsMono Nerd Font", monospace;
  font-size: 13px;
  transition: all 0.3s ease;
}

/* Idle State - Subtle */
#custom-wisprarch.wisprarch-idle {
  background: rgba(255, 255, 255, 0.05);
  color: #6c7086;
}

#custom-wisprarch.wisprarch-idle:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #cdd6f4;
}

/* Recording State - Vibrant waveform */
#custom-wisprarch.wisprarch-recording {
  background: linear-gradient(135deg, #f38ba8 0%, #fab387 100%);
  color: #1e1e2e;
  font-weight: bold;
  animation: recording-pulse 1.5s ease-in-out infinite;
}

@keyframes recording-pulse {
  0%, 100% { 
    box-shadow: 0 0 0 0 rgba(243, 139, 168, 0.4);
  }
  50% { 
    box-shadow: 0 0 0 8px rgba(243, 139, 168, 0);
  }
}

/* Processing State */
#custom-wisprarch.wisprarch-processing {
  background: linear-gradient(135deg, #89b4fa 0%, #b4befe 100%);
  color: #1e1e2e;
  animation: processing-spin 1s linear infinite;
}

@keyframes processing-spin {
  from { filter: hue-rotate(0deg); }
  to { filter: hue-rotate(360deg); }
}

/* Error State */
#custom-wisprarch.wisprarch-error {
  background: #f38ba8;
  color: #1e1e2e;
}
```

### 3. Restart Waybar

```bash
pkill waybar && waybar &
```

## States

| State | Display | Description |
|-------|---------|-------------|
| **Idle** | `󰍬` | Ready to record |
| **Recording** | `󰍬 ▂▅▃▆▄` | Live waveform visualization |
| **Processing** | `󰦖` | Transcribing audio |
| **Error** | `` | Something went wrong |

## Customization

Edit `~/.config/wisprarch/config.toml`:

```toml
[ui.waybar]
idle_text = "󰍬"
recording_text = "●"
idle_tooltip = "Super+R to record"
recording_tooltip = "Recording..."
```

## Alternative: Minimal Style

```css
#custom-wisprarch {
  padding: 0 8px;
}

#custom-wisprarch.wisprarch-recording {
  color: #f38ba8;
}
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Module not showing | Add `"custom/wisprarch"` to a modules list |
| Shows "N/A" | Check service: `systemctl --user status wisprarch` |
| Click doesn't work | Test: `curl -X POST http://127.0.0.1:3737/toggle` |
| Waveform not updating | Use the continuous exec version above |
