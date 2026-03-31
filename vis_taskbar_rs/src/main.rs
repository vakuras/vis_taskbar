#![allow(unsafe_op_in_unsafe_fn, unused_must_use)]

mod audio;
mod config;
mod renderer;
mod spectrum;
mod taskbar;
mod tray;
mod ui;

use config::Settings;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("vis_taskbar starting");

    let settings = Arc::new(Mutex::new(Settings::load()));
    let is_running = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));

    // Channel for spectrum frames (bounded, drop old frames if renderer is slow)
    let (frame_tx, frame_rx) = crossbeam_channel::bounded(2);

    // Start audio + render
    let start = {
        let settings = settings.clone();
        let is_running = is_running.clone();
        let stop = stop.clone();
        let frame_tx = frame_tx.clone();
        let frame_rx = frame_rx.clone();

        move || {
            if is_running.load(Ordering::Relaxed) {
                return;
            }
            stop.store(false, Ordering::Relaxed);
            is_running.store(true, Ordering::Relaxed);

            // Audio thread
            let stop_a = stop.clone();
            let tx = frame_tx.clone();
            std::thread::Builder::new()
                .name("audio-capture".into())
                .spawn(move || {
                    audio::audio_thread(tx, stop_a);
                })
                .expect("Failed to spawn audio thread");

            // Render thread
            let stop_r = stop.clone();
            let settings_r = settings.clone();
            let is_running_r = is_running.clone();
            let rx = frame_rx.clone();

            let full_taskbar = settings.lock().unwrap().full_taskbar;
            std::thread::Builder::new()
                .name("renderer".into())
                .spawn(move || {
                    if let Some(tb) = taskbar::TaskbarInfo::locate() {
                        renderer::render_loop(rx, settings_r, stop_r, &tb, full_taskbar);
                    } else {
                        log::error!("Failed to locate taskbar");
                    }
                    is_running_r.store(false, Ordering::Relaxed);
                })
                .expect("Failed to spawn render thread");
        }
    };

    let stop_fn = {
        let stop = stop.clone();
        let is_running = is_running.clone();
        move || {
            stop.store(true, Ordering::Relaxed);
            is_running.store(false, Ordering::Relaxed);
        }
    };

    // Auto-start
    start();

    // Run UI on main thread (message loop)
    ui::run_ui(
        settings.clone(),
        is_running.clone(),
        Box::new(start),
        Box::new(stop_fn),
    );

    // Cleanup
    stop.store(true, Ordering::Relaxed);
    log::info!("vis_taskbar exiting");
}
