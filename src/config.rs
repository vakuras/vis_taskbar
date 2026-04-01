use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// RGB color with components in 0.0..=1.0 range.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VisRgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl VisRgb {
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    /// Pack to Win32 COLORREF (0x00BBGGRR).
    pub fn to_colorref(self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        r | (g << 8) | (b << 16)
    }

    /// Unpack from Win32 COLORREF.
    pub fn from_colorref(cr: u32) -> Self {
        Self {
            r: (cr & 0xFF) as f32 / 255.0,
            g: ((cr >> 8) & 0xFF) as f32 / 255.0,
            b: ((cr >> 16) & 0xFF) as f32 / 255.0,
        }
    }
}

/// FFT window function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowType {
    Hann,
    Hamming,
    BlackmanHarris,
}

impl Default for WindowType {
    fn default() -> Self {
        Self::Hann
    }
}

/// How to merge FFT bins when multiple map to one display bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinMergeMode {
    Max,
    Average,
}

impl Default for BinMergeMode {
    fn default() -> Self {
        Self::Max
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub color_top: VisRgb,
    pub color_bottom: VisRgb,
    pub color_peaks: VisRgb,
    pub step_multiplier: u32,
    pub sleep_time_ms: u32,
    pub bars: bool,
    #[serde(default)]
    pub invert_direction: bool,
    #[serde(default)]
    pub window_type: WindowType,
    #[serde(default = "default_freq_cutoff")]
    pub freq_cutoff_hz: u32,
    #[serde(default)]
    pub bin_merge: BinMergeMode,
    #[serde(default = "default_log_spread")]
    pub log_spread: bool,
    #[serde(default = "default_gain")]
    pub gain: f32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

fn default_freq_cutoff() -> u32 { 18000 }
fn default_log_spread() -> bool { false }
fn default_gain() -> f32 { 6.0 }
fn default_opacity() -> f32 { 0.5 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            color_top: VisRgb::new(1.0, 1.0, 0.0),    // yellow
            color_bottom: VisRgb::new(1.0, 0.0, 0.0),  // red
            color_peaks: VisRgb::new(1.0, 1.0, 1.0),   // white
            step_multiplier: 1,
            sleep_time_ms: 15,
            bars: false,
            invert_direction: false,
            window_type: WindowType::Hann,
            freq_cutoff_hz: 18000,
            bin_merge: BinMergeMode::Max,
            log_spread: false,
            gain: 6.0,
            opacity: 0.5,
        }
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        let exe = std::env::current_exe().unwrap_or_default();
        exe.with_extension("toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
                log::warn!("Failed to parse config {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => {
                log::info!("No config file found, using defaults");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        log::info!("Config saved to {}", path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let s = Settings::default();
        assert_eq!(s.sleep_time_ms, 15);
        assert_eq!(s.step_multiplier, 1);
        assert!(!s.bars);
        assert!(s.color_top.r > 0.0);
    }

    #[test]
    fn settings_roundtrip_toml() {
        let s = Settings::default();
        let toml_str = toml::to_string_pretty(&s).unwrap();
        let s2: Settings = toml::from_str(&toml_str).unwrap();
        assert_eq!(s.sleep_time_ms, s2.sleep_time_ms);
        assert_eq!(s.step_multiplier, s2.step_multiplier);
        assert_eq!(s.bars, s2.bars);
    }

    #[test]
    fn visrgb_colorref_roundtrip() {
        let c = VisRgb::new(1.0, 0.5, 0.0);
        let cr = c.to_colorref();
        let c2 = VisRgb::from_colorref(cr);
        assert!((c.r - c2.r).abs() < 0.01);
        assert!((c.g - c2.g).abs() < 0.01);
        assert!((c.b - c2.b).abs() < 0.01);
    }

    #[test]
    fn visrgb_black_white() {
        assert_eq!(VisRgb::new(0.0, 0.0, 0.0).to_colorref(), 0);
        assert_eq!(VisRgb::new(1.0, 1.0, 1.0).to_colorref(), 0x00FFFFFF);
    }

    #[test]
    fn default_new_fields() {
        let s = Settings::default();
        assert_eq!(s.window_type, WindowType::Hann);
        assert_eq!(s.bin_merge, BinMergeMode::Max);
        assert_eq!(s.freq_cutoff_hz, 18000);
        assert!((s.gain - 6.0).abs() < 0.01);
        assert!((s.opacity - 0.5).abs() < 0.01);
        assert!(!s.log_spread);
        assert!(!s.invert_direction);
    }

    #[test]
    fn settings_roundtrip_all_fields() {
        let mut s = Settings::default();
        s.window_type = WindowType::BlackmanHarris;
        s.bin_merge = BinMergeMode::Average;
        s.freq_cutoff_hz = 16000;
        s.gain = 8.5;
        s.opacity = 0.75;
        s.log_spread = true;
        s.invert_direction = true;
        s.bars = true;

        let toml_str = toml::to_string_pretty(&s).unwrap();
        let s2: Settings = toml::from_str(&toml_str).unwrap();

        assert_eq!(s.window_type, s2.window_type);
        assert_eq!(s.bin_merge, s2.bin_merge);
        assert_eq!(s.freq_cutoff_hz, s2.freq_cutoff_hz);
        assert!((s.gain - s2.gain).abs() < 0.01);
        assert!((s.opacity - s2.opacity).abs() < 0.01);
        assert_eq!(s.log_spread, s2.log_spread);
        assert_eq!(s.invert_direction, s2.invert_direction);
        assert_eq!(s.bars, s2.bars);
    }

    #[test]
    fn settings_backwards_compat() {
        // Old config without new fields should still load with defaults
        let old_toml = r#"
            step_multiplier = 2
            sleep_time_ms = 20
            bars = true
            [color_top]
            r = 0.0
            g = 1.0
            b = 0.0
            [color_bottom]
            r = 0.0
            g = 0.0
            b = 1.0
            [color_peaks]
            r = 1.0
            g = 1.0
            b = 1.0
        "#;
        let s: Settings = toml::from_str(old_toml).unwrap();
        assert_eq!(s.step_multiplier, 2);
        assert!(s.bars);
        // New fields should get defaults
        assert_eq!(s.window_type, WindowType::Hann);
        assert!((s.gain - 6.0).abs() < 0.01);
        assert!(!s.invert_direction);
    }
}
