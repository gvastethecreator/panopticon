//! Persistent user settings for the Panopticon desktop application.
//!
//! Settings are stored in `%APPDATA%\Panopticon\settings.toml` when
//! available, falling back to the system temporary directory if `%APPDATA%`
//! cannot be resolved.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{PanopticonError, Result};
use crate::layout::{LayoutCustomization, LayoutType};

const DEFAULT_REFRESH_INTERVAL_MS: u32 = 2_000;
const REFRESH_INTERVALS_MS: [u32; 4] = [1_000, 2_000, 5_000, 10_000];
const DEFAULT_BACKGROUND_COLOR_HEX: &str = "181513";
const DEFAULT_TAG_COLOR_HEX: &str = "D29A5C";

/// Visual styling persisted for a manual tag/group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TagStyle {
    /// RGB hex string (`RRGGBB`) used to tint grouped content.
    pub color_hex: String,
}

impl Default for TagStyle {
    fn default() -> Self {
        Self {
            color_hex: DEFAULT_TAG_COLOR_HEX.to_owned(),
        }
    }
}

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
    /// Explicit per-app override for `hide_on_select`; `None` means inherit
    /// the current global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_on_select_override: Option<bool>,
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
            hide_on_select_override: None,
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
    /// Global styling associated with each known manual tag.
    pub tag_styles: BTreeMap<String, TagStyle>,
    /// Fixed window width in pixels (`None` = automatic).
    pub fixed_width: Option<u32>,
    /// Fixed window height in pixels (`None` = automatic).
    pub fixed_height: Option<u32>,
    /// Dock the window to a screen edge, reserving desktop space.
    pub dock_edge: Option<DockEdge>,
    /// Target monitor name for docking (e.g. `DISPLAY1`).
    pub dock_monitor: Option<String>,
    /// Base RGB background colour (`RRGGBB`) used for the client area.
    pub background_color_hex: String,
    /// Use Windows 11 backdrop / rounded-corner chrome when available.
    pub use_system_backdrop: bool,
    /// Show the status toolbar at the top of the window.
    pub show_toolbar: bool,
    /// Show per-window title/app information below thumbnails.
    pub show_window_info: bool,
    /// Per-layout custom resize ratios (column/row proportions).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layout_customizations: BTreeMap<String, LayoutCustomization>,
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
            tag_styles: BTreeMap::new(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_monitor: None,
            background_color_hex: DEFAULT_BACKGROUND_COLOR_HEX.to_owned(),
            use_system_backdrop: true,
            show_toolbar: true,
            show_window_info: true,
            layout_customizations: BTreeMap::new(),
        }
    }
}

impl AppSettings {
    /// Return the custom layout override for the given layout type, if any.
    #[must_use]
    pub fn layout_custom(&self, layout: LayoutType) -> Option<&LayoutCustomization> {
        self.layout_customizations
            .get(layout.label())
            .filter(|c| !c.is_empty())
    }

    /// Store (or clear) a layout customization for the given layout type.
    pub fn set_layout_custom(&mut self, layout: LayoutType, custom: LayoutCustomization) {
        if custom.is_empty() {
            self.layout_customizations.remove(layout.label());
        } else {
            self.layout_customizations
                .insert(layout.label().to_owned(), custom);
        }
    }

    /// Clear all custom layout ratios for the given layout type.
    pub fn clear_layout_custom(&mut self, layout: LayoutType) {
        self.layout_customizations.remove(layout.label());
    }

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
            .and_then(|rule| rule.hide_on_select_override)
            .unwrap_or(self.hide_on_select)
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

    /// Return the RGB hex string used for `tag`, or the default tag colour.
    #[must_use]
    pub fn tag_color_hex(&self, tag: &str) -> String {
        let Some(tag) = normalize_tag(tag) else {
            return DEFAULT_TAG_COLOR_HEX.to_owned();
        };

        self.tag_styles.get(&tag).map_or_else(
            || DEFAULT_TAG_COLOR_HEX.to_owned(),
            |style| style.color_hex.clone(),
        )
    }

