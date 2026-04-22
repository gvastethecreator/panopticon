//! Persistent user settings for the Panopticon desktop application.
//!
//! Settings are stored in `%APPDATA%\Panopticon\settings.toml` when
//! available, falling back to the system temporary directory if `%APPDATA%`
//! cannot be resolved.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{PanopticonError, Result};
use crate::i18n;
use crate::layout::{LayoutCustomization, LayoutType};

const DEFAULT_REFRESH_INTERVAL_MS: u32 = 2_000;
const REFRESH_INTERVALS_MS: [u32; 4] = [1_000, 2_000, 5_000, 10_000];
const DEFAULT_BACKGROUND_COLOR_HEX: &str = "181513";
const DEFAULT_BACKGROUND_IMAGE_OPACITY_PCT: u8 = 25;
const DEFAULT_THUMBNAIL_RENDER_SCALE_PCT: u8 = 100;
const MIN_THUMBNAIL_RENDER_SCALE_PCT: u8 = 50;
const DEFAULT_TAG_COLOR_HEX: &str = "D29A5C";
const DEFAULT_SHORTCUT_LAYOUT_GRID: &str = "1";
const DEFAULT_SHORTCUT_LAYOUT_MOSAIC: &str = "2";
const DEFAULT_SHORTCUT_LAYOUT_BENTO: &str = "3";
const DEFAULT_SHORTCUT_LAYOUT_FIBONACCI: &str = "4";
const DEFAULT_SHORTCUT_LAYOUT_COLUMNS: &str = "5";
const DEFAULT_SHORTCUT_LAYOUT_ROW: &str = "6";
const DEFAULT_SHORTCUT_LAYOUT_COLUMN: &str = "7";
const DEFAULT_SHORTCUT_RESET_LAYOUT: &str = "0";
const DEFAULT_SHORTCUT_CYCLE_LAYOUT: &str = "Tab";
const DEFAULT_SHORTCUT_CYCLE_THEME: &str = "T";
const DEFAULT_SHORTCUT_TOGGLE_ANIMATIONS: &str = "A";
const DEFAULT_SHORTCUT_TOGGLE_TOOLBAR: &str = "H";
const DEFAULT_SHORTCUT_TOGGLE_WINDOW_INFO: &str = "I";
const DEFAULT_SHORTCUT_TOGGLE_ALWAYS_ON_TOP: &str = "P";
const DEFAULT_SHORTCUT_OPEN_SETTINGS: &str = "O";
const DEFAULT_SHORTCUT_OPEN_MENU: &str = "M";
const DEFAULT_SHORTCUT_GLOBAL_ACTIVATE: &str = "Ctrl+Alt+P";
const DEFAULT_SHORTCUT_REFRESH: &str = "R";
const DEFAULT_SHORTCUT_EXIT: &str = "Esc";
const INVALID_PROFILE_NAME_CHARACTERS: [char; 9] = [':', '"', '<', '>', '|', '?', '*', '/', '\\'];

const fn default_true() -> bool {
    true
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "serde field defaults for Option<T> must return the wrapped field type"
)]
fn default_global_activate_shortcut() -> Option<String> {
    Some(DEFAULT_SHORTCUT_GLOBAL_ACTIVATE.to_owned())
}

/// Supported final key for the configurable global activation hotkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalHotkeyKey {
    /// ASCII letter or digit that maps directly to a Win32 virtual key.
    Character(char),
    /// Function-key row (`F1` .. `F24`).
    Function(u8),
    /// Tab key.
    Tab,
    /// Escape key.
    Esc,
    /// Enter / Return key.
    Enter,
    /// Space bar.
    Space,
}

/// Parsed global activation hotkey with its modifier set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalHotkeyBinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: GlobalHotkeyKey,
}

impl GlobalHotkeyBinding {
    /// Canonical display form used for persistence and the settings editor.
    #[must_use]
    pub fn canonical_string(self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl".to_owned());
        }
        if self.alt {
            parts.push("Alt".to_owned());
        }
        if self.shift {
            parts.push("Shift".to_owned());
        }
        parts.push(match self.key {
            GlobalHotkeyKey::Character(character) => character.to_string(),
            GlobalHotkeyKey::Function(index) => format!("F{index}"),
            GlobalHotkeyKey::Tab => "Tab".to_owned(),
            GlobalHotkeyKey::Esc => "Esc".to_owned(),
            GlobalHotkeyKey::Enter => "Enter".to_owned(),
            GlobalHotkeyKey::Space => "Space".to_owned(),
        });
        parts.join("+")
    }
}

/// Validation result for an interactively supplied profile name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileNameValidation {
    /// Input was blank after trimming surrounding whitespace.
    Empty,
    /// Input is valid and can be stored as a profile filename.
    Valid(String),
    /// Input contains characters that Windows filenames cannot represent.
    Invalid(String),
}

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

/// Thumbnail refresh strategy for individual applications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThumbnailRefreshMode {
    /// Default: real-time DWM mirroring (live preview).
    #[default]
    Realtime,
    /// Frozen: the thumbnail is captured once and not refreshed.
    Frozen,
    /// Interval: the thumbnail refreshes every N milliseconds.
    Interval,
}

/// Strategy used to keep related windows visually grouped together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WindowGrouping {
    #[default]
    None,
    Application,
    Monitor,
    WindowTitle,
    ClassName,
}

/// How the optional background image should be fit into the dashboard canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundImageFit {
    #[default]
    Cover,
    Contain,
    Fill,
    Preserve,
}

