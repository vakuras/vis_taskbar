use crate::config::Settings;
use crate::renderer::SpectrumFrame;
use crate::spectrum::Spectrum;

use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use windows::core::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

const FFT_SIZE: usize = 256;

/// Captures system audio via WASAPI loopback and sends spectrum frames.
pub fn audio_thread(
    frame_tx: Sender<SpectrumFrame>,
    stop: Arc<AtomicBool>,
    settings: Arc<Mutex<Settings>>,
) {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() {
            log::error!("CoInitializeEx failed: {hr:?}");
            return;
        }

        match run_capture(&frame_tx, &stop, &settings) {
            Ok(()) => log::info!("Audio capture thread exiting normally"),
            Err(e) => log::error!("Audio capture failed: {e}"),
        }

        CoUninitialize();
    }
}

unsafe fn run_capture(
    frame_tx: &Sender<SpectrumFrame>,
    stop: &Arc<AtomicBool>,
    settings: &Arc<Mutex<Settings>>,
) -> Result<()> {
    // Get default render device for loopback
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;

    // Stuttering fix: open a render client briefly
    stuttering_fix(&device)?;

    // Open loopback capture
    let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
    let mix_format = audio_client.GetMixFormat()?;
    let wfx = &*mix_format;

    let channels = wfx.nChannels as usize;
    let block_align = wfx.nBlockAlign as usize;

    // Copy packed fields to avoid unaligned reference
    let sample_rate = { wfx.nSamplesPerSec };
    let bits_per_sample = { wfx.wBitsPerSample };
    log::info!(
        "Audio format: {}ch, {}Hz, {} bits, block_align={}",
        channels,
        sample_rate,
        bits_per_sample,
        block_align
    );

    audio_client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
        0,
        0,
        mix_format,
        None,
    )?;

    let capture_client: IAudioCaptureClient = audio_client.GetService()?;
    audio_client.Start()?;

    let cfg = settings.lock().unwrap().clone();
    let mut spectrum_left = Spectrum::with_window(FFT_SIZE, cfg.window_type);
    let mut spectrum_right = Spectrum::with_window(FFT_SIZE, cfg.window_type);
    let bin_size = spectrum_left.bin_size();
    let sample_rate = { wfx.nSamplesPerSec } as f32;

    // Accumulation buffer for interleaved float samples
    let samples_needed = FFT_SIZE * channels;
    let mut sample_buf: Vec<f32> = Vec::with_capacity(samples_needed);

    while !stop.load(Ordering::Relaxed) {
        // Small sleep to let buffer fill
        std::thread::sleep(std::time::Duration::from_millis(5));

        // Check for settings changes
        if let Ok(s) = settings.try_lock() {
            spectrum_left.set_window_type(s.window_type);
            spectrum_right.set_window_type(s.window_type);
        }

        loop {
            let mut data_ptr = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;

            let hr = capture_client.GetBuffer(
                &mut data_ptr,
                &mut num_frames,
                &mut flags,
                None,
                None,
            );
            if hr.is_err() {
                break;
            }

            if num_frames == 0 {
                let _ = capture_client.ReleaseBuffer(0);
                break;
            }

            // Convert raw bytes to f32 samples
            let total_samples = num_frames as usize * channels;
            let float_slice =
                std::slice::from_raw_parts(data_ptr as *const f32, total_samples);

            for &s in float_slice {
                if sample_buf.len() < samples_needed {
                    sample_buf.push(s);
                }
            }

            let _ = capture_client.ReleaseBuffer(num_frames);

            // Once we have enough samples, process FFT
            if sample_buf.len() >= samples_needed {
                let cfg = settings.lock().unwrap().clone();
                let left_mags = spectrum_left.process(&sample_buf, 0, channels);
                let right_mags = if channels >= 2 {
                    spectrum_right.process(&sample_buf, 1, channels)
                } else {
                    left_mags
                };

                // Apply frequency cutoff and optional log spread
                let cutoff_bin = ((cfg.freq_cutoff_hz as f32 / sample_rate) * FFT_SIZE as f32)
                    .ceil() as usize;
                let usable_bins = cutoff_bin.min(bin_size);
                let output_size = bin_size; // keep same output size for renderer

                let mut values = vec![0u8; output_size * 2];

                if cfg.log_spread && usable_bins > 1 {
                    // Log-spread: map output_size bars to usable_bins FFT bins
                    // t^1.15 — very gentle curve
                    for out in 0..output_size {
                        let t0 = out as f32 / output_size as f32;
                        let t1 = (out + 1) as f32 / output_size as f32;
                        let start = (t0.powf(1.15) * usable_bins as f32) as usize;
                        let end = ((t1.powf(1.15) * usable_bins as f32) as usize).max(start + 1).min(usable_bins);

                        let left_val = merge_bins(left_mags, start, end, cfg.bin_merge);
                        let right_val = merge_bins(right_mags, start, end, cfg.bin_merge);

                        values[out] = (left_val * cfg.gain * 255.0).min(255.0) as u8;
                        values[out + output_size] = (right_val * cfg.gain * 255.0).min(255.0) as u8;
                    }
                } else {
                    // Linear: direct mapping with cutoff
                    for i in 0..output_size {
                        if i < usable_bins {
                            values[i] = (left_mags[i] * cfg.gain * 255.0).min(255.0) as u8;
                            values[i + output_size] = (right_mags[i] * cfg.gain * 255.0).min(255.0) as u8;
                        }
                    }
                }

                let _ = frame_tx.try_send(SpectrumFrame { values });
                sample_buf.clear();
            }
        }
    }

    audio_client.Stop()?;
    Ok(())
}

/// The "stuttering fix" from the original C++ code:
/// Open a render client briefly to kick-start the audio engine.
unsafe fn stuttering_fix(device: &IMMDevice) -> Result<()> {
    let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
    let format = client.GetMixFormat()?;

    client.Initialize(
        AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_NOPERSIST,
        10_000_000, // 1 second in 100ns units
        0,
        format,
        None,
    )?;

    let buf_size = client.GetBufferSize()?;
    let render: IAudioRenderClient = client.GetService()?;
    let data = render.GetBuffer(buf_size)?;
    render.ReleaseBuffer(buf_size, AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)?;

    // Don't start, just release
    let _ = data;
    Ok(())
}

fn merge_bins(mags: &[f32], start: usize, end: usize, mode: crate::config::BinMergeMode) -> f32 {
    if start >= end || start >= mags.len() { return 0.0; }
    let end = end.min(mags.len());
    match mode {
        crate::config::BinMergeMode::Max => {
            mags[start..end].iter().cloned().fold(0.0f32, f32::max)
        }
        crate::config::BinMergeMode::Average => {
            let sum: f32 = mags[start..end].iter().sum();
            sum / (end - start) as f32
        }
    }
}
