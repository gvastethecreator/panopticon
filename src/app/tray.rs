//! Tray icon, popup menu, and icon-generation helpers for the Panopticon UI.

use std::mem;

use anyhow::{anyhow, Result};
use panopticon::settings::{AppSelectionEntry, DockEdge, HiddenAppEntry, WindowGrouping};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HINSTANCE, HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconFromResourceEx, CreatePopupMenu, DestroyIcon, DestroyMenu, DrawIconEx,
    GetClassLongPtrW, GetCursorPos, LoadIconW, SendMessageW, SetForegroundWindow, TrackPopupMenu,
    DI_NORMAL, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG, ICON_SMALL, ICON_SMALL2, IDI_APPLICATION,
    IMAGE_FLAGS, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, WM_APP, WM_GETICON, WM_LBUTTONUP,
    WM_RBUTTONUP,
};

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
    /// Whether Panopticon should start hidden in the tray.
    pub start_in_tray: bool,
    /// Whether layout switching / resizing is locked.
    pub locked_layout: bool,
    /// Whether separator dragging is locked.
    pub lock_cell_resize: bool,
    /// Preferred grouping mode for ordering visible windows.
    pub group_windows_by: WindowGrouping,
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
    /// Toggle start-in-tray behavior.
    ToggleStartInTray,
    /// Toggle locked layout behavior.
    ToggleLockedLayout,
    /// Toggle cell / column resize locking.
    ToggleLockCellResize,
    /// Open the dedicated settings window.
    OpenSettingsWindow,
    /// Exit the application.
    Exit,
}

/// Application icon handles used by the Win32 window class and the tray icon.
pub struct AppIcons {
    /// Large icon for the main window.
    pub large: HICON,
    /// Small icon for the taskbar / tray.
    pub small: HICON,
    owns_handles: bool,
}

impl AppIcons {
    /// Create custom generated icons for Panopticon.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated icon resource cannot be converted
    /// into a live [`HICON`].
    pub fn new() -> Result<Self> {
        Ok(Self {
            large: create_generated_icon(48)?,
            small: create_generated_icon(16)?,
            owns_handles: true,
        })
    }

    /// Create icons with a custom accent colour for differentiated instances.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated icon resource cannot be converted.
    pub fn with_accent(r: u8, g: u8, b: u8) -> Result<Self> {
        Ok(Self {
            large: create_colored_icon(48, [r, g, b])?,
            small: create_colored_icon(16, [r, g, b])?,
            owns_handles: true,
        })
    }

    /// Fallback to the system application icon when custom icon generation
    /// fails.
    #[must_use]
    pub fn fallback_system() -> Self {
        // SAFETY: shared stock icon managed by the OS; must not be destroyed.
        let icon = unsafe { LoadIconW(HINSTANCE::default(), IDI_APPLICATION).unwrap_or_default() };
        Self {
            large: icon,
            small: icon,
            owns_handles: false,
        }
    }
}

/// Predefined accent colours for instance differentiation.
pub const INSTANCE_ACCENT_PALETTE: &[[u8; 3]] = &[
    [0xD2, 0x9A, 0x5C], // Amber (default)
    [0x5C, 0xA9, 0xFF], // Sky
    [0x3C, 0xCF, 0x91], // Mint
    [0xFF, 0x6B, 0x8A], // Rose
    [0x9B, 0x7B, 0xFF], // Violet
    [0xF4, 0xB7, 0x40], // Sun
    [0xFF, 0x8C, 0x42], // Tangerine
    [0x42, 0xD4, 0xD4], // Teal
];

impl Drop for AppIcons {
    fn drop(&mut self) {
        if self.owns_handles {
            if !self.large.0.is_null() {
                // SAFETY: owned icon created by `CreateIconFromResourceEx`.
                unsafe {
                    let _ = DestroyIcon(self.large);
                }
            }
            if !self.small.0.is_null() && self.small != self.large {
                // SAFETY: owned icon created by `CreateIconFromResourceEx`.
                unsafe {
                    let _ = DestroyIcon(self.small);
                }
            }
        }
    }
}

/// Runtime tray icon registration.
pub struct TrayIcon {
    hwnd: HWND,
    active: bool,
}