/// Customizable keyboard shortcuts used by the dashboard window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ShortcutBindings {
    pub layout_grid: String,
    pub layout_mosaic: String,
    pub layout_bento: String,
    pub layout_fibonacci: String,
    pub layout_columns: String,
    pub layout_row: String,
    pub layout_column: String,
    pub reset_layout: String,
    pub cycle_layout: String,
    pub cycle_theme: String,
    pub toggle_animations: String,
    pub toggle_toolbar: String,
    pub toggle_window_info: String,
    pub toggle_always_on_top: String,
    pub open_settings: String,
    pub open_menu: String,
    #[serde(
        default = "default_global_activate_shortcut",
        skip_serializing_if = "Option::is_none"
    )]
    pub global_activate: Option<String>,
    pub refresh_now: String,
    pub exit_app: String,
    #[serde(default = "default_true")]
    pub alt_toggles_toolbar: bool,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        Self {
            layout_grid: DEFAULT_SHORTCUT_LAYOUT_GRID.to_owned(),
            layout_mosaic: DEFAULT_SHORTCUT_LAYOUT_MOSAIC.to_owned(),
            layout_bento: DEFAULT_SHORTCUT_LAYOUT_BENTO.to_owned(),
            layout_fibonacci: DEFAULT_SHORTCUT_LAYOUT_FIBONACCI.to_owned(),
            layout_columns: DEFAULT_SHORTCUT_LAYOUT_COLUMNS.to_owned(),
            layout_row: DEFAULT_SHORTCUT_LAYOUT_ROW.to_owned(),
            layout_column: DEFAULT_SHORTCUT_LAYOUT_COLUMN.to_owned(),
            reset_layout: DEFAULT_SHORTCUT_RESET_LAYOUT.to_owned(),
            cycle_layout: DEFAULT_SHORTCUT_CYCLE_LAYOUT.to_owned(),
            cycle_theme: DEFAULT_SHORTCUT_CYCLE_THEME.to_owned(),
            toggle_animations: DEFAULT_SHORTCUT_TOGGLE_ANIMATIONS.to_owned(),
            toggle_toolbar: DEFAULT_SHORTCUT_TOGGLE_TOOLBAR.to_owned(),
            toggle_window_info: DEFAULT_SHORTCUT_TOGGLE_WINDOW_INFO.to_owned(),
            toggle_always_on_top: DEFAULT_SHORTCUT_TOGGLE_ALWAYS_ON_TOP.to_owned(),
            open_settings: DEFAULT_SHORTCUT_OPEN_SETTINGS.to_owned(),
            open_menu: DEFAULT_SHORTCUT_OPEN_MENU.to_owned(),
            global_activate: default_global_activate_shortcut(),
            refresh_now: DEFAULT_SHORTCUT_REFRESH.to_owned(),
            exit_app: DEFAULT_SHORTCUT_EXIT.to_owned(),
            alt_toggles_toolbar: true,
        }
    }
}

impl ShortcutBindings {
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            layout_grid: normalize_shortcut_binding(
                &self.layout_grid,
                DEFAULT_SHORTCUT_LAYOUT_GRID,
            ),
            layout_mosaic: normalize_shortcut_binding(
                &self.layout_mosaic,
                DEFAULT_SHORTCUT_LAYOUT_MOSAIC,
            ),
            layout_bento: normalize_shortcut_binding(
                &self.layout_bento,
                DEFAULT_SHORTCUT_LAYOUT_BENTO,
            ),
            layout_fibonacci: normalize_shortcut_binding(
                &self.layout_fibonacci,
                DEFAULT_SHORTCUT_LAYOUT_FIBONACCI,
            ),
            layout_columns: normalize_shortcut_binding(
                &self.layout_columns,
                DEFAULT_SHORTCUT_LAYOUT_COLUMNS,
            ),
            layout_row: normalize_shortcut_binding(&self.layout_row, DEFAULT_SHORTCUT_LAYOUT_ROW),
            layout_column: normalize_shortcut_binding(
                &self.layout_column,
                DEFAULT_SHORTCUT_LAYOUT_COLUMN,
            ),
            reset_layout: normalize_shortcut_binding(
                &self.reset_layout,
                DEFAULT_SHORTCUT_RESET_LAYOUT,
            ),
            cycle_layout: normalize_shortcut_binding(
                &self.cycle_layout,
                DEFAULT_SHORTCUT_CYCLE_LAYOUT,
            ),
            cycle_theme: normalize_shortcut_binding(
                &self.cycle_theme,
                DEFAULT_SHORTCUT_CYCLE_THEME,
            ),
            toggle_animations: normalize_shortcut_binding(
                &self.toggle_animations,
                DEFAULT_SHORTCUT_TOGGLE_ANIMATIONS,
            ),
            toggle_toolbar: normalize_shortcut_binding(
                &self.toggle_toolbar,
                DEFAULT_SHORTCUT_TOGGLE_TOOLBAR,
            ),
            toggle_window_info: normalize_shortcut_binding(
                &self.toggle_window_info,
                DEFAULT_SHORTCUT_TOGGLE_WINDOW_INFO,
            ),
            toggle_always_on_top: normalize_shortcut_binding(
                &self.toggle_always_on_top,
                DEFAULT_SHORTCUT_TOGGLE_ALWAYS_ON_TOP,
            ),
            open_settings: normalize_shortcut_binding(
                &self.open_settings,
                DEFAULT_SHORTCUT_OPEN_SETTINGS,
            ),
            open_menu: normalize_shortcut_binding(&self.open_menu, DEFAULT_SHORTCUT_OPEN_MENU),
            global_activate: normalize_global_hotkey_binding(
                self.global_activate.as_deref(),
                Some(DEFAULT_SHORTCUT_GLOBAL_ACTIVATE),
            ),
            refresh_now: normalize_shortcut_binding(&self.refresh_now, DEFAULT_SHORTCUT_REFRESH),
            exit_app: normalize_shortcut_binding(&self.exit_app, DEFAULT_SHORTCUT_EXIT),
            alt_toggles_toolbar: self.alt_toggles_toolbar,
        }
    }
}

