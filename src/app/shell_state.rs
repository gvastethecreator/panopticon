//! Native window shell state.
//!
//! Groups the fields that describe the Win32 window handle, tray icon,
//! application icons, dock/appbar mode, and last known window size.

use windows::Win32::Foundation::HWND;

use crate::app::tray::{AppIcons, TrayIcon};

/// The subset of [`AppState`] that deals with the native window shell.
pub(crate) struct ShellState {
    pub(crate) hwnd: HWND,
    pub(crate) tray_icon: Option<TrayIcon>,
    pub(crate) icons: AppIcons,
    pub(crate) is_appbar: bool,
    pub(crate) last_size: (i32, i32),
}

impl ShellState {
    pub(crate) fn new(icons: AppIcons) -> Self {
        Self {
            hwnd: HWND::default(),
            tray_icon: None,
            icons,
            is_appbar: false,
            last_size: (0, 0),
        }
    }
}