impl TrayIcon {
    /// Register the tray icon for `hwnd`.
    ///
    /// # Errors
    ///
    /// Returns an error if `Shell_NotifyIconW(NIM_ADD, …)` fails.
    pub fn add(hwnd: HWND, icon: HICON) -> Result<Self> {
        let nid = notify_data(hwnd, icon);

        // SAFETY: valid window handle, fixed icon ID, and fully initialised
        // NOTIFYICONDATAW structure.
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &raw const nid).as_bool() };
        if !added {
            return Err(anyhow!("failed to add tray icon"));
        }

        Ok(Self { hwnd, active: true })
    }

    /// Re-register the tray icon (e.g., after an Explorer restart).
    pub fn readd(&mut self, icon: HICON) {
        let nid = notify_data(self.hwnd, icon);
        // SAFETY: valid window handle, fixed icon ID.
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &raw const nid).as_bool() };
        if added {
            self.active = true;
        } else {
            tracing::warn!("Failed to re-add tray icon after Explorer restart");
        }
    }

    /// Remove the tray icon if it is currently registered.
    pub fn remove(&mut self) {
        if self.active {
            let nid = notify_data(self.hwnd, HICON::default());
            // SAFETY: same window / icon ID pair used for registration.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &raw const nid);
            }
            self.active = false;
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Convert a tray callback into a higher-level action.
#[must_use]
pub fn handle_tray_message(
    hwnd: HWND,
    lparam: LPARAM,
    state: &TrayMenuState,
) -> Option<TrayAction> {
    match lparam.0 as u32 {
        WM_LBUTTONUP => Some(TrayAction::Toggle),
        WM_RBUTTONUP => show_application_context_menu(hwnd, state),
        _ => None,
    }
}

#[must_use]
pub fn show_application_context_menu(hwnd: HWND, state: &TrayMenuState) -> Option<TrayAction> {
    show_application_context_menu_at(hwnd, state, None)
}

/// Draw a window icon inside `rect`, centered and scaled.
#[allow(dead_code)]
pub fn draw_window_icon(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    hwnd: HWND,
    rect: windows::Win32::Foundation::RECT,
    size: i32,
) {
    if let Some(icon) = resolve_window_icon_sized(hwnd, size >= 32) {
        let x = rect.left + ((rect.right - rect.left - size) / 2);
        let y = rect.top + ((rect.bottom - rect.top - size) / 2);

        // SAFETY: `hdc` is valid for the current paint pass; `icon` is a live
        // window-owned icon handle borrowed from the source window / class.
        unsafe {
            let _ = DrawIconEx(hdc, x, y, icon, size, size, 0, None, DI_NORMAL);
        }
    }
}

/// Resolve the best available icon for a source window.
#[allow(dead_code)]
#[must_use]
pub fn resolve_window_icon(hwnd: HWND) -> Option<HICON> {
    resolve_window_icon_sized(hwnd, false)
}

/// Resolve the best available icon for a source window, preferring either the
/// large or the small handle depending on the intended render size.
#[must_use]
pub fn resolve_window_icon_sized(hwnd: HWND, prefer_large: bool) -> Option<HICON> {
    // SAFETY: message send / class queries are read-only operations on a live
    // window handle. Returned icons are borrowed; callers must not destroy them.
    unsafe {
        let icon_order = if prefer_large {
            [ICON_BIG, ICON_SMALL2, ICON_SMALL]
        } else {
            [ICON_SMALL2, ICON_SMALL, ICON_BIG]
        };

        for icon_type in icon_order {
            let icon = SendMessageW(hwnd, WM_GETICON, WPARAM(icon_type as usize), LPARAM(0));
            if icon.0 != 0 {
                return Some(HICON(icon.0 as *mut _));
            }
        }

        let class_order = if prefer_large {
            [GCLP_HICON, GCLP_HICONSM]
        } else {
            [GCLP_HICONSM, GCLP_HICON]
        };

        for class_index in class_order {
            let class_icon = GetClassLongPtrW(hwnd, class_index);
            if class_icon != 0 {
                return Some(HICON(class_icon as *mut _));
            }
        }
    }

    None
}

fn notify_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW {
        cbSize: mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: icon,
        ..Default::default()
    };
    write_wide_string(&mut nid.szTip, "Panopticon — Live window overview");
    nid
}