impl WindowGrouping {
    /// User-facing label for the current grouping mode.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::None => i18n::t("group.none"),
            Self::Application => i18n::t("group.application"),
            Self::Monitor => i18n::t("group.monitor"),
            Self::WindowTitle => i18n::t("group.title"),
            Self::ClassName => i18n::t("group.class"),
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
    /// Explicit per-app override for `preserve_aspect_ratio`; `None` means
    /// inherit the current global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserve_aspect_ratio_override: Option<bool>,
    /// Whether activating this app should hide Panopticon afterwards.
    pub hide_on_select: bool,
    /// Explicit per-app override for `hide_on_select`; `None` means inherit
    /// the current global default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_on_select_override: Option<bool>,
    /// Reserved slot index for this app in the visible layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_position: Option<usize>,
    /// Manual tags used to build custom groups and filters.
    pub tags: Vec<String>,
    /// Custom accent colour hex (`RRGGBB`) assigned directly to this app
    /// (independent of any tag).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_hex: Option<String>,
    /// Thumbnail refresh strategy for this application.
    #[serde(default)]
    pub thumbnail_refresh_mode: ThumbnailRefreshMode,
    /// Thumbnail refresh interval in milliseconds (used when mode is `Interval`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_refresh_interval_ms: Option<u32>,
}

impl Default for AppRule {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            hidden: false,
            preserve_aspect_ratio: false,
            preserve_aspect_ratio_override: None,
            hide_on_select: true,
            hide_on_select_override: None,
            pinned_position: None,
            tags: Vec::new(),
            color_hex: None,
            thumbnail_refresh_mode: ThumbnailRefreshMode::default(),
            thumbnail_refresh_interval_ms: None,
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

impl DockEdge {
    /// Layout orientation forced by this dock edge.
    #[must_use]
    pub const fn forced_layout(self) -> LayoutType {
        match self {
            Self::Left | Self::Right => LayoutType::Column,
            Self::Top | Self::Bottom => LayoutType::Row,
        }
    }
}

/// User preferences persisted between application launches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools)]
pub struct AppSettings {
    /// Preferred UI language for the application.
    #[serde(default)]
    pub language: i18n::Locale,
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
    /// Preferred grouping mode for ordering windows in the layout.
    #[serde(default)]
    pub group_windows_by: WindowGrouping,
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
    /// Thickness used when the dock is vertical (`left` / `right`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock_column_thickness: Option<u32>,
    /// Thickness used when the dock is horizontal (`top` / `bottom`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock_row_thickness: Option<u32>,
    /// Selected bundled UI theme; `None` = classic Panopticon theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_id: Option<String>,
    /// Base RGB background colour (`RRGGBB`) used for the client area.
    pub background_color_hex: String,
    /// Use Windows 11 backdrop / rounded-corner chrome when available.
    pub use_system_backdrop: bool,
    /// Show the optional status bar at the bottom of the window.
    pub show_toolbar: bool,
    /// Show per-window title/app information below thumbnails.
    pub show_window_info: bool,
    /// Start the application hidden in the system tray.
    #[serde(default)]
    pub start_in_tray: bool,
    /// Start the application automatically when the user logs into Windows.
    #[serde(default)]
    pub run_at_startup: bool,
    /// Optional file path to a background image displayed behind thumbnails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_image_path: Option<String>,
    /// How the optional background image should be fitted into the dashboard.
    #[serde(default)]
    pub background_image_fit: BackgroundImageFit,
    /// Opacity percentage used when rendering the optional background image.
    #[serde(default = "default_background_image_opacity_pct")]
    pub background_image_opacity_pct: u8,
    /// Per-layout custom resize ratios (column/row proportions).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layout_customizations: BTreeMap<String, LayoutCustomization>,
    /// Prevent layout changes via keyboard shortcuts or toolbar clicks.
    #[serde(default)]
    pub locked_layout: bool,
    /// Prevent dragging separators that resize cells / columns.
    #[serde(default)]
    pub lock_cell_resize: bool,
    /// Show the application icon overlay in each thumbnail cell.
    #[serde(default = "default_true")]
    pub show_app_icons: bool,
    /// Percentage used to scale the live DWM thumbnail within each card.
    #[serde(default = "default_thumbnail_render_scale_pct")]
    pub thumbnail_render_scale_pct: u8,
    /// User-configurable dashboard keyboard shortcuts.
    #[serde(default)]
    pub shortcuts: ShortcutBindings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: i18n::Locale::English,
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
            group_windows_by: WindowGrouping::None,
            app_rules: BTreeMap::new(),
            tag_styles: BTreeMap::new(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_column_thickness: None,
            dock_row_thickness: None,
            theme_id: None,
            background_color_hex: DEFAULT_BACKGROUND_COLOR_HEX.to_owned(),
            use_system_backdrop: true,
            show_toolbar: true,
            show_window_info: true,
            start_in_tray: false,
            run_at_startup: false,
            background_image_path: None,
            background_image_fit: BackgroundImageFit::default(),
            background_image_opacity_pct: default_background_image_opacity_pct(),
            layout_customizations: BTreeMap::new(),
            locked_layout: false,
            lock_cell_resize: false,
            show_app_icons: true,
            thumbnail_render_scale_pct: default_thumbnail_render_scale_pct(),
            shortcuts: ShortcutBindings::default(),
        }
    }
}

