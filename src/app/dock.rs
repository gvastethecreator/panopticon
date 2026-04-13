//! App-bar (dock) registration, positioning, and system-menu helpers.

use std::mem;

use panopticon::settings::DockEdge;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::{AppState, WM_APPBAR_CALLBACK};

// ───────────────────────── Dock lifecycle ─────────────────────────

pub(crate) fn apply_dock_mode(state: &mut AppState) {
    let hwnd = state.hwnd;
    // SAFETY: hwnd is our live main window; switching style to borderless popup
    // on the UI thread before registering as an appbar.
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_VISIBLE).0 as isize);
    }
    sync_dock_system_menu(hwnd, true);
    if register_appbar(hwnd) {
        state.is_appbar = true;
        reposition_appbar(state);
    }
}

pub(crate) fn register_appbar(hwnd: HWND) -> bool {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: WM_APPBAR_CALLBACK,
        ..Default::default()
    };
    // SAFETY: abd is stack-allocated with correct cbSize; hwnd is our window.
    unsafe { SHAppBarMessage(ABM_NEW, &raw mut abd) != 0 }
}

pub(crate) fn unregister_appbar(hwnd: HWND) {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    // SAFETY: abd is stack-allocated with correct cbSize; releasing the appbar
    // registration for our window.
    unsafe {
        let _ = SHAppBarMessage(ABM_REMOVE, &raw mut abd);
    }
}

#[allow(clippy::similar_names)]
pub(crate) fn reposition_appbar(state: &mut AppState) {
    let Some(edge) = state.settings.dock_edge else {
        return;
    };
    let hwnd = state.hwnd;
    let monitor_rect = get_monitor_rect(hwnd);
    let abe = dock_edge_to_abe(edge);
    let thickness = match edge {
        DockEdge::Left | DockEdge::Right => state.settings.fixed_width.unwrap_or(300) as i32,
        DockEdge::Top | DockEdge::Bottom => state.settings.fixed_height.unwrap_or(200) as i32,
    };

    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uEdge: abe,
        rc: monitor_rect,
        ..Default::default()
    };

    // SAFETY: abd is stack-allocated with correct cbSize and valid hwnd.
    // ABM_QUERYPOS, ABM_SETPOS and SetWindowPos are called sequentially on
    // the UI thread to negotiate and apply the appbar position.
    unsafe {
        let _ = SHAppBarMessage(ABM_QUERYPOS, &raw mut abd);
        match edge {
            DockEdge::Left => abd.rc.right = abd.rc.left + thickness,
            DockEdge::Right => abd.rc.left = abd.rc.right - thickness,
            DockEdge::Top => abd.rc.bottom = abd.rc.top + thickness,
            DockEdge::Bottom => abd.rc.top = abd.rc.bottom - thickness,
        }
        let _ = SHAppBarMessage(ABM_SETPOS, &raw mut abd);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            abd.rc.left,
            abd.rc.top,
            abd.rc.right - abd.rc.left,
            abd.rc.bottom - abd.rc.top,
            SWP_NOACTIVATE,
        );
    }
}

pub(crate) fn restore_floating_style(hwnd: HWND) {
    // SAFETY: hwnd is our live window; restoring the normal overlapped style
    // and refreshing the frame on the UI thread.
    unsafe {
        let _ = SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            (WS_OVERLAPPEDWINDOW | WS_VISIBLE).0 as isize,
        );
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
    sync_dock_system_menu(hwnd, false);
}

// ───────────────────────── Window appearance helpers ─────────────────────────

pub(crate) fn apply_window_appearance(hwnd: HWND, settings: &panopticon::settings::AppSettings) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    let dark_mode: i32 = 1;
    let corner = DWMWCP_ROUND;
    let backdrop = if settings.use_system_backdrop {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };
    // SAFETY: hwnd is our live window; all values are stack-allocated with
    // correct sizes. DwmSetWindowAttribute is a read-only DWM configuration call.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::from_ref(&dark_mode).cast::<c_void>(),
            mem::size_of_val(&dark_mode) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corner).cast::<c_void>(),
            mem::size_of_val(&corner) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::from_ref(&backdrop).cast::<c_void>(),
            mem::size_of_val(&backdrop) as u32,
        );
    }
}

pub(crate) fn apply_topmost_mode(hwnd: HWND, always_on_top: bool) {
    // SAFETY: hwnd is our live window; toggling the topmost z-order flag.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            if always_on_top {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_NOTOPMOST)
            },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