#[allow(clippy::too_many_lines)]
pub fn show_application_context_menu_at(
    hwnd: HWND,
    state: &TrayMenuState,
    anchor: Option<POINT>,
) -> Option<TrayAction> {
    // SAFETY: menu is created, populated, and destroyed on the same thread.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let toggle_label = if state.window_visible {
            "Hide to tray"
        } else {
            "Show Panopticon"
        };

        let visibility_title = encode_wide("Visibility");
        let layout_title = encode_wide("Layout");
        let display_title = encode_wide("Display");
        let behaviour_title = encode_wide("Behaviour");
        let filters_title = encode_wide("Filters");
        let toggle = encode_wide(toggle_label);
        let refresh = encode_wide("Refresh windows");
        let open_settings = encode_wide("Open settings window");
        let next_layout = encode_wide("Next layout");
        let lock_layout = encode_wide("Lock layout switching");
        let lock_cell_resize = encode_wide("Lock cell / column resizing");
        let dock_title = encode_wide("Dock position");
        let grouping_title = encode_wide("Group windows by");
        let minimize_to_tray = encode_wide("Hide on minimize");
        let close_to_tray = encode_wide("Hide on close");
        let refresh_interval = encode_wide(&format!(
            "Cycle refresh interval ({})",
            format_refresh_interval_label(state.refresh_interval_ms)
        ));
        let animations = encode_wide("Animate transitions");
        let default_aspect_ratio = encode_wide("Default: preserve aspect ratio");
        let default_hide_on_select = encode_wide("Default: hide after activation");
        let always_on_top = encode_wide("Keep Panopticon on top");
        let show_toolbar = encode_wide("Show header");
        let show_window_info = encode_wide("Show window info");
        let show_app_icons = encode_wide("Show app icons in cells");
        let start_in_tray = encode_wide("Start hidden in tray");
        let dock_none = encode_wide("Floating (no dock)");
        let dock_left = encode_wide("Left");
        let dock_right = encode_wide("Right");
        let dock_top = encode_wide("Top");
        let dock_bottom = encode_wide("Bottom");
        let group_none = encode_wide("No grouping");
        let group_application = encode_wide("Application");
        let group_monitor = encode_wide("Monitor");
        let group_window_title = encode_wide("Window title");
        let group_class_name = encode_wide("Window class");
        let restore_hidden_title = encode_wide("Restore hidden apps");
        let restore_all_hidden = encode_wide("Restore all hidden apps");
        let monitor_filter_title = encode_wide("Filter by monitor");
        let monitor_all = encode_wide("All monitors");
        let tag_filter_title = encode_wide("Filter by tag");
        let tag_filter_all = encode_wide("All tags");
        let app_filter_title = encode_wide("Filter by application");
        let app_filter_all = encode_wide("All applications");
        let exit = encode_wide("Exit");

        let mut hidden_labels: Vec<Vec<u16>> = Vec::with_capacity(state.hidden_apps.len());
        let mut monitor_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_monitors.len());
        let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_tags.len());
        let mut app_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_apps.len());
        let mut restore_actions: Vec<(u16, String)> = Vec::with_capacity(state.hidden_apps.len());
        let mut monitor_actions: Vec<(u16, String)> =
            Vec::with_capacity(state.available_monitors.len());
        let mut tag_actions: Vec<(u16, String)> = Vec::with_capacity(state.available_tags.len());
        let mut app_actions: Vec<(u16, String)> = Vec::with_capacity(state.available_apps.len());

        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(visibility_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_TOGGLE as usize,
            PCWSTR(toggle.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_REFRESH as usize,
            PCWSTR(refresh.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_OPEN_SETTINGS as usize,
            PCWSTR(open_settings.as_ptr()),
        );

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(layout_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_NEXT_LAYOUT as usize,
            PCWSTR(next_layout.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.locked_layout),
            CMD_TRAY_TOGGLE_LOCKED_LAYOUT as usize,
            PCWSTR(lock_layout.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.lock_cell_resize),
            CMD_TRAY_TOGGLE_LOCK_CELL_RESIZE as usize,
            PCWSTR(lock_cell_resize.as_ptr()),
        );

        // ── Dock submenu ───
        {
            let dock_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge.is_none()),
                CMD_TRAY_DOCK_NONE as usize,
                PCWSTR(dock_none.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Left)),
                CMD_TRAY_DOCK_LEFT as usize,
                PCWSTR(dock_left.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Right)),
                CMD_TRAY_DOCK_RIGHT as usize,
                PCWSTR(dock_right.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Top)),
                CMD_TRAY_DOCK_TOP as usize,
                PCWSTR(dock_top.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Bottom)),
                CMD_TRAY_DOCK_BOTTOM as usize,
                PCWSTR(dock_bottom.as_ptr()),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                dock_menu.0 as usize,
                PCWSTR(dock_title.as_ptr()),
            );
        }

        {
            let grouping_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::None),
                CMD_TRAY_GROUP_NONE as usize,
                PCWSTR(group_none.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::Application),
                CMD_TRAY_GROUP_APPLICATION as usize,
                PCWSTR(group_application.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::Monitor),
                CMD_TRAY_GROUP_MONITOR as usize,
                PCWSTR(group_monitor.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::WindowTitle),
                CMD_TRAY_GROUP_WINDOW_TITLE as usize,
                PCWSTR(group_window_title.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::ClassName),
                CMD_TRAY_GROUP_CLASS_NAME as usize,
                PCWSTR(group_class_name.as_ptr()),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                grouping_menu.0 as usize,
                PCWSTR(grouping_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(display_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_toolbar),
            CMD_TRAY_TOGGLE_TOOLBAR as usize,
            PCWSTR(show_toolbar.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_window_info),
            CMD_TRAY_TOGGLE_WINDOW_INFO as usize,
            PCWSTR(show_window_info.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_app_icons),
            CMD_TRAY_TOGGLE_APP_ICONS as usize,
            PCWSTR(show_app_icons.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.always_on_top),
            CMD_TRAY_TOGGLE_ALWAYS_ON_TOP as usize,
            PCWSTR(always_on_top.as_ptr()),
        );

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(behaviour_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING
                | if state.minimize_to_tray {
                    windows::Win32::UI::WindowsAndMessaging::MF_CHECKED
                } else {
                    windows::Win32::UI::WindowsAndMessaging::MF_UNCHECKED
                },
            CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY as usize,
            PCWSTR(minimize_to_tray.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING
                | if state.close_to_tray {
                    windows::Win32::UI::WindowsAndMessaging::MF_CHECKED
                } else {
                    windows::Win32::UI::WindowsAndMessaging::MF_UNCHECKED
                },
            CMD_TRAY_TOGGLE_CLOSE_TO_TRAY as usize,
            PCWSTR(close_to_tray.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_CYCLE_REFRESH as usize,
            PCWSTR(refresh_interval.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.animate_transitions),
            CMD_TRAY_TOGGLE_ANIMATIONS as usize,
            PCWSTR(animations.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.preserve_aspect_ratio),
            CMD_TRAY_TOGGLE_DEFAULT_ASPECT_RATIO as usize,
            PCWSTR(default_aspect_ratio.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.hide_on_select) | disabled_flag(state.is_docked),
            CMD_TRAY_TOGGLE_DEFAULT_HIDE_ON_SELECT as usize,
            PCWSTR(default_hide_on_select.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.start_in_tray),
            CMD_TRAY_TOGGLE_START_IN_TRAY as usize,
            PCWSTR(start_in_tray.as_ptr()),
        );

        let has_filter_section = !state.available_monitors.is_empty()
            || !state.available_tags.is_empty()
            || !state.available_apps.is_empty();
        if has_filter_section {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                PCWSTR(filters_title.as_ptr()),
            );
        }

        if !state.available_monitors.is_empty() {
            let monitor_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                monitor_menu,
                MF_STRING | checked_flag(state.active_monitor_filter.is_none()),
                CMD_TRAY_MONITOR_ALL as usize,
                PCWSTR(monitor_all.as_ptr()),
            );

            for (index, monitor) in state.available_monitors.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_MONITOR_BASE.checked_add(index as u16) else {
                    break;
                };

                monitor_labels.push(encode_wide(monitor));
                if let Some(label) = monitor_labels.last() {
                    let _ = AppendMenuW(
                        monitor_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_monitor_filter.as_deref() == Some(monitor.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                monitor_actions.push((command_id, monitor.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                monitor_menu.0 as usize,
                PCWSTR(monitor_filter_title.as_ptr()),
            );
        }

        if !state.available_tags.is_empty() {
            let tag_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                tag_menu,
                MF_STRING | checked_flag(state.active_tag_filter.is_none()),
                CMD_TRAY_TAG_FILTER_ALL as usize,
                PCWSTR(tag_filter_all.as_ptr()),
            );

            for (index, tag) in state.available_tags.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_TAG_FILTER_BASE.checked_add(index as u16) else {
                    break;
                };

                tag_labels.push(encode_wide(tag));
                if let Some(label) = tag_labels.last() {
                    let _ = AppendMenuW(
                        tag_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_tag_filter.as_deref() == Some(tag.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                tag_actions.push((command_id, tag.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                tag_menu.0 as usize,
                PCWSTR(tag_filter_title.as_ptr()),
            );
        }

        if !state.available_apps.is_empty() {
            let app_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                app_menu,
                MF_STRING | checked_flag(state.active_app_filter.is_none()),
                CMD_TRAY_APP_FILTER_ALL as usize,
                PCWSTR(app_filter_all.as_ptr()),
            );

            for (index, app) in state.available_apps.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_APP_FILTER_BASE.checked_add(index as u16) else {
                    break;
                };

                app_labels.push(encode_wide(&app.label));
                if let Some(label) = app_labels.last() {
                    let _ = AppendMenuW(
                        app_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_app_filter.as_deref() == Some(app.app_id.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                app_actions.push((command_id, app.app_id.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                app_menu.0 as usize,
                PCWSTR(app_filter_title.as_ptr()),
            );
        }

        if !state.hidden_apps.is_empty() {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let hidden_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                hidden_menu,
                MF_STRING,
                CMD_TRAY_RESTORE_ALL_HIDDEN as usize,
                PCWSTR(restore_all_hidden.as_ptr()),
            );
            let _ = AppendMenuW(hidden_menu, MF_SEPARATOR, 0, PCWSTR::null());

            for (index, hidden_app) in state.hidden_apps.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_RESTORE_HIDDEN_BASE.checked_add(index as u16)
                else {
                    break;
                };
                hidden_labels.push(encode_wide(&hidden_app.label));
                if let Some(label) = hidden_labels.last() {
                    let _ = AppendMenuW(
                        hidden_menu,
                        MF_STRING,
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                restore_actions.push((command_id, hidden_app.app_id.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                hidden_menu.0 as usize,
                PCWSTR(restore_hidden_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_EXIT as usize,
            PCWSTR(exit.as_ptr()),
        );

        let mut cursor = anchor.unwrap_or_default();
        if anchor.is_none() {
            let _ = GetCursorPos(&raw mut cursor);
        }
        let _ = SetForegroundWindow(hwnd);

        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            cursor.x,
            cursor.y,
            0,
            hwnd,
            None,
        );

        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::WM_NULL,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );

        let _ = DestroyMenu(menu);

        match command.0 as u16 {
            CMD_TRAY_TOGGLE => Some(TrayAction::Toggle),
            CMD_TRAY_REFRESH => Some(TrayAction::Refresh),
            CMD_TRAY_NEXT_LAYOUT => Some(TrayAction::NextLayout),
            CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY => Some(TrayAction::ToggleMinimizeToTray),
            CMD_TRAY_TOGGLE_CLOSE_TO_TRAY => Some(TrayAction::ToggleCloseToTray),
            CMD_TRAY_CYCLE_REFRESH => Some(TrayAction::CycleRefreshInterval),
            CMD_TRAY_TOGGLE_ANIMATIONS => Some(TrayAction::ToggleAnimateTransitions),
            CMD_TRAY_TOGGLE_DEFAULT_ASPECT_RATIO => Some(TrayAction::ToggleDefaultAspectRatio),
            CMD_TRAY_TOGGLE_DEFAULT_HIDE_ON_SELECT => Some(TrayAction::ToggleDefaultHideOnSelect),
            CMD_TRAY_TOGGLE_ALWAYS_ON_TOP => Some(TrayAction::ToggleAlwaysOnTop),
            CMD_TRAY_TOGGLE_TOOLBAR => Some(TrayAction::ToggleToolbar),
            CMD_TRAY_TOGGLE_WINDOW_INFO => Some(TrayAction::ToggleWindowInfo),
            CMD_TRAY_TOGGLE_APP_ICONS => Some(TrayAction::ToggleAppIcons),
            CMD_TRAY_TOGGLE_START_IN_TRAY => Some(TrayAction::ToggleStartInTray),
            CMD_TRAY_TOGGLE_LOCKED_LAYOUT => Some(TrayAction::ToggleLockedLayout),
            CMD_TRAY_TOGGLE_LOCK_CELL_RESIZE => Some(TrayAction::ToggleLockCellResize),
            CMD_TRAY_OPEN_SETTINGS => Some(TrayAction::OpenSettingsWindow),
            CMD_TRAY_DOCK_NONE => Some(TrayAction::SetDockEdge(None)),
            CMD_TRAY_DOCK_LEFT => Some(TrayAction::SetDockEdge(Some(DockEdge::Left))),
            CMD_TRAY_DOCK_RIGHT => Some(TrayAction::SetDockEdge(Some(DockEdge::Right))),
            CMD_TRAY_DOCK_TOP => Some(TrayAction::SetDockEdge(Some(DockEdge::Top))),
            CMD_TRAY_DOCK_BOTTOM => Some(TrayAction::SetDockEdge(Some(DockEdge::Bottom))),
            CMD_TRAY_GROUP_NONE => Some(TrayAction::SetWindowGrouping(WindowGrouping::None)),
            CMD_TRAY_GROUP_APPLICATION => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::Application))
            }
            CMD_TRAY_GROUP_MONITOR => Some(TrayAction::SetWindowGrouping(WindowGrouping::Monitor)),
            CMD_TRAY_GROUP_WINDOW_TITLE => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::WindowTitle))
            }
            CMD_TRAY_GROUP_CLASS_NAME => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::ClassName))
            }
            CMD_TRAY_MONITOR_ALL => Some(TrayAction::SetMonitorFilter(None)),
            CMD_TRAY_TAG_FILTER_ALL => Some(TrayAction::SetTagFilter(None)),
            CMD_TRAY_APP_FILTER_ALL => Some(TrayAction::SetAppFilter(None)),
            CMD_TRAY_RESTORE_ALL_HIDDEN => Some(TrayAction::RestoreAllHidden),
            CMD_TRAY_EXIT => Some(TrayAction::Exit),
            dynamic => monitor_actions
                .into_iter()
                .find_map(|(command_id, monitor)| {
                    (dynamic == command_id).then_some(TrayAction::SetMonitorFilter(Some(monitor)))
                })
                .or_else(|| {
                    tag_actions.into_iter().find_map(|(command_id, tag)| {
                        (dynamic == command_id).then_some(TrayAction::SetTagFilter(Some(tag)))
                    })
                })
                .or_else(|| {
                    app_actions.into_iter().find_map(|(command_id, app_id)| {
                        (dynamic == command_id).then_some(TrayAction::SetAppFilter(Some(app_id)))
                    })
                })
                .or_else(|| {
                    restore_actions
                        .into_iter()
                        .find_map(|(command_id, app_id)| {
                            (dynamic == command_id).then_some(TrayAction::RestoreHidden(app_id))
                        })
                }),
        }
    }
}

const fn checked_flag(enabled: bool) -> windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS {
    if enabled {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    }
}

const fn disabled_flag(disabled: bool) -> windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS {
    if disabled {
        MF_GRAYED
    } else {
        windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_FLAGS(0)
    }
}

fn format_refresh_interval_label(interval_ms: u32) -> String {
    if interval_ms.is_multiple_of(1_000) {
        format!("{}s", interval_ms / 1_000)
    } else {
        format!("{:.1}s", f64::from(interval_ms) / 1_000.0)
    }
}

fn create_generated_icon(size: u8) -> Result<HICON> {
    let bytes = build_icon_resource(size);
    let image_data = &bytes[22..];

    // SAFETY: `bytes` contains a valid in-memory ICO resource with a single
    // 32-bit image; the buffer outlives the call.
    let icon = unsafe {
        CreateIconFromResourceEx(
            image_data,
            BOOL(1),
            0x0003_0000,
            i32::from(size),
            i32::from(size),
            IMAGE_FLAGS(0),
        )
    };

    if icon.is_err() {
        Err(anyhow!("failed to create generated icon handle"))
    } else {
        Ok(icon?)
    }
}

fn create_colored_icon(size: u8, accent_rgb: [u8; 3]) -> Result<HICON> {
    let bytes = build_colored_icon_resource(size, accent_rgb);
    let image_data = &bytes[22..];

    // SAFETY: same as `create_generated_icon`.
    let icon = unsafe {
        CreateIconFromResourceEx(
            image_data,
            BOOL(1),
            0x0003_0000,
            i32::from(size),
            i32::from(size),
            IMAGE_FLAGS(0),
        )
    };

    if icon.is_err() {
        Err(anyhow!("failed to create coloured icon handle"))
    } else {
        Ok(icon?)
    }
}

fn build_icon_resource(size: u8) -> Vec<u8> {
    let size_usize = usize::from(size);
    let mask_stride = size_usize.div_ceil(32) * 4;
    let image_size = 40 + (size_usize * size_usize * 4) + (mask_stride * size_usize);
    let image_offset = 6 + 16;

    let mut bytes = Vec::with_capacity(image_offset + image_size);

    // ICONDIR
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());

    // ICONDIRENTRY
    bytes.push(size);
    bytes.push(size);
    bytes.push(0);
    bytes.push(0);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&(image_size as u32).to_le_bytes());
    bytes.extend_from_slice(&(image_offset as u32).to_le_bytes());

    // BITMAPINFOHEADER
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size)).to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size) * 2).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&((size_usize * size_usize * 4) as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    // XOR bitmap (BGRA, bottom-up)
    for y in (0..size_usize).rev() {
        for x in 0..size_usize {
            let pixel = icon_pixel(x as f32, y as f32, size as f32);
            bytes.extend_from_slice(&pixel);
        }
    }

    // AND mask (all zero = fully visible; alpha controls transparency)
    bytes.resize(image_offset + image_size, 0);

    bytes
}

