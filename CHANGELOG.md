# Changelog

All notable changes to vis_taskbar are documented in this file.

## [0.5.0] - 2026-03-31

### Changed
- **Complete rewrite in Rust** — replaced all C++ code with safe, modern Rust
- Replaced KISS FFT with `rustfft` crate (Hamming window, 256-point FFT → 128 bins)
- Replaced raw binary config with TOML via `serde` (human-readable, versioned)
- Replaced unsafe `TerminateThread` with cooperative `AtomicBool` stop signaling
- Replaced shared mutable buffers with `crossbeam-channel` and `Arc<Mutex>`
- Thread-safe audio→renderer communication (no more data races)

### Added
- Modern dark-themed settings UI (Eventide Midnight palette)
- Dark title bar via `DwmSetWindowAttribute`
- Owner-drawn buttons with accent coloring
- Live spectrum preview in settings dialog (shared OpenGL renderer)
- Restart button for audio device changes
- System tray icon with custom embedded icon
- `Segoe UI` font with semibold section headers
- `env_logger` for structured logging
- `build.rs` for Windows resource compilation (embedded icon)

### Removed
- Winamp plugin support (`vis_taskbar`)
- foobar2000 component support (`foo_vis_taskbar`)
- All C++ source code, Visual Studio projects, and `.sln` files
- KISS FFT library (replaced by `rustfft`)
- Windows 7 / Aero version check (now works on Windows 10/11)

### Fixed
- OS version check used `&&` instead of `||` (wrong OS versions could pass)
- WASAPI audio buffer position tracked frames instead of bytes
- `TerminateThread()` in cleanup paths could leak resources or deadlock
- Shared buffers accessed across threads without synchronization
- GDI brush leaked on every `WM_CTLCOLORSTATIC` repaint
- FFT inverse window could divide by zero at window endpoints
- `GetVersionEx` deprecated API usage
- Raw binary `SETTINGS` struct had no versioning or validation

## [0.1.4] - 2011

### Added
- foobar2000 component (`foo_vis_taskbar`) — visualization via foobar2000's spectrum stream
- foobar2000 preferences page with color pickers and configuration

## [0.1.3] - 2011

### Added
- Standalone Windows 7 application (`win7_vis_taskbar`)
- WASAPI loopback audio capture (no media player dependency)
- System tray icon with context menu (Show/Hide, Start/Stop, Exit)
- Preferences dialog with color pickers, sleep time, step multiplier
- Stuttering fix (brief render client initialization before capture)

## [0.1.2] - 2010

### Added
- Shared rendering engine (`vis_taskbar_common`)
  - Taskbar discovery (`Shell_TrayWnd` + child enumeration)
  - OpenGL overlay window (WS_POPUP, WS_EX_TOOLWINDOW)
  - Mirrored left/right spectrum rendering with gradient colors
  - Peak indicator line overlay
  - Bar falloff (-7/frame) and peak falloff (-2/frame)
  - Configuration load/save (binary `.cfg` file)
  - Full taskbar vs. task list area mode
  - Bar mode vs. smooth fill mode
- FFT processing library (`fft_spectrum`)
  - KISS FFT integration (BSD licensed)
  - Hamming window support
  - Lazy signal/cartesian/polar state machine
  - Real FFT optimization via `kiss_fftr`

## [0.1.1] - 2010

### Added
- Winamp visualization plugin (`vis_taskbar`)
- Receives spectrum data from Winamp's visualization API
- DLL exports: `winampVisGetHeader`, module callbacks
- Preferences dialog for color and timing configuration

## [0.1.0] - 2010

### Added
- Initial proof of concept
- OpenGL rendering on Windows 7 taskbar via Aero glass transparency
- Basic spectrum bar visualization with gradient coloring
