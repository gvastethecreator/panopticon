//! Tray icon, popup menu, and icon-generation helpers for the Panopticon UI.

mod icons;
mod menu;
mod notify;

use panopticon::settings::{
    AppSelectionEntry, DockEdge, HiddenAppEntry, ToolbarPosition, WindowGrouping,
};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

pub use self::icons::{
    apply_window_icons, resolve_window_icon, resolve_window_icon_from_executable,
    resolve_window_icon_sized, AppIcons, INSTANCE_ACCENT_PALETTE,
};
pub use self::menu::{handle_tray_message, show_application_context_menu_at};
pub use self::notify::TrayIcon;

/// Callback message sent by the tray icon.
pub const WM_TRAYICON: u32 = WM_APP + 1;

const TRAY_ICON_ID: u32 = 1;
const CMD_TRAY_TOGGLE: u16 = 1;
const CMD_TRAY_REFRESH: u16 = 2;
const CMD_TRAY_NEXT_LAYOUT: u16 = 3;
const CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY: u16 = 4;
const CMD_TRAY_TOGGLE_CLOSE_TO_TRAY: u16 = 5;
const CMD_TRAY_CYCLE_REFRESH: u16 = 6;
const CMD_TRAY_TOGGLE_ANIMATIONS: u16 = 7;
const CMD_TRAY_TOGGLE_DEFAULT_ASPECT_RATIO: u16 = 8;
const CMD_TRAY_TOGGLE_DEFAULT_HIDE_ON_SELECT: u16 = 9;
const CMD_TRAY_TOGGLE_ALWAYS_ON_TOP: u16 = 10;
const CMD_TRAY_RESTORE_ALL_HIDDEN: u16 = 11;
const CMD_TRAY_EXIT: u16 = 12;
const CMD_TRAY_RESTORE_HIDDEN_BASE: u16 = 100;
const CMD_TRAY_MONITOR_ALL: u16 = 200;
const CMD_TRAY_MONITOR_BASE: u16 = 210;
const CMD_TRAY_TAG_FILTER_ALL: u16 = 300;
const CMD_TRAY_TAG_FILTER_BASE: u16 = 310;
const CMD_TRAY_APP_FILTER_ALL: u16 = 400;
const CMD_TRAY_APP_FILTER_BASE: u16 = 410;
const CMD_TRAY_DOCK_NONE: u16 = 500;
const CMD_TRAY_DOCK_LEFT: u16 = 501;
const CMD_TRAY_DOCK_RIGHT: u16 = 502;
const CMD_TRAY_DOCK_TOP: u16 = 503;
const CMD_TRAY_DOCK_BOTTOM: u16 = 504;
const CMD_TRAY_GROUP_NONE: u16 = 520;
const CMD_TRAY_GROUP_APPLICATION: u16 = 521;
const CMD_TRAY_GROUP_MONITOR: u16 = 522;
const CMD_TRAY_GROUP_WINDOW_TITLE: u16 = 523;
const CMD_TRAY_GROUP_CLASS_NAME: u16 = 524;
const CMD_TRAY_TOGGLE_TOOLBAR: u16 = 13;
const CMD_TRAY_OPEN_SETTINGS: u16 = 14;
const CMD_TRAY_TOGGLE_WINDOW_INFO: u16 = 15;
const CMD_TRAY_TOGGLE_APP_ICONS: u16 = 16;
const CMD_TRAY_TOGGLE_START_IN_TRAY: u16 = 17;
const CMD_TRAY_TOGGLE_LOCKED_LAYOUT: u16 = 18;
const CMD_TRAY_TOGGLE_LOCK_CELL_RESIZE: u16 = 19;
const CMD_TRAY_OPEN_ABOUT: u16 = 20;
const CMD_TRAY_TOOLBAR_TOP: u16 = 21;
const CMD_TRAY_TOOLBAR_BOTTOM: u16 = 22;
const CMD_TRAY_LOAD_DEFAULT_WORKSPACE: u16 = 30;
const CMD_TRAY_LOAD_WORKSPACE_BASE: u16 = 40;

