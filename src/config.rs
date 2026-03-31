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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub color_top: VisRgb,
    pub color_bottom: VisRgb,
    pub color_peaks: VisRgb,
    pub step_multiplier: u32,
    pub sleep_time_ms: u32,
    pub bars: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            color_top: VisRgb::new(1.0, 1.0, 0.0),    // yellow
            color_bottom: VisRgb::new(1.0, 0.0, 0.0),  // red
            color_peaks: VisRgb::new(1.0, 1.0, 1.0),   // white
            step_multiplier: 1,
            sleep_time_ms: 15,
            bars: false,
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
}
