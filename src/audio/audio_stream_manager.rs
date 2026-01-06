#![allow(clippy::arc_with_non_send_sync)]

use crate::audio::audio_analyzer::{AudioAnalyzerHandle, NUM_BANDS};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

/// State of the audio recording session
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Stopping,
}

/// Manages the lifecycle of audio streams and recordings
pub struct AudioStreamManager {
    device: cpal::Device,
    config: cpal::StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    active_stream: Arc<Mutex<Option<cpal::Stream>>>,
    state: Arc<Mutex<RecordingState>>,
    audio_level: Arc<Mutex<f32>>,
    /// FFT-based audio analyzer for frequency band visualization.
    analyzer: AudioAnalyzerHandle,
}

impl AudioStreamManager {
    /// Create a new audio stream manager
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        info!("Using audio device: {}", device.name()?);

        let _config = device.default_input_config()?;
        let sample_rate = 16000; // Whisper optimal
        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            device,
            config,
            samples: Arc::new(Mutex::new(Vec::new())),
            active_stream: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(RecordingState::Idle)),
            audio_level: Arc::new(Mutex::new(0.0)),
            analyzer: AudioAnalyzerHandle::new(sample_rate),
        })
    }

    /// Get current audio level (0.0 to 1.0)
    pub fn get_audio_level(&self) -> f32 {
        self.analyzer.get_audio_level()
    }

    pub fn get_audio_level_handle(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.audio_level)
    }

    /// Get current frequency band levels for visualization.
    /// Returns array of NUM_BANDS values, each 0.0 to 1.0.
    pub fn get_frequency_bands(&self) -> [f32; NUM_BANDS] {
        self.analyzer.get_bands()
    }

    /// Get handle to the audio analyzer for sharing between threads.
    pub fn get_analyzer_handle(&self) -> AudioAnalyzerHandle {
        self.analyzer.clone()
    }

    /// Start recording audio, properly managing stream lifecycle
    pub async fn start_recording(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        match *state {
            RecordingState::Recording => {
                return Err(anyhow::anyhow!("Recording already in progress"));
            }
            RecordingState::Stopping => {
                return Err(anyhow::anyhow!("Previous recording still stopping"));
            }
            RecordingState::Idle => {}
        }

        // Stop any existing stream before starting new one
        self.cleanup_stream();

        // Clear samples buffer for new recording
        {
            let mut samples = self.samples.lock().unwrap();
            samples.clear();
            samples.shrink_to_fit(); // Free memory from previous recordings
        }

        debug!("Creating new audio stream");

        // Reset analyzer state for new recording
        self.analyzer.reset();

        let samples_clone = self.samples.clone();
        let analyzer_clone = self.analyzer.clone();
        let err_fn = |err| error!("Audio stream error: {}", err);

        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Store samples for WAV recording
                if let Ok(mut samples) = samples_clone.lock() {
                    samples.extend_from_slice(data);
                }

                // Process through FFT analyzer for frequency bands
                analyzer_clone.process_samples(data);
            },
            err_fn,
            None,
        )?;

        stream.play()?;
        info!("Started audio recording");

        // Store stream for proper cleanup
        *self.active_stream.lock().unwrap() = Some(stream);
        *state = RecordingState::Recording;

        Ok(())
    }

    /// Stop recording and save audio to file
    pub async fn stop_recording(&self, output_path: PathBuf) -> Result<PathBuf> {
        let mut state = self.state.lock().unwrap();

        match *state {
            RecordingState::Idle => {
                return Err(anyhow::anyhow!("No recording in progress"));
            }
            RecordingState::Stopping => {
                return Err(anyhow::anyhow!("Recording already stopping"));
            }
            RecordingState::Recording => {}
        }

        *state = RecordingState::Stopping;
        drop(state); // Release lock before cleanup

        // Stop and cleanup stream
        self.cleanup_stream();

        // Extract samples
        let samples = {
            let samples_guard = self.samples.lock().unwrap();
            samples_guard.clone()
        };

        if samples.is_empty() {
            *self.state.lock().unwrap() = RecordingState::Idle;
            return Err(anyhow::anyhow!("No audio samples recorded"));
        }

        info!("Stopping recording, {} samples captured", samples.len());

        // Write WAV file
        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(&output_path, spec)?;
        for sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;

        // Clear samples and reset state
        {
            let mut samples = self.samples.lock().unwrap();
            samples.clear();
            samples.shrink_to_fit();
        }

        *self.state.lock().unwrap() = RecordingState::Idle;

        info!("Audio saved to: {:?}", output_path);
        Ok(output_path)
    }

    /// Cleanup any active stream
    fn cleanup_stream(&self) {
        let mut active_stream = self.active_stream.lock().unwrap();
        if let Some(stream) = active_stream.take() {
            debug!("Cleaning up audio stream");
            // Stream is automatically stopped when dropped
            drop(stream);
        }
    }
}

impl Drop for AudioStreamManager {
    fn drop(&mut self) {
        debug!("Dropping AudioStreamManager, cleaning up resources");
        self.cleanup_stream();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_ci() -> bool {
        std::env::var("CI").is_ok()
            || std::env::var("GITHUB_ACTIONS").is_ok()
            || std::env::var("GITLAB_CI").is_ok()
            || std::env::var("TRAVIS").is_ok()
    }

    #[tokio::test]
    async fn test_audio_stream_manager_creation() {
        if is_ci() {
            // Skip audio tests in CI - no audio devices available
            return;
        }

        // This test may fail in CI without audio devices
        let _manager = AudioStreamManager::new();
    }
}
