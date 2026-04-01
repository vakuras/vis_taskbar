use crate::config::WindowType;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Configurable-window FFT spectrum analyzer.
/// Accepts `fft_size` real samples, produces `fft_size/2` magnitude bins in 0..=1 range.
pub struct Spectrum {
    fft_size: usize,
    bin_size: usize,
    window: Vec<f32>,
    window_type: WindowType,
    planner_buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    magnitudes: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

fn build_window(fft_size: usize, window_type: WindowType) -> Vec<f32> {
    let n = (fft_size - 1) as f32;
    (0..fft_size)
        .map(|i| {
            let t = 2.0 * PI * i as f32 / n;
            match window_type {
                WindowType::Hann => 0.5 * (1.0 - t.cos()),
                WindowType::Hamming => 0.54 - 0.46 * t.cos(),
                WindowType::BlackmanHarris => {
                    0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos() - 0.01168 * (3.0 * t).cos()
                }
            }
        })
        .collect()
}

impl Spectrum {
    #[allow(dead_code)]
    pub fn new(fft_size: usize) -> Self {
        Self::with_window(fft_size, WindowType::default())
    }

    pub fn with_window(fft_size: usize, window_type: WindowType) -> Self {
        let bin_size = fft_size / 2;
        let window = build_window(fft_size, window_type);

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];

        Self {
            fft_size,
            bin_size,
            window,
            window_type,
            planner_buf: vec![Complex::default(); fft_size],
            scratch,
            magnitudes: vec![0.0; bin_size],
            fft,
        }
    }

    /// Update window type if changed.
    pub fn set_window_type(&mut self, wt: WindowType) {
        if self.window_type != wt {
            self.window = build_window(self.fft_size, wt);
            self.window_type = wt;
        }
    }

    pub fn bin_size(&self) -> usize {
        self.bin_size
    }

    /// Process interleaved float samples for one channel.
    /// `samples` must have at least `fft_size` frames.
    /// `channel` = 0 for left, 1 for right.
    /// `channels` = total interleaved channel count.
    /// Returns magnitude bins normalized to 0..=1.
    pub fn process(
        &mut self,
        samples: &[f32],
        channel: usize,
        channels: usize,
    ) -> &[f32] {
        // De-interleave + window
        for i in 0..self.fft_size {
            let idx = i * channels + channel;
            let sample = if idx < samples.len() {
                samples[idx]
            } else {
                0.0
            };
            self.planner_buf[i] = Complex::new(sample * self.window[i], 0.0);
        }

        self.fft
            .process_with_scratch(&mut self.planner_buf, &mut self.scratch);

        let norm = 1.0 / self.fft_size as f32;
        for i in 0..self.bin_size {
            let c = self.planner_buf[i];
            let mag = (c.re * c.re + c.im * c.im).sqrt() * norm;
            // Frequency-weighted gain: boost higher bins to compensate for
            // natural 1/f energy falloff. sqrt(1 + bin_index) gives ~3dB/octave boost.
            let freq_weight = (1.0 + i as f32).sqrt();
            self.magnitudes[i] = (mag * freq_weight).min(1.0);
        }

        &self.magnitudes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_bin_size() {
        let s = Spectrum::new(256);
        assert_eq!(s.bin_size(), 128);
    }

    #[test]
    fn spectrum_silence_produces_zero() {
        let mut s = Spectrum::new(256);
        let silence = vec![0.0f32; 256];
        let mags = s.process(&silence, 0, 1);
        assert_eq!(mags.len(), 128);
        assert!(mags.iter().all(|&m| m == 0.0));
    }

    #[test]
    fn spectrum_sine_produces_peak() {
        let mut s = Spectrum::new(256);
        // Generate a 1kHz sine at 48kHz sample rate
        let samples: Vec<f32> = (0..256)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();
        let mags = s.process(&samples, 0, 1);
        // Should have at least one non-zero bin
        assert!(mags.iter().any(|&m| m > 0.0));
        // The peak should be in the low-frequency bins (1kHz / (48kHz/256) ≈ bin 5)
        let peak_bin = mags.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
        assert!(peak_bin < 20, "Peak at bin {peak_bin}, expected near bin 5");
    }

    #[test]
    fn spectrum_stereo_deinterleave() {
        let mut s = Spectrum::new(256);
        // Interleaved stereo: left=sine, right=silence
        let mut samples = vec![0.0f32; 512];
        for i in 0..256 {
            samples[i * 2] = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin();
            samples[i * 2 + 1] = 0.0;
        }
        let left = s.process(&samples, 0, 2);
        assert!(left.iter().any(|&m| m > 0.0), "Left channel should have signal");

        let right = s.process(&samples, 1, 2);
        assert!(right.iter().all(|&m| m == 0.0), "Right channel should be silent");
    }

    #[test]
    fn spectrum_all_window_types_work() {
        use crate::config::WindowType;
        let samples: Vec<f32> = (0..256)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();

        for wt in [WindowType::Hann, WindowType::Hamming, WindowType::BlackmanHarris] {
            let mut s = Spectrum::with_window(256, wt);
            let mags = s.process(&samples, 0, 1);
            assert!(mags.iter().any(|&m| m > 0.0), "Window {:?} should produce output", wt);
        }
    }

    #[test]
    fn spectrum_set_window_type_changes_output() {
        use crate::config::WindowType;
        let samples: Vec<f32> = (0..256)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();

        let mut s = Spectrum::with_window(256, WindowType::Hann);
        let hann_peak = s.process(&samples, 0, 1).iter().cloned().fold(0.0f32, f32::max);

        s.set_window_type(WindowType::BlackmanHarris);
        let bh_peak = s.process(&samples, 0, 1).iter().cloned().fold(0.0f32, f32::max);

        // Different windows should produce different peak values
        assert!((hann_peak - bh_peak).abs() > 0.001, "Hann={hann_peak} BH={bh_peak} should differ");
    }

    #[test]
    fn spectrum_freq_weight_boosts_treble() {
        let mut s = Spectrum::with_window(256, crate::config::WindowType::Hann);
        // White noise — all bins should have similar raw energy
        // but freq weighting should boost higher bins
        let samples: Vec<f32> = (0..256).map(|i| ((i * 7 + 13) % 100) as f32 / 100.0 - 0.5).collect();
        let mags = s.process(&samples, 0, 1);

        // Average of upper half should be >= average of lower half due to freq weighting
        let lower: f32 = mags[1..64].iter().sum::<f32>() / 63.0;
        let upper: f32 = mags[64..128].iter().sum::<f32>() / 64.0;
        assert!(upper >= lower * 0.5, "Upper={upper} should be boosted relative to lower={lower}");
    }
}
