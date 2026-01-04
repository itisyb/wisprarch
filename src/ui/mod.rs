use anyhow::Result;
use std::process::Command;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct Indicator {
    audio_feedback_enabled: bool,
}

impl Default for Indicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Indicator {
    pub fn new() -> Self {
        Self {
            audio_feedback_enabled: true,
        }
    }

    pub fn with_audio_feedback(mut self, enabled: bool) -> Self {
        self.audio_feedback_enabled = enabled;
        self
    }

    pub async fn show_recording(&self) -> Result<()> {
        info!("Recording started");
        self.play_sound("start").await;
        Ok(())
    }

    pub async fn show_processing(&self) -> Result<()> {
        info!("Processing started");
        self.play_sound("stop").await;
        Ok(())
    }

    pub async fn show_complete(&self, text: &str) -> Result<()> {
        info!("Transcription complete: {} chars", text.len());
        self.play_sound("complete").await;
        Ok(())
    }

    pub async fn show_error(&self, error: &str) -> Result<()> {
        warn!("Error: {}", error);
        Ok(())
    }

    async fn play_sound(&self, sound_type: &str) {
        if !self.audio_feedback_enabled {
            return;
        }

        debug!("Playing {} sound", sound_type);

        // Use a simple approach with system commands
        let sound_type = sound_type.to_string();
        tokio::spawn(async move {
            if let Err(e) = Self::play_simple_sound(&sound_type).await {
                debug!("Failed to play sound: {}", e);
            }
        });
    }

    async fn play_simple_sound(sound_type: &str) -> Result<()> {
        let (freq, duration_ms) = match sound_type {
            "start" => (800, 150),     // High pitch, short beep
            "stop" => (400, 200),      // Low pitch, longer beep
            "complete" => (1000, 100), // Very high pitch, very short beep
            _ => (500, 150),
        };

        // Try generating custom beep tones first (more distinctive)
        if let Ok(output) = Self::generate_beep_tone(freq, duration_ms).await {
            if output.status.success() || output.status.code() == Some(124) {
                debug!(
                    "Played {} with generated tone ({}Hz, {}ms)",
                    sound_type, freq, duration_ms
                );
                return Ok(());
            }
        }

        // Fallback to system sounds if tone generation fails
        let sound_files = vec![
            "/usr/share/sounds/alsa/Front_Left.wav",
            "/usr/share/sounds/freedesktop/stereo/bell.oga",
            "/usr/share/sounds/Oxygen-Sys-Log-In.ogg",
        ];

        for sound_file in sound_files {
            if std::path::Path::new(sound_file).exists() {
                if let Ok(output) = Command::new("aplay").arg(sound_file).output() {
                    if output.status.success() {
                        debug!("Played {} with aplay: {}", sound_type, sound_file);
                        return Ok(());
                    }
                }
            }
        }

        debug!("No working sound method found for {}", sound_type);
        Ok(())
    }

    async fn generate_beep_tone(freq: u32, duration_ms: u32) -> Result<std::process::Output> {
        // Try different methods to generate custom beep tones

        // Method 1: Use speaker-test (if available)
        let duration_secs = format!("{:.1}", duration_ms as f64 / 1000.0);
        if let Ok(output) = Command::new("timeout")
            .args([
                &duration_secs,
                "speaker-test",
                "-t",
                "sine",
                "-f",
                &freq.to_string(),
                "-c",
                "1",
            ])
            .output()
        {
            if output.status.success() || output.status.code() == Some(124) {
                // 124 = timeout success
                return Ok(output);
            }
        }

        // Method 2: Use beep command (if available)
        if let Ok(output) = Command::new("beep")
            .args(["-f", &freq.to_string(), "-l", &duration_ms.to_string()])
            .output()
        {
            return Ok(output);
        }

        // Method 3: Generate tone with paplay + Python
        let python_cmd = format!(
            "python3 -c \"
import math, sys
samples = int(44100 * {duration_ms} / 1000)
freq = {freq}
for i in range(samples):
    t = i / 44100.0
    sample = math.sin(2.0 * math.pi * freq * t) * 0.3
    sample_i16 = int(sample * 16384)
    sys.stdout.buffer.write(sample_i16.to_bytes(2, 'little', signed=True))
\" | paplay --raw --format=s16le --rate=44100 --channels=1"
        );

        if let Ok(output) = Command::new("bash").args(["-c", &python_cmd]).output() {
            return Ok(output);
        }

        Err(anyhow::anyhow!("No tone generation method available"))
    }
}
