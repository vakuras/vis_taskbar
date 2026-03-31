use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

/// Hamming-windowed FFT spectrum analyzer.
/// Accepts `fft_size` real samples, produces `fft_size/2` magnitude bins in 0..=1 range.
pub struct Spectrum {
    fft_size: usize,
    bin_size: usize,
    window: Vec<f32>,
    planner_buf: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    magnitudes: Vec<f32>,
    fft: std::sync::Arc<dyn rustfft::Fft<f32>>,
}

impl Spectrum {
    pub fn new(fft_size: usize) -> Self {
        let bin_size = fft_size / 2;

        // Hamming window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| 0.54 - 0.46 * (2.0 * PI * i as f32 / (fft_size - 1) as f32).cos())
            .collect();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];

        Self {
            fft_size,
            bin_size,
            window,
            planner_buf: vec![Complex::default(); fft_size],
            scratch,
            magnitudes: vec![0.0; bin_size],
            fft,
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

        // Magnitude (normalized)
        let norm = 1.0 / self.fft_size as f32;
        for i in 0..self.bin_size {
            let c = self.planner_buf[i];
            let mag = (c.re * c.re + c.im * c.im).sqrt() * norm;
            // Scale up for visibility (the C++ code mapped via * 256)
            self.magnitudes[i] = (mag * 4.0).min(1.0);
        }

        &self.magnitudes
    }
}
