//! Window enumeration and filtering.
//!
//! Uses the Win32 [`EnumWindows`] callback to discover all visible,
//! top-level application windows while filtering out tool windows,
//! system chrome, and other non-interactive surfaces.

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, TRUE};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    GWL_EXSTYLE, GW_OWNER, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Metadata for a discovered top-level window.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Native window handle.
    pub hwnd: HWND,
    /// UTF-16 decoded window title.
    pub title: String,
}

/// Enumerate all visible, top-level application windows.
///
/// The returned list excludes tool windows, invisible windows, windows
/// without a title, and known system surfaces (e.g. *Program Manager*).
#[must_use]
pub fn enumerate_windows() -> Vec<WindowInfo> {
    let mut results: Vec<WindowInfo> = Vec::new();

    // SAFETY: `EnumWindows` invokes `enum_callback` synchronously on the
    // calling thread.  The `LPARAM` carries a valid pointer to `results`
    // which outlives the callback.
    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(std::ptr::from_mut(&mut results) as isize),
        );
    }

    results
}

/// Per-window callback invoked by [`EnumWindows`].
///
/// # Safety
///
/// `lparam` must be a valid pointer to a `Vec<WindowInfo>` that outlives
/// the callback invocation.
unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` was set in `enumerate_windows` to point at a live Vec.
    let results = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };

    // Must be visible.
    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }

    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

    // Skip tool windows (unless they also have WS_EX_APPWINDOW).
    if (ex_style & WS_EX_TOOLWINDOW.0 != 0) && (ex_style & WS_EX_APPWINDOW.0 == 0) {
        return TRUE;
    }

    // Skip non-activatable windows that are not app windows.
    if (ex_style & WS_EX_NOACTIVATE.0 != 0) && (ex_style & WS_EX_APPWINDOW.0 == 0) {
        return TRUE;
    }

    // Skip owned windows (unless WS_EX_APPWINDOW).
    if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
        if owner != HWND::default() && (ex_style & WS_EX_APPWINDOW.0 == 0) {
            return TRUE;
        }
    }

    // Must have a non-empty title.
    let title_len = GetWindowTextLengthW(hwnd);
    if title_len == 0 {
        return TRUE;
    }

    let mut buffer = vec![0u16; (title_len + 1) as usize];
    let copied = GetWindowTextW(hwnd, &mut buffer);
    if copied == 0 {
        return TRUE;
    }

    let title = String::from_utf16_lossy(&buffer[..copied as usize]);

    // Filter out known system windows.
    if is_system_window(&title) {
        return TRUE;
    }

    results.push(WindowInfo { hwnd, title });

    TRUE
}

/// Returns `true` for window titles that belong to known system surfaces.
fn is_system_window(title: &str) -> bool {
    const BLOCKED: &[&str] = &[
        "Program Manager",
        "Windows Input Experience",
        "MSCTFIME UI",
        "Default IME",
    ];
    BLOCKED.contains(&title)
}
