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
    /// Manual tags used to build custom groups and filters.
    pub tags: Vec<String>,
}

impl Default for AppRule {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            hidden: false,
            preserve_aspect_ratio: false,
            hide_on_select: true,
            tags: Vec::new(),
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

/// Lightweight application entry used by tray filter menus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSelectionEntry {
    /// Stable application identifier used for persistence.
    pub app_id: String,
    /// Human-friendly label shown to the user.
    pub label: String,
}

/// Edge of the screen where the window can be docked as an app-bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DockEdge {
    Left,
    Right,
    Top,
    Bottom,
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
    /// Optional global filter limiting windows to a specific monitor.
    pub active_monitor_filter: Option<String>,
    /// Optional global filter limiting windows to a specific manual tag.
    pub active_tag_filter: Option<String>,
    /// Optional global filter limiting windows to a single application.
    pub active_app_filter: Option<String>,
    /// Per-application remembered behaviour.
    pub app_rules: BTreeMap<String, AppRule>,
    /// Fixed window width in pixels (`None` = automatic).
    pub fixed_width: Option<u32>,
    /// Fixed window height in pixels (`None` = automatic).
    pub fixed_height: Option<u32>,
    /// Dock the window to a screen edge, reserving desktop space.
    pub dock_edge: Option<DockEdge>,
    /// Target monitor name for docking (e.g. `DISPLAY1`).
    pub dock_monitor: Option<String>,
    /// Show the status toolbar at the top of the window.
    pub show_toolbar: bool,
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
            active_monitor_filter: None,
            active_tag_filter: None,
            active_app_filter: None,
            app_rules: BTreeMap::new(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_monitor: None,
            show_toolbar: true,
        }
    }
}

impl AppSettings {
    /// Resolve the on-disk settings path for a given instance profile.
    #[must_use]
    pub fn path_for(profile: Option<&str>) -> PathBuf {
        let base_dir = std::env::var_os("APPDATA")
            .map_or_else(|| std::env::temp_dir().join("Panopticon"), PathBuf::from);
        let base = base_dir.join("Panopticon");
        match profile.filter(|p| !p.trim().is_empty()) {
            Some(name) => base.join("profiles").join(format!("{}.toml", name.trim())),
            None => base.join("settings.toml"),
        }
    }

    /// Resolve the on-disk settings path (default profile).
    #[must_use]
    pub fn path() -> PathBuf {
        Self::path_for(None)
    }

