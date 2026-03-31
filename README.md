# vis_taskbar

A real-time audio spectrum visualizer that renders directly on the Windows taskbar. Captures system audio via WASAPI loopback and draws mirrored frequency bars with gradient coloring and peak indicators using OpenGL.

![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)

## Features

- **Taskbar spectrum visualization** — renders audio bars directly behind/inside the Windows taskbar
- **WASAPI loopback capture** — visualizes any audio playing on the system, no plugins needed
- **Mirrored stereo display** — left channel spreads left from center, right channel spreads right
- **Configurable colors** — gradient top/bottom colors and peak line color
- **Bar and line modes** — discrete bars or smooth filled spectrum
- **Falloff animation** — smooth bar decay (-7/frame) with slower peak markers (-2/frame)
- **Dark-themed settings UI** — Eventide Midnight dark theme with live spectrum preview
- **System tray integration** — runs in the background with tray icon and context menu
- **TOML configuration** — human-readable config file, auto-saved next to the executable

## Screenshot

The visualizer draws a mirrored spectrum on the taskbar, with configurable gradient colors and peak indicators. The settings dialog provides a live preview of the current audio.

## Building

Requires Rust 1.75+ and Windows 10/11.

```bash
cargo build --release
```

The binary is output to `target/release/vis_taskbar.exe`.

## Usage

1. Run `vis_taskbar.exe`
2. A tray icon appears — right-click for menu (Settings, Start/Stop, Exit)
3. Double-click the tray icon to open the settings dialog
4. Play any audio — the spectrum appears on your taskbar

### Settings

| Setting | Description | Default |
|---------|-------------|---------|
| Full Taskbar | Use entire taskbar width vs. task list area only | On |
| Bar Mode | Discrete bars vs. smooth filled spectrum | Off |
| Refresh interval | Render loop delay in milliseconds | 15ms |
| Bar width multiplier | Scale factor for bar width | 1 |
| Top gradient | Color at the top of bars | Yellow |
| Bottom gradient | Color at the bottom of bars | Red |
| Peak line | Color of the peak indicator line | White |

## Architecture

```
src/
  main.rs        — entry point, thread orchestration
  audio.rs       — WASAPI loopback capture + stuttering fix
  spectrum.rs    — FFT via rustfft with Hamming window (256 → 128 bins)
  renderer.rs    — OpenGL spectrum rendering on taskbar overlay
  taskbar.rs     — Shell_TrayWnd discovery and rect tracking
  config.rs      — Settings struct with serde + TOML persistence
  tray.rs        — System tray icon and context menu
  ui.rs          — Dark-themed Win32 settings dialog with live preview
```

### Dependencies

- [windows](https://crates.io/crates/windows) — Win32 API bindings (WASAPI, OpenGL, DWM, Shell)
- [rustfft](https://crates.io/crates/rustfft) — Fast Fourier Transform
- [serde](https://crates.io/crates/serde) + [toml](https://crates.io/crates/toml) — Configuration serialization
- [crossbeam-channel](https://crates.io/crates/crossbeam-channel) — Lock-free audio→renderer communication
- [log](https://crates.io/crates/log) + [env_logger](https://crates.io/crates/env_logger) — Logging

## History

**vis_taskbar** started in 2010 as a personal project to bring audio visualization to the Windows 7 taskbar — inspired by the Aero glass transparency that made overlay rendering possible. The original C++ implementation supported three host modes:

- **Winamp plugin** (`vis_taskbar`) — received spectrum data from Winamp's visualization API
- **foobar2000 component** (`foo_vis_taskbar`) — pulled spectrum from foobar2000's visualization stream
- **Standalone application** (`win7_vis_taskbar`) — captured system audio via WASAPI loopback

All three shared a common OpenGL rendering engine (`vis_taskbar_common`) and FFT processing layer (`fft_spectrum`, using KISS FFT).

The code was uploaded to GitHub in **August 2017**.

In **March 2026**, the project was rewritten from scratch in Rust:
- Removed the Winamp and foobar2000 plugins (both players are effectively legacy)
- Replaced KISS FFT with `rustfft`
- Replaced raw binary config with TOML via `serde`
- Added a modern dark-themed settings UI with live spectrum preview
- Fixed all original C++ bugs (thread safety, GDI leaks, unsafe thread termination, buffer overflows)

## License

MIT License — Copyright (c) 2010-2026 Vadim Kuras