impl AppSettings {
    /// Return the layout Panopticon should actively use for the current dock state.
    #[must_use]
    pub const fn effective_layout(&self) -> LayoutType {
        match self.dock_edge {
            Some(edge) => edge.forced_layout(),
            None => self.initial_layout,
        }
    }

    /// Return the configured dock thickness for a specific edge, if any.
    #[must_use]
    pub const fn dock_thickness_for(&self, edge: DockEdge) -> Option<u32> {
        let raw = match edge {
            DockEdge::Left | DockEdge::Right => self.dock_column_thickness,
            DockEdge::Top | DockEdge::Bottom => self.dock_row_thickness,
        };
        match raw {
            Some(0) | None => None,
            Some(value) => Some(value),
        }
    }

    /// Return the custom layout override for the given layout type, if any.
    #[must_use]
    pub fn layout_custom(&self, layout: LayoutType) -> Option<&LayoutCustomization> {
        self.layout_customizations
            .get(layout.storage_key())
            .filter(|c| !c.is_empty())
    }

    /// Store (or clear) a layout customization for the given layout type.
    pub fn set_layout_custom(&mut self, layout: LayoutType, custom: LayoutCustomization) {
        if custom.is_empty() {
            self.layout_customizations.remove(layout.storage_key());
        } else {
            self.layout_customizations
                .insert(layout.storage_key().to_owned(), custom);
        }
    }

    /// Clear all custom layout ratios for the given layout type.
    pub fn clear_layout_custom(&mut self, layout: LayoutType) {
        self.layout_customizations.remove(layout.storage_key());
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

    /// Return known saved profile names discovered on disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the profile directory exists but cannot be enumerated.
    pub fn list_profiles() -> Result<Vec<String>> {
        let profiles_dir = Self::path_for(Some("sample"))
            .parent()
            .map_or_else(|| Self::path().with_file_name("profiles"), PathBuf::from);

        if !profiles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(profiles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                if let Some(profile) = normalize_profile_name(stem) {
                    profiles.push(profile);
                }
            }
        }
        profiles.sort();
        profiles.dedup();
        Ok(profiles)
    }

