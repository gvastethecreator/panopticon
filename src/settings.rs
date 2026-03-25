//! Persistent user settings for the Panopticon desktop application.
//!
//! Settings are stored in `%APPDATA%\Panopticon\settings.toml` when
//! available, falling back to the system temporary directory if `%APPDATA%`
//! cannot be resolved.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{PanopticonError, Result};
use crate::layout::LayoutType;

const DEFAULT_REFRESH_INTERVAL_MS: u32 = 2_000;
const REFRESH_INTERVALS_MS: [u32; 4] = [1_000, 2_000, 5_000, 10_000];

/// Persisted preferences for an individual application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppRule {
    /// Human-friendly label shown in restore menus.
    pub display_name: String,
    /// Whether windows from this application should be excluded from the layout.
    pub hidden: bool,
    /// Whether Panopticon should preserve the source aspect ratio for this app.
    pub preserve_aspect_ratio: bool,
    /// Whether activating this app should hide Panopticon afterwards.
    pub hide_on_select: bool,
}

impl Default for AppRule {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            hidden: false,
            preserve_aspect_ratio: false,
            hide_on_select: true,
        }
    }
}

/// Lightweight hidden-app entry used by menus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenAppEntry {
    /// Stable application identifier used for persistence.
    pub app_id: String,
    /// Human-friendly label shown to the user.
    pub label: String,
}