pub(crate) fn sync_dock_system_menu(hwnd: HWND, docked: bool) {
    // SAFETY: system menu belongs to the live top-level window and the command IDs are standard SC_* items.
    unsafe {
        let menu = GetSystemMenu(hwnd, false);
        if menu.0.is_null() {
            return;
        }

        let flags = MF_BYCOMMAND | if docked { MF_GRAYED } else { MF_ENABLED };
        for command in [SC_MOVE, SC_SIZE, SC_MINIMIZE, SC_MAXIMIZE, SC_CLOSE] {
            let _ = EnableMenuItem(menu, command, flags);
        }
    }
}

pub(crate) const fn is_blocked_dock_syscommand(command: usize) -> bool {
    let masked = command & 0xFFF0;
    masked == SC_MOVE as usize
        || masked == SC_SIZE as usize
        || masked == SC_MINIMIZE as usize
        || masked == SC_MAXIMIZE as usize
        || masked == SC_CLOSE as usize
}

pub(crate) fn docked_mode_active() -> bool {
    crate::UI_STATE.with(|state| {
        state.borrow().as_ref().is_some_and(|rc| {
            rc.try_borrow()
                .map(|state| state.settings.dock_edge.is_some())
                .unwrap_or(false)
        })
    })
}

// ───────────────────────── Geometry ─────────────────────────

pub(crate) fn get_monitor_rect(hwnd: HWND) -> RECT {
    // SAFETY: hwnd is a valid window; MonitorFromWindow and GetMonitorInfoW
    // are read-only queries with stack-allocated output structs.
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &raw mut info).as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        }
    }
}

pub(crate) fn keep_dialog_above_owner(
    dialog_hwnd: HWND,
    owner_hwnd: HWND,
    settings: &panopticon::settings::AppSettings,
) {
    if dialog_hwnd.0.is_null() {
        return;
    }

    let owner_is_visible = !owner_hwnd.0.is_null()
        && unsafe {
            // SAFETY: owner_hwnd belongs to our process when present; querying
            // visibility is a read-only Win32 call.
            IsWindowVisible(owner_hwnd).as_bool()
        };

    // SAFETY: dialog_hwnd belongs to a live window created by this process.
    // The owner handle is only attached when it is both valid and currently
    // visible so hidden tray-host windows do not hide the settings dialog.
    unsafe {
        let owner_raw = if owner_is_visible {
            owner_hwnd.0 as isize
        } else {
            0
        };
        let _ = SetWindowLongPtrW(dialog_hwnd, GWLP_HWNDPARENT, owner_raw);
        let _ = SetWindowPos(
            dialog_hwnd,
            if settings.always_on_top || settings.dock_edge.is_some() {
                Some(HWND_TOPMOST)
            } else {
                Some(HWND_TOP)
            },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

/// Centers the given window on the same monitor as its visible owner when
/// available, or on the current/primary monitor otherwise.
pub(crate) fn center_window_on_owner_monitor(dialog_hwnd: HWND, owner_hwnd: HWND) {
    let anchor_hwnd = if !owner_hwnd.0.is_null()
        && unsafe {
            // SAFETY: owner_hwnd belongs to our process when present; querying
            // visibility is a read-only Win32 call.
            IsWindowVisible(owner_hwnd).as_bool()
        } {
        owner_hwnd
    } else {
        dialog_hwnd
    };

    center_window_on_anchor_monitor(dialog_hwnd, anchor_hwnd);
}

fn center_window_on_anchor_monitor(hwnd: HWND, anchor_hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    // SAFETY: hwnd is a live window created by this process.
    unsafe {
        let monitor = MonitorFromWindow(anchor_hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &raw mut mi).as_bool() {
            return;
        }
        let mut rc = RECT::default();
        if GetWindowRect(hwnd, &raw mut rc).is_err() {
            return;
        }
        let win_w = rc.right - rc.left;
        let win_h = rc.bottom - rc.top;
        if win_w <= 0 || win_h <= 0 {
            return;
        }
        let work = mi.rcWork;
        let cx = work.left + (work.right - work.left - win_w) / 2;
        let cy = work.top + (work.bottom - work.top - win_h) / 2;
        let _ = SetWindowPos(
            hwnd,
            None,
            cx,
            cy,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

const fn dock_edge_to_abe(edge: DockEdge) -> u32 {
    match edge {
        DockEdge::Left => ABE_LEFT,
        DockEdge::Right => ABE_RIGHT,
        DockEdge::Top => ABE_TOP,
        DockEdge::Bottom => ABE_BOTTOM,
    }
}