    /// Return all profile labels, always including the implicit `default` one.
    ///
    /// # Errors
    ///
    /// Returns an error when the named-profile directory cannot be enumerated.
    pub fn list_profiles_with_default() -> Result<Vec<String>> {
        let mut profiles = Self::list_profiles()?;
        profiles.insert(0, "default".to_owned());
        profiles.dedup();
        Ok(profiles)
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
        let interval = REFRESH_INTERVALS_MS
            .iter()
            .copied()
            .find(|i| *i == self.refresh_interval_ms)
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_MS);
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
            .and_then(|rule| rule.preserve_aspect_ratio_override)
            .unwrap_or(self.preserve_aspect_ratio)
    }

    /// Returns the per-app custom colour hex, if one is assigned.
    #[must_use]
    pub fn app_color_hex(&self, app_id: &str) -> Option<&str> {
        self.app_rules
            .get(app_id)
            .and_then(|rule| rule.color_hex.as_deref())
    }

    /// Returns the thumbnail refresh mode for the given application.
    #[must_use]
    pub fn thumbnail_refresh_mode_for(&self, app_id: &str) -> ThumbnailRefreshMode {
        self.app_rules
            .get(app_id)
            .map_or(ThumbnailRefreshMode::Realtime, |rule| {
                rule.thumbnail_refresh_mode
            })
    }

    /// Returns the thumbnail refresh interval (ms) for the given application,
    /// defaulting to 5 000 ms when the mode is `Interval` but no custom value
    /// is set.
    #[must_use]
    pub fn thumbnail_refresh_interval_ms_for(&self, app_id: &str) -> u32 {
        self.app_rules
            .get(app_id)
            .and_then(|rule| rule.thumbnail_refresh_interval_ms)
            .unwrap_or(5_000)
    }

    /// Set the thumbnail refresh mode for a specific application.
    ///
    /// Returns `true` when the effective persisted state changed.
    pub fn set_app_thumbnail_refresh_mode(
        &mut self,
        app_id: &str,
        display_name: &str,
        mode: ThumbnailRefreshMode,
    ) -> bool {
        let rule = self.ensure_app_rule(app_id, display_name);
        let changed = rule.thumbnail_refresh_mode != mode
            || (mode == ThumbnailRefreshMode::Interval
                && rule.thumbnail_refresh_interval_ms.is_none());

        if !changed {
            return false;
        }

        rule.thumbnail_refresh_mode = mode;
        if mode == ThumbnailRefreshMode::Interval && rule.thumbnail_refresh_interval_ms.is_none() {
            rule.thumbnail_refresh_interval_ms = Some(5_000);
        }

        true
    }

    /// Returns the effective hide-on-select preference for `app_id`.
    #[must_use]
    pub fn hide_on_select_for(&self, app_id: &str) -> bool {
        if self.dock_edge.is_some() {
            return false;
        }
        self.app_rules
            .get(app_id)
            .and_then(|rule| rule.hide_on_select_override)
            .unwrap_or(self.hide_on_select)
    }

    /// Returns the reserved visible slot for `app_id`, if any.
    #[must_use]
    pub fn pinned_position_for(&self, app_id: &str) -> Option<usize> {
        self.app_rules
            .get(app_id)
            .and_then(|rule| rule.pinned_position)
    }

    /// Returns `true` when the app is pinned to a fixed visible slot.
    #[must_use]
    pub fn is_pinned_position(&self, app_id: &str) -> bool {
        self.pinned_position_for(app_id).is_some()
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

    /// Return a user-friendly label for the current grouping mode.
    #[must_use]
    pub fn grouping_label(&self) -> Option<String> {
        (self.group_windows_by != WindowGrouping::None).then(|| {
            format!(
                "{}{}",
                i18n::t("filter.grouped_by"),
                self.group_windows_by.label()
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
        let default_preserve_aspect_ratio = self.preserve_aspect_ratio;
        let next = !self.preserve_aspect_ratio_for(app_id);
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.preserve_aspect_ratio = next;
        rule.preserve_aspect_ratio_override =
            (next != default_preserve_aspect_ratio).then_some(next);
        next
    }

    /// Toggle hide-on-select for a specific application.
    pub fn toggle_app_hide_on_select(&mut self, app_id: &str, display_name: &str) -> bool {
        if self.dock_edge.is_some() {
            return false;
        }
        let default_hide_on_select = self.hide_on_select;
        let next = !self.hide_on_select_for(app_id);
        let rule = self.ensure_app_rule(app_id, display_name);
        rule.hide_on_select = next;
        rule.hide_on_select_override = (next != default_hide_on_select).then_some(next);
        next
    }

    /// Toggle a fixed visible slot for the application.
    pub fn toggle_app_pinned_position(
        &mut self,
        app_id: &str,
        display_name: &str,
        position: usize,
    ) -> bool {
        let rule = self.ensure_app_rule(app_id, display_name);
        if rule.pinned_position.is_some() {
            rule.pinned_position = None;
            false
        } else {
            rule.pinned_position = Some(position);
            true
        }
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

    /// Assign or clear a direct per-app accent colour.
    pub fn set_app_color_hex(
        &mut self,
        app_id: &str,
        display_name: &str,
        color_hex: Option<&str>,
    ) -> bool {
        let normalized = color_hex.and_then(normalize_color_hex);
        let rule = self.ensure_app_rule(app_id, display_name);
        if rule.color_hex == normalized {
            return false;
        }
        rule.color_hex = normalized;
        true
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
                    i18n::t("settings.hidden_app_fallback").to_owned()
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
            rule.preserve_aspect_ratio = rule
                .preserve_aspect_ratio_override
                .unwrap_or(self.preserve_aspect_ratio);
            rule.hide_on_select = rule.hide_on_select_override.unwrap_or(self.hide_on_select);
            rule.pinned_position = rule.pinned_position.filter(|_| !rule.hidden);
            rule.color_hex = rule.color_hex.as_deref().and_then(normalize_color_hex);
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
            language: self.language,
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
            group_windows_by: self.group_windows_by,
            app_rules,
            tag_styles,
            fixed_width: self.fixed_width,
            fixed_height: self.fixed_height,
            dock_edge: self.dock_edge,
            dock_column_thickness: normalize_positive_dimension(self.dock_column_thickness),
            dock_row_thickness: normalize_positive_dimension(self.dock_row_thickness),
            theme_id: self.theme_id.as_deref().and_then(normalize_profile_name),
            background_color_hex: normalize_color_hex(&self.background_color_hex)
                .unwrap_or_else(|| DEFAULT_BACKGROUND_COLOR_HEX.to_owned()),
            use_system_backdrop: self.use_system_backdrop,
            show_toolbar: self.show_toolbar,
            show_window_info: self.show_window_info,
            start_in_tray: self.start_in_tray,
            run_at_startup: self.run_at_startup,
            background_image_path: self.background_image_path.clone(),
            background_image_fit: self.background_image_fit,
            background_image_opacity_pct: self.background_image_opacity_pct.min(100),
            layout_customizations: self.layout_customizations.clone(),
            locked_layout: self.locked_layout,
            lock_cell_resize: self.lock_cell_resize,
            show_app_icons: self.show_app_icons,
            thumbnail_render_scale_pct: normalize_thumbnail_render_scale_pct(
                self.thumbnail_render_scale_pct,
            ),
            shortcuts: self.shortcuts.normalized(),
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
                preserve_aspect_ratio_override: None,
                hide_on_select,
                hide_on_select_override: None,
                pinned_position: None,
                tags: Vec::new(),
                color_hex: None,
                thumbnail_refresh_mode: ThumbnailRefreshMode::default(),
                thumbnail_refresh_interval_ms: None,
            });

        if rule.display_name.trim().is_empty() && !display_name.is_empty() {
            display_name.clone_into(&mut rule.display_name);
        }

        rule
    }
}

const fn default_background_image_opacity_pct() -> u8 {
    DEFAULT_BACKGROUND_IMAGE_OPACITY_PCT
}

const fn default_thumbnail_render_scale_pct() -> u8 {
    DEFAULT_THUMBNAIL_RENDER_SCALE_PCT
}

const fn normalize_thumbnail_render_scale_pct(value: u8) -> u8 {
    if value < MIN_THUMBNAIL_RENDER_SCALE_PCT {
        MIN_THUMBNAIL_RENDER_SCALE_PCT
    } else if value > 100 {
        100
    } else {
        value
    }
}

const fn normalize_positive_dimension(value: Option<u32>) -> Option<u32> {
    match value {
        Some(0) | None => None,
        Some(value) => Some(value),
    }
}

fn normalize_filter_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn is_invalid_profile_name_character(character: char) -> bool {
    character.is_control() || INVALID_PROFILE_NAME_CHARACTERS.contains(&character)
}

fn invalid_profile_name_characters(value: &str) -> Vec<char> {
    let mut invalid = Vec::new();
    for character in value.chars() {
        if is_invalid_profile_name_character(character) && !invalid.contains(&character) {
            invalid.push(character);
        }
    }
    invalid
}

fn format_profile_name_character(character: char) -> String {
    if character.is_control() {
        format!("U+{:04X}", u32::from(character))
    } else {
        character.to_string()
    }
}

/// Validate a user-supplied profile name before it is used as a settings file stem.
#[must_use]
pub fn validate_profile_name_input(value: &str) -> ProfileNameValidation {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return ProfileNameValidation::Empty;
    }

    let invalid = invalid_profile_name_characters(trimmed);
    if !invalid.is_empty() {
        let invalid_chars = invalid
            .into_iter()
            .map(format_profile_name_character)
            .collect::<Vec<_>>()
            .join(", ");
        return ProfileNameValidation::Invalid(i18n::t_fmt(
            "settings.profile_invalid_chars",
            &invalid_chars,
        ));
    }

    match normalize_profile_name(trimmed) {
        Some(profile_name) => ProfileNameValidation::Valid(profile_name),
        None => ProfileNameValidation::Empty,
    }
}

#[must_use]
pub fn normalize_profile_name(value: &str) -> Option<String> {
    let sanitized = value
        .chars()
        .filter(|character| !is_invalid_profile_name_character(*character))
        .collect::<String>();

    normalize_filter_value(&sanitized)
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

#[must_use]
pub fn normalize_shortcut_binding(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return fallback.to_owned();
    }

    let lower = trimmed.to_ascii_lowercase();
    match lower.as_str() {
        "tab" => "Tab".to_owned(),
        "esc" | "escape" => "Esc".to_owned(),
        "enter" | "return" => "Enter".to_owned(),
        "space" => "Space".to_owned(),
        _ => {
            let mut chars = trimmed.chars();
            match (chars.next(), chars.next()) {
                (Some(character), None) => character.to_ascii_uppercase().to_string(),
                _ => fallback.to_owned(),
            }
        }
    }
}

#[must_use]
pub fn normalize_global_hotkey_binding(
    value: Option<&str>,
    fallback: Option<&str>,
) -> Option<String> {
    match value.map(str::trim) {
        None | Some("") => None,
        Some(raw) => parse_global_hotkey_binding(raw)
            .map(GlobalHotkeyBinding::canonical_string)
            .or_else(|| {
                fallback
                    .and_then(parse_global_hotkey_binding)
                    .map(GlobalHotkeyBinding::canonical_string)
            }),
    }
}

#[must_use]
pub fn parse_global_hotkey_binding(value: &str) -> Option<GlobalHotkeyBinding> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut key = None;

    for token in value.split('+') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }

        match trimmed.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => {
                if ctrl {
                    return None;
                }
                ctrl = true;
            }
            "alt" => {
                if alt {
                    return None;
                }
                alt = true;
            }
            "shift" => {
                if shift {
                    return None;
                }
                shift = true;
            }
            _ => {
                if key.is_some() {
                    return None;
                }
                key = parse_global_hotkey_key(trimmed);
            }
        }
    }

    Some(GlobalHotkeyBinding {
        ctrl,
        alt,
        shift,
        key: key?,
    })
    .filter(|binding| binding.ctrl || binding.alt || binding.shift)
}