/// User preferences persisted between application launches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct AppSettings {
    /// Layout used when the application starts.
    pub initial_layout: LayoutType,
    /// Refresh interval for window enumeration and layout updates.
    pub refresh_interval_ms: u32,
    /// Whether minimizing the main window should hide it to the tray.
    pub minimize_to_tray: bool,
    /// Whether closing the main window should hide it to the tray.
    pub close_to_tray: bool,
    /// Default aspect-ratio behaviour for newly customized apps.
    pub preserve_aspect_ratio: bool,
    /// Whether activating a window should hide Panopticon by default.
    pub hide_on_select: bool,
    /// Whether layout transitions should be animated.
    pub animate_transitions: bool,
    /// Whether the Panopticon window should stay topmost.
    pub always_on_top: bool,
    /// Per-application remembered behaviour.
    pub app_rules: BTreeMap<String, AppRule>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            initial_layout: LayoutType::Grid,
            refresh_interval_ms: DEFAULT_REFRESH_INTERVAL_MS,
            minimize_to_tray: true,
            close_to_tray: true,
            preserve_aspect_ratio: false,
            hide_on_select: true,
            animate_transitions: true,
            always_on_top: false,
            app_rules: BTreeMap::new(),
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

    /// Returns `true` when the application identified by `app_id` is hidden.
    #[must_use]
    pub fn is_hidden(&self, app_id: &str) -> bool {
        self.app_rules.get(app_id).is_some_and(|rule| rule.hidden)
    }

    /// Returns the effective aspect-ratio preference for `app_id`.
    #[must_use]
    pub fn preserve_aspect_ratio_for(&self, app_id: &str) -> bool {
        self.app_rules
            .get(app_id)
            .map_or(self.preserve_aspect_ratio, |rule| {
                rule.preserve_aspect_ratio
            })
    }

    /// Returns the effective hide-on-select preference for `app_id`.
    #[must_use]
    pub fn hide_on_select_for(&self, app_id: &str) -> bool {
        self.app_rules
            .get(app_id)
            .map_or(self.hide_on_select, |rule| rule.hide_on_select)
    }

    /// Toggle hidden state for `app_id`, creating a remembered app rule if necessary.
    pub fn toggle_hidden(&mut self, app_id: &str, display_name: &str) -> bool {
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.hidden = !rule.hidden;
        rule.hidden
    }

    /// Restore a single hidden app.
    pub fn restore_hidden_app(&mut self, app_id: &str) -> bool {
        if let Some(rule) = self.app_rules.get_mut(app_id) {
            let was_hidden = rule.hidden;
            rule.hidden = false;
            was_hidden
        } else {
            false
        }
    }

    /// Restore all hidden apps, returning how many states changed.
    pub fn restore_all_hidden_apps(&mut self) -> usize {
        let mut restored = 0;
        for rule in self.app_rules.values_mut() {
            if rule.hidden {
                rule.hidden = false;
                restored += 1;
            }
        }
        restored
    }

    /// Toggle aspect-ratio preservation for a specific application.
    pub fn toggle_app_preserve_aspect_ratio(&mut self, app_id: &str, display_name: &str) -> bool {
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.preserve_aspect_ratio = !rule.preserve_aspect_ratio;
        rule.preserve_aspect_ratio
    }

    /// Toggle hide-on-select for a specific application.
    pub fn toggle_app_hide_on_select(&mut self, app_id: &str, display_name: &str) -> bool {
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.hide_on_select = !rule.hide_on_select;
        rule.hide_on_select
    }

    /// Update the label stored for a remembered application without changing its behaviour.
    pub fn refresh_app_label(&mut self, app_id: &str, display_name: &str) {
        if let Some(rule) = self.app_rules.get_mut(app_id) {
            let display_name = display_name.trim();
            if rule.display_name != display_name && !display_name.is_empty() {
                display_name.clone_into(&mut rule.display_name);
            }
        }
    }

    /// Return a user-friendly list of hidden apps for menus.
    #[must_use]
    pub fn hidden_app_entries(&self) -> Vec<HiddenAppEntry> {
        let mut entries: Vec<HiddenAppEntry> = self
            .app_rules
            .iter()
            .filter(|(_, rule)| rule.hidden)
            .map(|(app_id, rule)| HiddenAppEntry {
                app_id: app_id.clone(),
                label: if rule.display_name.trim().is_empty() {
                    "Hidden app".to_owned()
                } else {
                    rule.display_name.clone()
                },
            })
            .collect();

        entries.sort_by(|left, right| left.label.cmp(&right.label));
        entries
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

        let mut app_rules = self.app_rules.clone();
        app_rules.retain(|app_id, _| !app_id.trim().is_empty());
        for rule in app_rules.values_mut() {
            rule.display_name = rule.display_name.trim().to_owned();
        }

        Self {
            initial_layout: self.initial_layout,
            refresh_interval_ms,
            minimize_to_tray: self.minimize_to_tray,
            close_to_tray: self.close_to_tray,
            preserve_aspect_ratio: self.preserve_aspect_ratio,
            hide_on_select: self.hide_on_select,
            animate_transitions: self.animate_transitions,
            always_on_top: self.always_on_top,
            app_rules,
        }
    }

    fn ensure_app_rule(&mut self, app_id: &str, display_name: &str) -> &mut AppRule {
        let display_name = display_name.trim();
        let preserve_aspect_ratio = self.preserve_aspect_ratio;
        let hide_on_select = self.hide_on_select;
        self.app_rules
            .entry(app_id.to_owned())
            .or_insert_with(|| AppRule {
                display_name: display_name.to_owned(),
                hidden: false,
                preserve_aspect_ratio,
                hide_on_select,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, HiddenAppEntry};
    use crate::layout::LayoutType;

    #[test]
    fn settings_roundtrip_through_toml() {
        let settings = AppSettings {
            initial_layout: LayoutType::Bento,
            refresh_interval_ms: 5_000,
            minimize_to_tray: false,
            close_to_tray: true,
            preserve_aspect_ratio: true,
            hide_on_select: false,
            animate_transitions: false,
            always_on_top: true,
            app_rules: Default::default(),
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
            preserve_aspect_ratio: false,
            hide_on_select: true,
            animate_transitions: true,
            always_on_top: false,
            app_rules: Default::default(),
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

    #[test]
    fn app_rules_inherit_defaults_when_first_created() {
        let mut settings = AppSettings {
            preserve_aspect_ratio: true,
            hide_on_select: false,
            ..AppSettings::default()
        };

        let hidden = settings.toggle_hidden("app:demo", "Demo App");

        assert!(hidden);
        let rule = settings.app_rules.get("app:demo").expect("app rule exists");
        assert!(rule.hidden);
        assert!(rule.preserve_aspect_ratio);
        assert!(!rule.hide_on_select);
    }

    #[test]
    fn restore_hidden_entries_returns_sorted_labels() {
        let mut settings = AppSettings::default();
        let _ = settings.toggle_hidden("two", "Zulu");
        let _ = settings.toggle_hidden("one", "Alpha");

        assert_eq!(
            settings.hidden_app_entries(),
            vec![
                HiddenAppEntry {
                    app_id: "one".to_owned(),
                    label: "Alpha".to_owned(),
                },
                HiddenAppEntry {
                    app_id: "two".to_owned(),
                    label: "Zulu".to_owned(),
                },
            ]
        );
    }
}