    /// Load settings from disk, returning defaults if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_or_default(profile: Option<&str>) -> Result<Self> {
        let path = Self::path_for(profile);
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
    pub fn save(&self, profile: Option<&str>) -> Result<()> {
        let path = Self::path_for(profile);
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

    /// Returns `true` when `app_id` belongs to `tag`.
    #[must_use]
    pub fn app_has_tag(&self, app_id: &str, tag: &str) -> bool {
        let Some(tag) = normalize_tag(tag) else {
            return false;
        };

        self.app_rules
            .get(app_id)
            .is_some_and(|rule| rule.tags.iter().any(|existing| existing == &tag))
    }

    /// Return all known tags sorted alphabetically.
    #[must_use]
    pub fn known_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .app_rules
            .values()
            .flat_map(|rule| rule.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Return the remembered tags for an application.
    #[must_use]
    pub fn tags_for(&self, app_id: &str) -> Vec<String> {
        self.app_rules
            .get(app_id)
            .map_or_else(Vec::new, |rule| rule.tags.clone())
    }

    /// Set the active global monitor filter.
    pub fn set_monitor_filter(&mut self, monitor: Option<&str>) {
        self.active_monitor_filter = monitor.and_then(normalize_filter_value);
    }

    /// Set the active manual tag/group filter.
    pub fn set_tag_filter(&mut self, tag: Option<&str>) {
        self.active_tag_filter = tag.and_then(normalize_tag);
        if self.active_tag_filter.is_some() {
            self.active_app_filter = None;
        }
    }

    /// Set the active automatic application filter.
    pub fn set_app_filter(&mut self, app_id: Option<&str>) {
        self.active_app_filter = app_id.and_then(normalize_filter_value);
        if self.active_app_filter.is_some() {
            self.active_tag_filter = None;
        }
    }

    /// Return a user-friendly description of the currently active group filter.
    #[must_use]
    pub fn active_group_filter_label(&self) -> Option<String> {
        if let Some(tag) = &self.active_tag_filter {
            return Some(format!("tag:{tag}"));
        }

        self.active_app_filter.as_ref().map(|app_id| {
            self.app_rules.get(app_id).map_or_else(
                || format!("app:{app_id}"),
                |rule| {
                    if rule.display_name.trim().is_empty() {
                        format!("app:{app_id}")
                    } else {
                        format!("app:{}", rule.display_name)
                    }
                },
            )
        })
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

    /// Toggle a manual tag for a specific application.
    pub fn toggle_app_tag(&mut self, app_id: &str, display_name: &str, tag: &str) -> bool {
        let Some(tag) = normalize_tag(tag) else {
            return false;
        };

        let rule = self.ensure_app_rule(app_id, display_name);
        if let Some(index) = rule.tags.iter().position(|existing| existing == &tag) {
            rule.tags.remove(index);
            false
        } else {
            rule.tags.push(tag);
            rule.tags.sort();
            rule.tags.dedup();
            true
        }
    }

    /// Create a new tag based on the app display name and assign it to the app.
    pub fn create_tag_from_app(&mut self, app_id: &str, display_name: &str) -> Option<String> {
        let tag = derive_tag_from_label(display_name)?;
        let _ = self.toggle_app_tag(app_id, display_name, &tag);
        Some(tag)
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
            rule.tags = rule
                .tags
                .iter()
                .filter_map(|tag| normalize_tag(tag))
                .collect();
            rule.tags.sort();
            rule.tags.dedup();
        }

        let active_monitor_filter = self
            .active_monitor_filter
            .as_deref()
            .and_then(normalize_filter_value);
        let active_tag_filter = self.active_tag_filter.as_deref().and_then(normalize_tag);
        let active_app_filter = if active_tag_filter.is_some() {
            None
        } else {
            self.active_app_filter
                .as_deref()
                .and_then(normalize_filter_value)
        };

        Self {
            initial_layout: self.initial_layout,
            refresh_interval_ms,
            minimize_to_tray: self.minimize_to_tray,
            close_to_tray: self.close_to_tray,
            preserve_aspect_ratio: self.preserve_aspect_ratio,
            hide_on_select: self.hide_on_select,
            animate_transitions: self.animate_transitions,
            always_on_top: self.always_on_top,
            active_monitor_filter,
            active_tag_filter,
            active_app_filter,
            app_rules,
            fixed_width: self.fixed_width,
            fixed_height: self.fixed_height,
            dock_edge: self.dock_edge,
            dock_monitor: self
                .dock_monitor
                .as_deref()
                .and_then(normalize_filter_value),
            show_toolbar: self.show_toolbar,
        }
    }

    fn ensure_app_rule(&mut self, app_id: &str, display_name: &str) -> &mut AppRule {
        let display_name = display_name.trim();
        let preserve_aspect_ratio = self.preserve_aspect_ratio;
        let hide_on_select = self.hide_on_select;

        let rule = self
            .app_rules
            .entry(app_id.to_owned())
            .or_insert_with(|| AppRule {
                display_name: display_name.to_owned(),
                hidden: false,
                preserve_aspect_ratio,
                hide_on_select,
                tags: Vec::new(),
            });

        if rule.display_name.trim().is_empty() && !display_name.is_empty() {
            display_name.clone_into(&mut rule.display_name);
        }

        rule
    }
}

fn normalize_filter_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn normalize_tag(tag: &str) -> Option<String> {
    let collapsed = tag.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_ascii_lowercase())
}

fn derive_tag_from_label(label: &str) -> Option<String> {
    let seed: String = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();

    normalize_tag(&seed)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
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
            active_monitor_filter: Some("DISPLAY1".to_owned()),
            active_tag_filter: Some("work".to_owned()),
            active_app_filter: None,
            app_rules: std::collections::BTreeMap::default(),
            fixed_width: Some(120),
            fixed_height: None,
            dock_edge: Some(super::DockEdge::Left),
            dock_monitor: Some("DISPLAY1".to_owned()),
            show_toolbar: false,
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
            active_monitor_filter: Some("DISPLAY2".to_owned()),
            active_tag_filter: None,
            active_app_filter: Some("exe:demo".to_owned()),
            app_rules: std::collections::BTreeMap::default(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_monitor: None,
            show_toolbar: true,
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
        assert!(rule.tags.is_empty());
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

    #[test]
    fn tag_assignment_is_normalized_and_toggleable() {
        let mut settings = AppSettings::default();

        assert!(settings.toggle_app_tag("app:demo", "Demo App", "  Work Bench "));
        assert!(settings.app_has_tag("app:demo", "work bench"));
        assert_eq!(settings.known_tags(), vec!["work bench".to_owned()]);

        assert!(!settings.toggle_app_tag("app:demo", "Demo App", "work bench"));
        assert!(!settings.app_has_tag("app:demo", "work bench"));
    }

    #[test]
    fn creating_tag_from_app_uses_normalized_label() {
        let mut settings = AppSettings::default();

        let created = settings.create_tag_from_app("app:browser", "Arc Browser");

        assert_eq!(created.as_deref(), Some("arc browser"));
        assert!(settings.app_has_tag("app:browser", "arc browser"));
    }

    #[test]
    fn selecting_tag_filter_clears_app_filter() {
        let mut settings = AppSettings::default();
        settings.set_app_filter(Some("exe:arc"));

        settings.set_tag_filter(Some("workspace"));

        assert_eq!(settings.active_tag_filter.as_deref(), Some("workspace"));
        assert!(settings.active_app_filter.is_none());
    }

    #[test]
    fn normalized_settings_drop_empty_filters_and_tags() {
        let mut settings = AppSettings {
            active_monitor_filter: Some("   ".to_owned()),
            active_tag_filter: Some("  Focused Work  ".to_owned()),
            active_app_filter: Some("   ".to_owned()),
            ..AppSettings::default()
        };
        let _ = settings.toggle_app_tag("app:one", "One", "  Alpha Team ");
        let _ = settings.toggle_app_tag("app:one", "One", " ");

        let normalized = settings.normalized();

        assert!(normalized.active_monitor_filter.is_none());
        assert_eq!(
            normalized.active_tag_filter.as_deref(),
            Some("focused work")
        );
        assert!(normalized.active_app_filter.is_none());
        assert_eq!(
            normalized.tags_for("app:one"),
            vec!["alpha team".to_owned()]
        );
    }
}
