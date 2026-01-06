//! Real-time audio frequency analyzer for visualization.
//!
//! Uses FFT to extract frequency bands from audio samples for
//! creating a multi-bar audio visualizer like CAVA.

use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};

/// Number of frequency bands for visualization.
pub const NUM_BANDS: usize = 8;

/// FFT window size (must be power of 2).
/// 512 samples at 16kHz = 32ms window, good for voice.
const FFT_SIZE: usize = 512;

/// Frequency ranges for each band (Hz).
/// Optimized for voice frequencies (fundamental 85-255Hz, harmonics up to 8kHz).
const BAND_RANGES: [(f32, f32); NUM_BANDS] = [
    (60.0, 150.0),    // Sub-bass / low voice fundamentals
    (150.0, 300.0),   // Bass / voice fundamentals
    (300.0, 600.0),   // Low-mid / voice body
    (600.0, 1200.0),  // Mid / voice clarity
    (1200.0, 2400.0), // Upper-mid / voice presence
    (2400.0, 4000.0), // High-mid / sibilance
    (4000.0, 6000.0), // High / air
    (6000.0, 8000.0), // Ultra-high / brilliance
];

/// Real-time audio analyzer that performs FFT and extracts frequency bands.
pub struct AudioAnalyzer {
    /// Ring buffer for recent audio samples.
    sample_buffer: Vec<f32>,
    /// Current write position in ring buffer.
    write_pos: usize,
    /// FFT planner (reusable).
    fft_planner: FftPlanner<f32>,
    /// Scratch buffer for FFT input.
    fft_input: Vec<Complex<f32>>,
    /// Scratch buffer for FFT output.
    fft_output: Vec<Complex<f32>>,
    /// Current frequency band levels (0.0 to 1.0).
    bands: [f32; NUM_BANDS],
    /// Smoothed band levels for display.
    smoothed_bands: [f32; NUM_BANDS],
    /// Sample rate (Hz).
    sample_rate: f32,
    /// Hann window coefficients for reducing spectral leakage.
    window: Vec<f32>,
    /// Overall audio level (RMS).
    audio_level: f32,
}

impl AudioAnalyzer {
    /// Create a new audio analyzer.
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz (typically 16000 for this app).
    pub fn new(sample_rate: u32) -> Self {
        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|i| {
                let t = i as f32 / (FFT_SIZE - 1) as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
            })
            .collect();

        Self {
            sample_buffer: vec![0.0; FFT_SIZE],
            write_pos: 0,
            fft_planner: FftPlanner::new(),
            fft_input: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            fft_output: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            bands: [0.0; NUM_BANDS],
            smoothed_bands: [0.0; NUM_BANDS],
            sample_rate: sample_rate as f32,
            window,
            audio_level: 0.0,
        }
    }

    /// Process new audio samples and update frequency bands.
    ///
    /// Call this from the audio callback with each chunk of samples.
    pub fn process_samples(&mut self, samples: &[f32]) {
        // Calculate RMS for overall level
        if !samples.is_empty() {
            let rms: f32 =
                (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
            let level = (rms * 3.0).min(1.0);
            self.audio_level = self.audio_level * 0.7 + level * 0.3;
        }

        // Add samples to ring buffer
        for &sample in samples {
            self.sample_buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % FFT_SIZE;
        }

        // Perform FFT analysis
        self.analyze_spectrum();
    }

    /// Perform FFT and extract frequency bands.
    fn analyze_spectrum(&mut self) {
        // Copy samples to FFT input with Hann window applied
        // Read from ring buffer in correct order
        for i in 0..FFT_SIZE {
            let idx = (self.write_pos + i) % FFT_SIZE;
            self.fft_input[i] = Complex::new(self.sample_buffer[idx] * self.window[i], 0.0);
        }

        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(FFT_SIZE);
        self.fft_output.copy_from_slice(&self.fft_input);
        fft.process(&mut self.fft_output);

        // Calculate magnitude for each frequency bin
        let bin_width = self.sample_rate / FFT_SIZE as f32;
        let nyquist_bin = FFT_SIZE / 2;

        // Extract energy for each frequency band
        for (band_idx, &(low_freq, high_freq)) in BAND_RANGES.iter().enumerate() {
            let low_bin = ((low_freq / bin_width) as usize).min(nyquist_bin);
            let high_bin = ((high_freq / bin_width) as usize).min(nyquist_bin);

            if low_bin >= high_bin {
                self.bands[band_idx] = 0.0;
                continue;
            }

            // Calculate average magnitude in this band
            let mut sum = 0.0;
            let mut count = 0;
            for bin in low_bin..high_bin {
                let magnitude = self.fft_output[bin].norm();
                sum += magnitude;
                count += 1;
            }

            let avg_magnitude = if count > 0 { sum / count as f32 } else { 0.0 };

            // Normalize and apply non-linear scaling for better visual response
            // Voice typically has lower energy in high frequencies
            let band_boost = match band_idx {
                0 => 1.0, // Sub-bass
                1 => 1.2, // Bass
                2 => 1.5, // Low-mid (primary voice)
                3 => 2.0, // Mid (voice clarity)
                4 => 2.5, // Upper-mid
                5 => 3.0, // High-mid
                6 => 4.0, // High
                7 => 5.0, // Ultra-high
                _ => 1.0,
            };

            // Scale magnitude to 0.0-1.0 range
            // Balanced sensitivity - responds to voice but not always maxed
            let scaled = avg_magnitude * band_boost * 20.0;
            // Square root for compressed dynamic range
            let normalized = (scaled.sqrt()).clamp(0.0, 1.0);
            self.bands[band_idx] = normalized;
        }

        // Apply smoothing for visual appeal (attack fast, decay slow)
        for i in 0..NUM_BANDS {
            if self.bands[i] > self.smoothed_bands[i] {
                // Fast attack
                self.smoothed_bands[i] = self.smoothed_bands[i] * 0.2 + self.bands[i] * 0.8;
            } else {
                // Fast decay - bars drop quickly when you stop speaking
                self.smoothed_bands[i] = self.smoothed_bands[i] * 0.5 + self.bands[i] * 0.5;
            }
        }
    }

    /// Get the current smoothed frequency band levels.
    ///
    /// Returns array of 8 values, each 0.0 to 1.0.
    pub fn get_bands(&self) -> [f32; NUM_BANDS] {
        self.smoothed_bands
    }

    /// Get the overall audio level (RMS-based).
    pub fn get_audio_level(&self) -> f32 {
        self.audio_level
    }

    /// Reset all levels to zero.
    pub fn reset(&mut self) {
        self.sample_buffer.fill(0.0);
        self.write_pos = 0;
        self.bands = [0.0; NUM_BANDS];
        self.smoothed_bands = [0.0; NUM_BANDS];
        self.audio_level = 0.0;
    }
}