fn icon_pixel(x: f32, y: f32, size: f32) -> [u8; 4] {
    let center = (size - 1.0) / 2.0;
    let dx = x - center;
    let dy = y - center;
    let distance = (dx * dx + dy * dy).sqrt();

    let outer = size * 0.47;
    let ring = size * 0.41;
    let eye_x = dx / (size * 0.36);
    let eye_y = dy / (size * 0.22);
    let eye = eye_x * eye_x + eye_y * eye_y;
    let iris = distance <= size * 0.14;
    let pupil = distance <= size * 0.07;
    let highlight = (x - size * 0.62).powi(2) + (y - size * 0.36).powi(2) <= (size * 0.05).powi(2);

    let transparent = [0, 0, 0, 0];
    let dark = [0x19, 0x1A, 0x20, 0xFF];
    let slate = [0x2D, 0x31, 0x3B, 0xFF];
    let accent = [0xC8, 0x89, 0x56, 0xFF];
    let accent_ring = [0xE2, 0xA0, 0x61, 0xFF];
    let near_white = [0xF4, 0xF6, 0xFA, 0xFF];
    let pupil_color = [0x08, 0x0A, 0x0E, 0xFF];

    if distance > outer {
        transparent
    } else if distance >= ring {
        accent_ring
    } else if highlight {
        near_white
    } else if pupil {
        pupil_color
    } else if iris {
        accent
    } else if eye <= 1.0 {
        slate
    } else {
        dark
    }
}

