use std::f32::consts::PI;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::debug;

const SAMPLE_RATE: u32 = 44100;

pub fn play_start_sound(custom_path: Option<&str>) {
    if let Some(path) = custom_path {
        if play_file(path) {
            return;
        }
    }
    let samples = generate_start_sound();
    play_samples(&samples);
}

pub fn play_complete_sound(custom_path: Option<&str>) {
    if let Some(path) = custom_path {
        if play_file(path) {
            return;
        }
    }
    let samples = generate_complete_sound();
    play_samples(&samples);
}

fn play_file(path: &str) -> bool {
    let path = Path::new(path);
    if !path.exists() {
        debug!("Custom sound file not found: {:?}", path);
        return false;
    }

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let result = match ext {
        "wav" => Command::new("aplay").arg("-q").arg(path).status(),
        "ogg" | "oga" => Command::new("paplay").arg(path).status(),
        "mp3" => Command::new("mpv")
            .args(["--no-video", "--really-quiet"])
            .arg(path)
            .status(),
        _ => Command::new("paplay").arg(path).status(),
    };

    match result {
        Ok(status) if status.success() => {
            debug!("Played custom sound: {:?}", path);
            true
        }
        _ => {
            debug!("Failed to play custom sound: {:?}", path);
            false
        }
    }
}

fn generate_start_sound() -> Vec<i16> {
    let duration_ms = 120;
    let num_samples = (SAMPLE_RATE as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let progress = i as f32 / num_samples as f32;

        let freq = 600.0 + 400.0 * progress;

        let envelope = (progress * PI).sin();

        let fundamental = (2.0 * PI * freq * t).sin();
        let harmonic2 = 0.3 * (2.0 * PI * freq * 2.0 * t).sin();
        let harmonic3 = 0.15 * (2.0 * PI * freq * 3.0 * t).sin();

        let sample = (fundamental + harmonic2 + harmonic3) * envelope * 0.25;
        samples.push((sample * 32767.0) as i16);
    }

    samples
}

fn generate_complete_sound() -> Vec<i16> {
    let duration_ms = 180;
    let num_samples = (SAMPLE_RATE as usize * duration_ms) / 1000;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let progress = i as f32 / num_samples as f32;

        let freq1 = 880.0;
        let freq2 = 1320.0;

        let attack = 0.05;
        let envelope = if progress < attack {
            progress / attack
        } else {
            let decay_progress = (progress - attack) / (1.0 - attack);
            (-decay_progress * 4.0).exp()
        };

        let tone1 = (2.0 * PI * freq1 * t).sin();
        let tone2 = 0.6 * (2.0 * PI * freq2 * t).sin();
        let shimmer = 0.1 * (2.0 * PI * 2200.0 * t).sin() * (-progress * 8.0).exp();

        let sample = (tone1 + tone2 + shimmer) * envelope * 0.2;
        samples.push((sample * 32767.0) as i16);
    }

    samples
}

fn play_samples(samples: &[i16]) {
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

    if let Ok(mut child) = Command::new("paplay")
        .args(["--raw", "--format=s16le", "--rate=44100", "--channels=1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(&bytes);
        }
        let _ = child.wait();
        debug!("Played sound via paplay");
        return;
    }

    if let Ok(mut child) = Command::new("aplay")
        .args(["-f", "S16_LE", "-r", "44100", "-c", "1", "-q", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(&bytes);
        }
        let _ = child.wait();
        debug!("Played sound via aplay");
        return;
    }

    debug!("No audio player available (tried paplay, aplay)");
}