fn parse_global_hotkey_key(token: &str) -> Option<GlobalHotkeyKey> {
    let trimmed = token.trim();
    let lower = trimmed.to_ascii_lowercase();

    match lower.as_str() {
        "tab" => Some(GlobalHotkeyKey::Tab),
        "esc" | "escape" => Some(GlobalHotkeyKey::Esc),
        "enter" | "return" => Some(GlobalHotkeyKey::Enter),
        "space" => Some(GlobalHotkeyKey::Space),
        _ => {
            if let Some(function_index) = lower
                .strip_prefix('f')
                .and_then(|digits| digits.parse::<u8>().ok())
                .filter(|index| (1..=24).contains(index))
            {
                return Some(GlobalHotkeyKey::Function(function_index));
            }

            let mut chars = trimmed.chars();
            match (chars.next(), chars.next()) {
                (Some(character), None) if character.is_ascii_alphanumeric() => {
                    Some(GlobalHotkeyKey::Character(character.to_ascii_uppercase()))
                }
                _ => None,
            }
        }
    }
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
    use super::{
        normalize_global_hotkey_binding, normalize_shortcut_binding, parse_global_hotkey_binding,
        validate_profile_name_input, AppRule, AppSettings, BackgroundImageFit, GlobalHotkeyBinding,
        GlobalHotkeyKey, HiddenAppEntry, ProfileNameValidation, ShortcutBindings, TagStyle,
        ThumbnailRefreshMode,
    };
    use crate::layout::LayoutType;

    #[test]
    fn settings_roundtrip_through_toml() {
        let settings = AppSettings {
            language: crate::i18n::Locale::English,
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
            group_windows_by: super::WindowGrouping::Application,
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
            dock_column_thickness: Some(280),
            dock_row_thickness: Some(180),
            theme_id: Some("theme:demo".to_owned()),
            background_color_hex: "101820".to_owned(),
            use_system_backdrop: true,
            show_toolbar: false,
            show_window_info: false,
            start_in_tray: false,
            run_at_startup: true,
            background_image_path: None,
            background_image_fit: BackgroundImageFit::Contain,
            background_image_opacity_pct: 42,
            layout_customizations: std::collections::BTreeMap::default(),
            locked_layout: false,
            lock_cell_resize: false,
            show_app_icons: true,
            thumbnail_render_scale_pct: 82,
            shortcuts: ShortcutBindings::default(),
        };

        let encoded = toml::to_string_pretty(&settings).expect("serialize settings");
        let decoded: AppSettings = toml::from_str(&encoded).expect("deserialize settings");

        assert_eq!(decoded, settings);
    }

    #[test]
    fn invalid_refresh_interval_normalizes_to_default() {
        let settings = AppSettings {
            language: crate::i18n::Locale::English,
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
            group_windows_by: super::WindowGrouping::Monitor,
            app_rules: std::collections::BTreeMap::default(),
            tag_styles: std::collections::BTreeMap::default(),
            fixed_width: None,
            fixed_height: None,
            dock_edge: None,
            dock_column_thickness: Some(0),
            dock_row_thickness: Some(0),
            theme_id: Some("  work  ".to_owned()),
            background_color_hex: "ZZZZZZ".to_owned(),
            use_system_backdrop: false,
            show_toolbar: true,
            show_window_info: true,
            start_in_tray: false,
            run_at_startup: false,
            background_image_path: None,
            background_image_fit: BackgroundImageFit::default(),
            background_image_opacity_pct: 255,
            layout_customizations: std::collections::BTreeMap::default(),
            locked_layout: false,
            lock_cell_resize: false,
            show_app_icons: true,
            thumbnail_render_scale_pct: 10,
            shortcuts: ShortcutBindings::default(),
        };

        assert_eq!(settings.normalized().refresh_interval_ms, 2_000);
        assert_eq!(settings.normalized().background_color_hex, "181513");
        assert_eq!(settings.normalized().background_image_opacity_pct, 100);
        assert_eq!(settings.normalized().thumbnail_render_scale_pct, 50);
        assert_eq!(settings.normalized().theme_id.as_deref(), Some("work"));
    }

    #[test]
    fn effective_layout_follows_dock_orientation_without_overwriting_default_layout() {
        let settings = AppSettings {
            initial_layout: LayoutType::Bento,
            dock_edge: Some(super::DockEdge::Right),
            ..AppSettings::default()
        };

        assert_eq!(settings.initial_layout, LayoutType::Bento);
        assert_eq!(settings.effective_layout(), LayoutType::Column);

        let horizontal = AppSettings {
            dock_edge: Some(super::DockEdge::Bottom),
            ..settings
        };
        assert_eq!(horizontal.effective_layout(), LayoutType::Row);
    }

    #[test]
    fn dock_thickness_is_stored_per_orientation() {
        let settings = AppSettings {
            dock_column_thickness: Some(320),
            dock_row_thickness: Some(180),
            ..AppSettings::default()
        };

        assert_eq!(
            settings.dock_thickness_for(super::DockEdge::Left),
            Some(320)
        );
        assert_eq!(
            settings.dock_thickness_for(super::DockEdge::Right),
            Some(320)
        );
        assert_eq!(settings.dock_thickness_for(super::DockEdge::Top), Some(180));
        assert_eq!(
            settings.dock_thickness_for(super::DockEdge::Bottom),
            Some(180)
        );
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
        assert_eq!(rule.pinned_position, None);
        assert!(rule.tags.is_empty());
    }

    #[test]
    fn pinning_app_position_toggles_reserved_slot() {
        let mut settings = AppSettings::default();

        assert!(settings.toggle_app_pinned_position("app:demo", "Demo App", 3));
        assert_eq!(settings.pinned_position_for("app:demo"), Some(3));

        assert!(!settings.toggle_app_pinned_position("app:demo", "Demo App", 3));
        assert_eq!(settings.pinned_position_for("app:demo"), None);
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
    fn app_color_hex_can_be_assigned_and_cleared() {
        let mut settings = AppSettings::default();

        assert!(settings.set_app_color_hex("app:browser", "Arc Browser", Some("#5CA9FF")));
        assert_eq!(settings.app_color_hex("app:browser"), Some("5CA9FF"));

        assert!(settings.set_app_color_hex("app:browser", "Arc Browser", None));
        assert_eq!(settings.app_color_hex("app:browser"), None);
    }

    #[test]
    fn thumbnail_refresh_mode_can_be_assigned_per_app() {
        let mut settings = AppSettings::default();

        assert!(settings.set_app_thumbnail_refresh_mode(
            "app:browser",
            "Arc Browser",
            ThumbnailRefreshMode::Interval,
        ));
        assert_eq!(
            settings.thumbnail_refresh_mode_for("app:browser"),
            ThumbnailRefreshMode::Interval
        );
        assert_eq!(
            settings.thumbnail_refresh_interval_ms_for("app:browser"),
            5_000
        );

        assert!(settings.set_app_thumbnail_refresh_mode(
            "app:browser",
            "Arc Browser",
            ThumbnailRefreshMode::Frozen,
        ));
        assert_eq!(
            settings.thumbnail_refresh_mode_for("app:browser"),
            ThumbnailRefreshMode::Frozen
        );
        assert_eq!(
            settings.thumbnail_refresh_interval_ms_for("app:browser"),
            5_000
        );
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
                preserve_aspect_ratio_override: None,
                hide_on_select: true,
                hide_on_select_override: None,
                pinned_position: None,
                tags: Vec::new(),
                color_hex: None,
                thumbnail_refresh_mode: super::ThumbnailRefreshMode::default(),
                thumbnail_refresh_interval_ms: None,
            },
        );

        assert!(settings.hide_on_select_for("app:legacy"));
        settings.hide_on_select = false;

        assert!(!settings.hide_on_select_for("app:legacy"));
    }

    #[test]
    fn normalized_legacy_hide_on_select_difference_stays_inherited() {
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
                preserve_aspect_ratio_override: None,
                hide_on_select: true,
                hide_on_select_override: None,
                pinned_position: None,
                tags: Vec::new(),
                color_hex: None,
                thumbnail_refresh_mode: super::ThumbnailRefreshMode::default(),
                thumbnail_refresh_interval_ms: None,
            },
        );

        let normalized = settings.normalized();

        assert!(!normalized.hide_on_select_for("app:explicit"));
        assert_eq!(
            normalized
                .app_rules
                .get("app:explicit")
                .and_then(|rule| rule.hide_on_select_override),
            None
        );
        assert_eq!(
            normalized
                .app_rules
                .get("app:explicit")
                .map(|rule| rule.hide_on_select),
            Some(false)
        );
    }

    #[test]
    fn normalized_legacy_preserve_aspect_ratio_difference_stays_inherited() {
        let mut settings = AppSettings {
            preserve_aspect_ratio: false,
            ..AppSettings::default()
        };
        settings.app_rules.insert(
            "app:aspect".to_owned(),
            AppRule {
                display_name: "Aspect".to_owned(),
                hidden: false,
                preserve_aspect_ratio: true,
                preserve_aspect_ratio_override: None,
                hide_on_select: true,
                hide_on_select_override: None,
                pinned_position: None,
                tags: Vec::new(),
                color_hex: None,
                thumbnail_refresh_mode: super::ThumbnailRefreshMode::default(),
                thumbnail_refresh_interval_ms: None,
            },
        );

        let normalized = settings.normalized();

        assert!(!normalized.preserve_aspect_ratio_for("app:aspect"));
        assert_eq!(
            normalized
                .app_rules
                .get("app:aspect")
                .and_then(|rule| rule.preserve_aspect_ratio_override),
            None
        );
        assert_eq!(
            normalized
                .app_rules
                .get("app:aspect")
                .map(|rule| rule.preserve_aspect_ratio),
            Some(false)
        );
    }

    #[test]
    fn dock_mode_forces_effective_hide_on_select_off() {
        let mut settings = AppSettings {
            hide_on_select: true,
            dock_edge: Some(super::DockEdge::Left),
            ..AppSettings::default()
        };
        settings.app_rules.insert(
            "app:docked".to_owned(),
            AppRule {
                display_name: "Docked".to_owned(),
                hidden: false,
                preserve_aspect_ratio: false,
                preserve_aspect_ratio_override: None,
                hide_on_select: true,
                hide_on_select_override: Some(true),
                pinned_position: None,
                tags: Vec::new(),
                color_hex: None,
                thumbnail_refresh_mode: super::ThumbnailRefreshMode::default(),
                thumbnail_refresh_interval_ms: None,
            },
        );

        assert!(!settings.hide_on_select_for("app:docked"));
    }

    #[test]
    fn shortcut_bindings_normalize_known_special_keys() {
        assert_eq!(normalize_shortcut_binding("tab", "X"), "Tab");
        assert_eq!(normalize_shortcut_binding(" escape ", "X"), "Esc");
        assert_eq!(normalize_shortcut_binding("q", "X"), "Q");
        assert_eq!(normalize_shortcut_binding("", "X"), "X");
        assert_eq!(normalize_shortcut_binding("ctrl+t", "X"), "X");
    }

    #[test]
    fn global_hotkey_parser_accepts_canonical_modifier_chords() {
        assert_eq!(
            parse_global_hotkey_binding("ctrl + alt + p"),
            Some(GlobalHotkeyBinding {
                ctrl: true,
                alt: true,
                shift: false,
                key: GlobalHotkeyKey::Character('P'),
            })
        );
        assert_eq!(
            parse_global_hotkey_binding("Shift+Ctrl+Space")
                .map(GlobalHotkeyBinding::canonical_string),
            Some("Ctrl+Shift+Space".to_owned())
        );
        assert_eq!(
            parse_global_hotkey_binding("Alt+F12"),
            Some(GlobalHotkeyBinding {
                ctrl: false,
                alt: true,
                shift: false,
                key: GlobalHotkeyKey::Function(12),
            })
        );
    }

    #[test]
    fn global_hotkey_normalization_allows_disabling_and_rejects_invalid_values() {
        assert_eq!(
            normalize_global_hotkey_binding(Some(""), Some("Ctrl+Alt+P")),
            None
        );
        assert_eq!(
            normalize_global_hotkey_binding(Some("Ctrl+Alt+Shift"), Some("Ctrl+Alt+P")),
            Some("Ctrl+Alt+P".to_owned())
        );
        assert_eq!(
            normalize_global_hotkey_binding(Some("P"), Some("Ctrl+Alt+P")),
            Some("Ctrl+Alt+P".to_owned())
        );
    }

    #[test]
    fn profile_name_validation_distinguishes_blank_valid_and_invalid_input() {
        assert_eq!(
            validate_profile_name_input("  work  "),
            ProfileNameValidation::Valid("work".to_owned())
        );
        assert_eq!(
            validate_profile_name_input("   "),
            ProfileNameValidation::Empty
        );
        assert!(matches!(
            validate_profile_name_input("ops?board"),
            ProfileNameValidation::Invalid(ref reason) if reason.contains('?')
        ));
        assert!(matches!(
            validate_profile_name_input("ops\u{0007}board"),
            ProfileNameValidation::Invalid(ref reason) if reason.contains("U+0007")
        ));
    }
}
