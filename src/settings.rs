//! Persistent user settings for the Panopticon desktop application.
//!
//! Settings are stored in `%APPDATA%\Panopticon\settings.toml` when
//! available, falling back to the system temporary directory if `%APPDATA%`
//! cannot be resolved.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{PanopticonError, Result};
use crate::layout::LayoutType;

const DEFAULT_REFRESH_INTERVAL_MS: u32 = 2_000;
const REFRESH_INTERVALS_MS: [u32; 4] = [1_000, 2_000, 5_000, 10_000];

/// User preferences persisted between application launches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppSettings {
    /// Layout used when the application starts.
    pub initial_layout: LayoutType,
    /// Refresh interval for window enumeration and layout updates.
    pub refresh_interval_ms: u32,
    /// Whether minimizing the main window should hide it to the tray.
    pub minimize_to_tray: bool,
    /// Whether closing the main window should hide it to the tray.
    pub close_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            initial_layout: LayoutType::Grid,
            refresh_interval_ms: DEFAULT_REFRESH_INTERVAL_MS,
            minimize_to_tray: true,
            close_to_tray: true,
        }
    }
}

impl AppSettings {
    /// Resolve the on-disk settings path.
    #[must_use]
    pub fn path() -> PathBuf {
        let base_dir = std::env::var_os("APPDATA")
            .map_or_else(|| std::env::temp_dir().join("Panopticon"), PathBuf::from);
        base_dir.join("Panopticon").join("settings.toml")
    }

    /// Load settings from disk, returning defaults if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_or_default() -> Result<Self> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let settings: Self = toml::from_str(&contents)
            .map_err(|error| PanopticonError::SettingsParse(error.to_string()))?;
        Ok(settings.normalized())
    }

    /// Persist the current settings to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the settings directory cannot be created or if the
    /// TOML payload cannot be serialized.
    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml = toml::to_string_pretty(&self.normalized())
            .map_err(|error| PanopticonError::SettingsParse(error.to_string()))?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    /// Advance the refresh interval through a curated list.
    pub fn cycle_refresh_interval(&mut self) {
        let current = self.normalized().refresh_interval_ms;
        let next_index = REFRESH_INTERVALS_MS
            .iter()
            .position(|interval| *interval == current)
            .map_or(0, |index| (index + 1) % REFRESH_INTERVALS_MS.len());
        self.refresh_interval_ms = REFRESH_INTERVALS_MS[next_index];
    }

    /// Return a human-friendly refresh-interval label.
    #[must_use]
    pub fn refresh_interval_label(&self) -> String {
        let interval = self.normalized().refresh_interval_ms;
        if interval.is_multiple_of(1_000) {
            format!("{}s", interval / 1_000)
        } else {
            format!("{:.1}s", f64::from(interval) / 1_000.0)
        }
    }

    /// Normalize values loaded from disk so invalid or surprising inputs do
    /// not leak into runtime behaviour.
    #[must_use]
    pub fn normalized(&self) -> Self {
        let refresh_interval_ms = REFRESH_INTERVALS_MS
            .iter()
            .copied()
            .find(|interval| *interval == self.refresh_interval_ms)
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_MS);

        Self {
            initial_layout: self.initial_layout,
            refresh_interval_ms,
            minimize_to_tray: self.minimize_to_tray,
            close_to_tray: self.close_to_tray,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppSettings;
    use crate::layout::LayoutType;

    #[test]
    fn settings_roundtrip_through_toml() {
        let settings = AppSettings {
            initial_layout: LayoutType::Bento,
            refresh_interval_ms: 5_000,
            minimize_to_tray: false,
            close_to_tray: true,
        };

        let encoded = toml::to_string_pretty(&settings).expect("serialize settings");
        let decoded: AppSettings = toml::from_str(&encoded).expect("deserialize settings");

        assert_eq!(decoded, settings);
    }

    #[test]
    fn invalid_refresh_interval_normalizes_to_default() {
        let settings = AppSettings {
            initial_layout: LayoutType::Columns,
            refresh_interval_ms: 777,
            minimize_to_tray: true,
            close_to_tray: false,
        };

        assert_eq!(settings.normalized().refresh_interval_ms, 2_000);
    }

    #[test]
    fn cycle_refresh_interval_moves_to_next_known_value() {
        let mut settings = AppSettings::default();
        settings.cycle_refresh_interval();
        assert_eq!(settings.refresh_interval_ms, 5_000);
        settings.cycle_refresh_interval();
        assert_eq!(settings.refresh_interval_ms, 10_000);
    }
}