/// Thread-safe handle for sharing analyzer between threads.
#[derive(Clone)]
pub struct AudioAnalyzerHandle {
    inner: Arc<Mutex<AudioAnalyzer>>,
}

impl AudioAnalyzerHandle {
    /// Create a new analyzer handle.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AudioAnalyzer::new(sample_rate))),
        }
    }

    /// Process samples (call from audio callback).
    pub fn process_samples(&self, samples: &[f32]) {
        if let Ok(mut analyzer) = self.inner.lock() {
            analyzer.process_samples(samples);
        }
    }

    /// Get current frequency bands.
    pub fn get_bands(&self) -> [f32; NUM_BANDS] {
        self.inner
            .lock()
            .map(|a| a.get_bands())
            .unwrap_or([0.0; NUM_BANDS])
    }

    /// Get overall audio level.
    pub fn get_audio_level(&self) -> f32 {
        self.inner
            .lock()
            .map(|a| a.get_audio_level())
            .unwrap_or(0.0)
    }

    /// Reset analyzer state.
    pub fn reset(&self) {
        if let Ok(mut analyzer) = self.inner.lock() {
            analyzer.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = AudioAnalyzer::new(16000);
        assert_eq!(analyzer.get_bands(), [0.0; NUM_BANDS]);
        assert_eq!(analyzer.get_audio_level(), 0.0);
    }

    #[test]
    fn test_process_silence() {
        let mut analyzer = AudioAnalyzer::new(16000);
        let silence = vec![0.0; 512];
        analyzer.process_samples(&silence);

        let bands = analyzer.get_bands();
        for band in bands {
            assert!(band < 0.01, "Silent input should produce near-zero bands");
        }
    }

    #[test]
    fn test_process_tone() {
        let mut analyzer = AudioAnalyzer::new(16000);

        // Generate a 500Hz sine wave (should appear in band 2: 300-600Hz)
        let samples: Vec<f32> = (0..512)
            .map(|i| {
                let t = i as f32 / 16000.0;
                (2.0 * std::f32::consts::PI * 500.0 * t).sin() * 0.5
            })
            .collect();

        // Process multiple times for smoothing to catch up
        for _ in 0..10 {
            analyzer.process_samples(&samples);
        }

        let bands = analyzer.get_bands();
        // Band 2 (300-600Hz) should have significant energy
        assert!(
            bands[2] > 0.1,
            "500Hz tone should produce energy in band 2, got {}",
            bands[2]
        );
    }

    #[test]
    fn test_analyzer_handle_thread_safety() {
        let handle = AudioAnalyzerHandle::new(16000);
        let handle_clone = handle.clone();

        // Simulate audio callback thread
        std::thread::spawn(move || {
            let samples = vec![0.1; 256];
            handle_clone.process_samples(&samples);
        })
        .join()
        .unwrap();

        // Main thread can still read
        let _bands = handle.get_bands();
        let _level = handle.get_audio_level();
    }

    #[test]
    fn test_reset() {
        let mut analyzer = AudioAnalyzer::new(16000);

        // Process some noise
        let samples: Vec<f32> = (0..512).map(|i| (i as f32 * 0.01).sin()).collect();
        analyzer.process_samples(&samples);

        // Reset
        analyzer.reset();

        assert_eq!(analyzer.get_bands(), [0.0; NUM_BANDS]);
        assert_eq!(analyzer.get_audio_level(), 0.0);
    }
}