fn build_colored_icon_resource(size: u8, accent_rgb: [u8; 3]) -> Vec<u8> {
    let size_usize = usize::from(size);
    let mask_stride = size_usize.div_ceil(32) * 4;
    let image_size = 40 + (size_usize * size_usize * 4) + (mask_stride * size_usize);
    let image_offset = 6 + 16;

    let mut bytes = Vec::with_capacity(image_offset + image_size);

    // ICONDIR
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());

    // ICONDIRENTRY
    bytes.push(size);
    bytes.push(size);
    bytes.push(0);
    bytes.push(0);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&(image_size as u32).to_le_bytes());
    bytes.extend_from_slice(&(image_offset as u32).to_le_bytes());

    // BITMAPINFOHEADER
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size)).to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size) * 2).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&((size_usize * size_usize * 4) as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    // Lighten accent for the ring
    let ring_rgb = [
        accent_rgb[0].saturating_add(0x10),
        accent_rgb[1].saturating_add(0x06),
        accent_rgb[2].saturating_add(0x0B),
    ];

    // XOR bitmap (BGRA, bottom-up)
    for y in (0..size_usize).rev() {
        for x in 0..size_usize {
            let pixel = icon_pixel_colored(x as f32, y as f32, size as f32, accent_rgb, ring_rgb);
            bytes.extend_from_slice(&pixel);
        }
    }

    // AND mask
    bytes.resize(image_offset + image_size, 0);

    bytes
}