    /// Return the tag colour encoded as a Windows `COLORREF`-compatible BGR value.
    #[must_use]
    pub fn tag_color_bgr(&self, tag: &str) -> u32 {
        rgb_hex_to_bgr(&self.tag_color_hex(tag)).unwrap_or(0x005C_9AD2)
    }

    /// Return the configured application background colour as BGR.
    #[must_use]
    pub fn background_color_bgr(&self) -> u32 {
        rgb_hex_to_bgr(&self.background_color_hex).unwrap_or(0x0018_1513)
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
        let default_hide_on_select = self.hide_on_select;
        let next = !self.hide_on_select_for(app_id);
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.hide_on_select = next;
        rule.hide_on_select_override = (next != default_hide_on_select).then_some(next);
        next
    }

    /// Toggle a manual tag for a specific application.
    pub fn toggle_app_tag(&mut self, app_id: &str, display_name: &str, tag: &str) -> bool {
        let Some(tag) = normalize_tag(tag) else {
            return false;
        };

        self.tag_styles.entry(tag.clone()).or_default();
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

    /// Create or update a tag with a specific colour and assign it to an app.
    pub fn assign_tag_with_color(
        &mut self,
        app_id: &str,
        display_name: &str,
        tag: &str,
        color_hex: &str,
    ) -> Option<String> {
        let tag = normalize_tag(tag)?;
        let color_hex = normalize_color_hex(color_hex)?;
        self.tag_styles.insert(tag.clone(), TagStyle { color_hex });

        let rule = self.ensure_app_rule(app_id, display_name);
        if !rule.tags.iter().any(|existing| existing == &tag) {
            rule.tags.push(tag.clone());
            rule.tags.sort();
            rule.tags.dedup();
        }

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
            rule.hide_on_select_override = rule.hide_on_select_override.or_else(|| {
                (rule.hide_on_select != self.hide_on_select).then_some(rule.hide_on_select)
            });
            rule.hide_on_select = rule.hide_on_select_override.unwrap_or(self.hide_on_select);
            rule.tags = rule
                .tags
                .iter()
                .filter_map(|tag| normalize_tag(tag))
                .collect();
            rule.tags.sort();
            rule.tags.dedup();
        }

        let known_tags: std::collections::BTreeSet<String> = app_rules
            .values()
            .flat_map(|rule| rule.tags.iter().cloned())
            .collect();

        let mut tag_styles = self
            .tag_styles
            .iter()
            .filter_map(|(tag, style)| {
                let tag = normalize_tag(tag)?;
                if !known_tags.contains(&tag) {
                    return None;
                }

                let color_hex = normalize_color_hex(&style.color_hex)?;
                Some((tag, TagStyle { color_hex }))
            })
            .collect::<BTreeMap<_, _>>();

        for tag in &known_tags {
            tag_styles.entry(tag.clone()).or_default();
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
            tag_styles,
            fixed_width: self.fixed_width,
            fixed_height: self.fixed_height,
            dock_edge: self.dock_edge,
            dock_monitor: self
                .dock_monitor
                .as_deref()
                .and_then(normalize_filter_value),
            background_color_hex: normalize_color_hex(&self.background_color_hex)
                .unwrap_or_else(|| DEFAULT_BACKGROUND_COLOR_HEX.to_owned()),
            use_system_backdrop: self.use_system_backdrop,
            show_toolbar: self.show_toolbar,
            show_window_info: self.show_window_info,
            layout_customizations: self.layout_customizations.clone(),
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
                hide_on_select_override: None,
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

fn normalize_color_hex(color_hex: &str) -> Option<String> {
    let trimmed = color_hex.trim().trim_start_matches('#');
    if trimmed.len() != 6
        || !trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }

    Some(trimmed.to_ascii_uppercase())
}

fn rgb_hex_to_bgr(color_hex: &str) -> Option<u32> {
    let normalized = normalize_color_hex(color_hex)?;
    let value = u32::from_str_radix(&normalized, 16).ok()?;
    let red = (value >> 16) & 0xFF;
    let green = (value >> 8) & 0xFF;
    let blue = value & 0xFF;
    Some((blue << 16) | (green << 8) | red)
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
    use super::{AppRule, AppSettings, HiddenAppEntry, TagStyle};
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
            tag_styles: std::collections::BTreeMap::from([(
                "work".to_owned(),
                TagStyle {
                    color_hex: "3366FF".to_owned(),
                },
            )]),
            fixed_width: Some(120),
            fixed_height: None,
            dock_edge: Some(super::DockEdge::Left),
            dock_monitor: Some("DISPLAY1".to_owned()),
            background_color_hex: "101820".to_owned(),
            use_system_backdrop: true,
            show_toolbar: false,
            show_window_info: false,
            layout_customizations: std::collections::BTreeMap::default(),
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
            tag_styles: std::collections::BTreeMap::default(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_monitor: None,
            background_color_hex: "ZZZZZZ".to_owned(),
            use_system_backdrop: false,
            show_toolbar: true,
            show_window_info: true,
            layout_customizations: std::collections::BTreeMap::default(),
        };

        assert_eq!(settings.normalized().refresh_interval_ms, 2_000);
        assert_eq!(settings.normalized().background_color_hex, "181513");
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
    fn assign_tag_with_color_persists_tag_style() {
        let mut settings = AppSettings::default();

        let created =
            settings.assign_tag_with_color("app:browser", "Arc Browser", " Focus ", "#22AA88");

        assert_eq!(created.as_deref(), Some("focus"));
        assert!(settings.app_has_tag("app:browser", "focus"));
        assert_eq!(settings.tag_color_hex("focus"), "22AA88");
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

    #[test]
    fn normalized_tag_styles_follow_known_tags_only() {
        let mut settings = AppSettings::default();
        let _ = settings.assign_tag_with_color("app:mail", "Mail", "Focus", "ff8844");
        settings.tag_styles.insert(
            "unused".to_owned(),
            TagStyle {
                color_hex: "00FF00".to_owned(),
            },
        );

        let normalized = settings.normalized();

        assert!(normalized.tag_styles.contains_key("focus"));
        assert!(!normalized.tag_styles.contains_key("unused"));
        assert_eq!(normalized.tag_color_hex("focus"), "FF8844");
    }

    #[test]
    fn hide_on_select_defaults_continue_to_follow_global_setting() {
        let mut settings = AppSettings::default();
        settings.app_rules.insert(
            "app:legacy".to_owned(),
            AppRule {
                display_name: "Legacy".to_owned(),
                hidden: false,
                preserve_aspect_ratio: false,
                hide_on_select: true,
                hide_on_select_override: None,
                tags: Vec::new(),
            },
        );

        assert!(settings.hide_on_select_for("app:legacy"));
        settings.hide_on_select = false;

        assert!(!settings.hide_on_select_for("app:legacy"));
    }

    #[test]
    fn normalized_legacy_hide_on_select_difference_becomes_override() {
        let mut settings = AppSettings {
            hide_on_select: false,
            ..AppSettings::default()
        };
        settings.app_rules.insert(
            "app:explicit".to_owned(),
            AppRule {
                display_name: "Explicit".to_owned(),
                hidden: false,
                preserve_aspect_ratio: false,
                hide_on_select: true,
                hide_on_select_override: None,
                tags: Vec::new(),
            },
        );

        let normalized = settings.normalized();

        assert!(normalized.hide_on_select_for("app:explicit"));
        assert_eq!(
            normalized
                .app_rules
                .get("app:explicit")
                .and_then(|rule| rule.hide_on_select_override),
            Some(true)
        );
    }
}