/// Snapshot of UI preferences needed to render the tray menu.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct TrayMenuState {
    /// Whether the main window is currently visible.
    pub window_visible: bool,
    /// Whether minimizing should hide to the tray.
    pub minimize_to_tray: bool,
    /// Whether closing should hide to the tray.
    pub close_to_tray: bool,
    /// Current refresh interval in milliseconds.
    pub refresh_interval_ms: u32,
    /// Whether transitions are animated.
    pub animate_transitions: bool,
    /// Default aspect-ratio preference for app rules.
    pub preserve_aspect_ratio: bool,
    /// Default hide-on-select behaviour for app rules.
    pub hide_on_select: bool,
    /// Whether the Panopticon window is forced topmost.
    pub always_on_top: bool,
    /// Currently active monitor filter, if any.
    pub active_monitor_filter: Option<String>,
    /// Monitors available from the current desktop snapshot.
    pub available_monitors: Vec<String>,
    /// Currently active manual tag filter, if any.
    pub active_tag_filter: Option<String>,
    /// Known manual tags that can be filtered.
    pub available_tags: Vec<String>,
    /// Currently active automatic application filter, if any.
    pub active_app_filter: Option<String>,
    /// Applications that can be filtered automatically.
    pub available_apps: Vec<AppSelectionEntry>,
    /// Hidden applications that can be restored.
    pub hidden_apps: Vec<HiddenAppEntry>,
    /// Current dock edge, if any.
    pub dock_edge: Option<DockEdge>,
    /// Whether Panopticon is currently operating in dock/appbar mode.
    pub is_docked: bool,
    /// Whether the toolbar is visible.
    pub show_toolbar: bool,
    /// Whether the window footer / metadata is visible.
    pub show_window_info: bool,
    /// Whether app icons are rendered in thumbnail cells.
    pub show_app_icons: bool,
    /// Status-bar placement (top or bottom of the window).
    pub toolbar_position: ToolbarPosition,
    /// Whether Panopticon should start hidden in the tray.
    pub start_in_tray: bool,
    /// Whether layout switching / resizing is locked.
    pub locked_layout: bool,
    /// Whether separator dragging is locked.
    pub lock_cell_resize: bool,
    /// Preferred grouping mode for ordering visible windows.
    pub group_windows_by: WindowGrouping,
    /// Active workspace label (`None` = default workspace).
    pub current_workspace: Option<String>,
    /// Workspaces that can be loaded into the current instance.
    pub available_workspaces: Vec<String>,
}

/// Commands emitted by the tray icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    /// Show or hide the main window.
    Toggle,
    /// Re-enumerate windows and refresh the layout.
    Refresh,
    /// Cycle to the next layout mode.
    NextLayout,
    /// Toggle “hide on minimize”.
    ToggleMinimizeToTray,
    /// Toggle “hide on close”.
    ToggleCloseToTray,
    /// Cycle the refresh interval.
    CycleRefreshInterval,
    /// Toggle animation for layout transitions.
    ToggleAnimateTransitions,
    /// Toggle default aspect-ratio preservation for apps.
    ToggleDefaultAspectRatio,
    /// Toggle default hide-on-select behaviour for apps.
    ToggleDefaultHideOnSelect,
    /// Toggle topmost behavior for the Panopticon window.
    ToggleAlwaysOnTop,
    /// Filter windows by a specific monitor.
    SetMonitorFilter(Option<String>),
    /// Filter windows by a manual tag/group.
    SetTagFilter(Option<String>),
    /// Filter windows automatically by application.
    SetAppFilter(Option<String>),
    /// Restore one hidden application.
    RestoreHidden(String),
    /// Restore all hidden applications.
    RestoreAllHidden,
    /// Dock the window to a screen edge (or undock).
    SetDockEdge(Option<DockEdge>),
    /// Order visible windows according to the chosen grouping mode.
    SetWindowGrouping(WindowGrouping),
    /// Toggle the toolbar visibility.
    ToggleToolbar,
    /// Toggle footer / window metadata visibility.
    ToggleWindowInfo,
    /// Toggle app-icon overlays in thumbnail cells.
    ToggleAppIcons,
    /// Change status-bar placement.
    SetToolbarPosition(ToolbarPosition),
    /// Toggle start-in-tray behavior.
    ToggleStartInTray,
    /// Toggle locked layout behavior.
    ToggleLockedLayout,
    /// Toggle cell / column resize locking.
    ToggleLockCellResize,
    /// Open the dedicated settings window.
    OpenSettingsWindow,
    /// Open the About window.
    OpenAboutWindow,
    /// Load another saved workspace into the current instance.
    LoadWorkspace(Option<String>),
    /// Exit the application.
    Exit,
}