/// Extract the executable icon from a file path.
///
/// The returned handle is owned by the caller and must be destroyed with
/// [`DestroyIcon`] when no longer needed.
#[must_use]
pub fn resolve_window_icon_from_executable(path: &str, prefer_large: bool) -> Option<HICON> {
    use windows::Win32::UI::Shell::ExtractIconExW;

    let wide = encode_wide(path);
    let mut large = [HICON::default(); 1];
    let mut small = [HICON::default(); 1];

    // SAFETY: `wide` is a valid, nul-terminated UTF-16 path and both icon
    // buffers outlive the call.
    let extracted = unsafe {
        ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(large.as_mut_ptr()),
            Some(small.as_mut_ptr()),
            1,
        )
    };
    if extracted == 0 {
        return None;
    }

    let preferred = if prefer_large { large[0] } else { small[0] };
    let secondary = if prefer_large { small[0] } else { large[0] };

    if !preferred.0.is_null() {
        if !secondary.0.is_null() && secondary != preferred {
            // SAFETY: `secondary` is an extracted icon handle owned by us.
            unsafe {
                let _ = DestroyIcon(secondary);
            }
        }
        Some(preferred)
    } else if !secondary.0.is_null() {
        Some(secondary)
    } else {
        None
    }
}

fn icon_pixel_colored(
    x: f32,
    y: f32,
    size: f32,
    accent_rgb: [u8; 3],
    ring_rgb: [u8; 3],
) -> [u8; 4] {
    let center = (size - 1.0) / 2.0;
    let dx = x - center;
    let dy = y - center;
    let distance = (dx * dx + dy * dy).sqrt();

    let outer = size * 0.47;
    let ring = size * 0.41;
    let eye_x = dx / (size * 0.36);
    let eye_y = dy / (size * 0.22);
    let eye = eye_x * eye_x + eye_y * eye_y;
    let iris = distance <= size * 0.14;
    let pupil = distance <= size * 0.07;
    let highlight = (x - size * 0.62).powi(2) + (y - size * 0.36).powi(2) <= (size * 0.05).powi(2);

    let transparent = [0, 0, 0, 0];
    let dark = [0x19, 0x1A, 0x20, 0xFF];
    let slate = [0x2D, 0x31, 0x3B, 0xFF];
    // BGRA order
    let accent = [accent_rgb[2], accent_rgb[1], accent_rgb[0], 0xFF];
    let accent_ring_color = [ring_rgb[2], ring_rgb[1], ring_rgb[0], 0xFF];
    let near_white = [0xF4, 0xF6, 0xFA, 0xFF];
    let pupil_color = [0x08, 0x0A, 0x0E, 0xFF];

    if distance > outer {
        transparent
    } else if distance >= ring {
        accent_ring_color
    } else if highlight {
        near_white
    } else if pupil {
        pupil_color
    } else if iris {
        accent
    } else if eye <= 1.0 {
        slate
    } else {
        dark
    }
}

fn write_wide_string<const N: usize>(buffer: &mut [u16; N], text: &str) {
    let encoded = text.encode_utf16();
    for (slot, value) in buffer.iter_mut().zip(encoded.chain(std::iter::once(0))) {
        *slot = value;
    }
}

fn encode_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
