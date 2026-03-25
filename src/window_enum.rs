//! Window enumeration and filtering.
//!
//! Uses the Win32 [`EnumWindows`] callback to discover all visible,
//! top-level application windows while filtering out tool windows,
//! system chrome, and other non-interactive surfaces.

use std::mem;
use std::path::Path;

use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, TRUE};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, GW_OWNER, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Metadata for a discovered top-level window.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// Native window handle.
    pub hwnd: HWND,
    /// UTF-16 decoded window title.
    pub title: String,
    /// Best-effort application identifier used for persistent rules.
    pub app_id: String,
    /// Best-effort friendly process / application name.
    pub process_name: String,
    /// Native window class name.
    pub class_name: String,
    /// Best-effort monitor name (for example `DISPLAY1`).
    pub monitor_name: String,
}

impl WindowInfo {
    /// Human-friendly label used in menus and badges.
    #[must_use]
    pub fn app_label(&self) -> String {
        if !self.process_name.is_empty() {
            self.process_name.clone()
        } else if !self.title.is_empty() {
            self.title.clone()
        } else if !self.class_name.is_empty() {
            self.class_name.clone()
        } else {
            "Application".to_owned()
        }
    }
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

    let class_name = get_class_name(hwnd);
    let process_path = get_process_image_path(hwnd);
    let process_name = process_path
        .as_deref()
        .and_then(|path| {
            Path::new(path)
                .file_stem()
                .or_else(|| Path::new(path).file_name())
        })
        .map_or_else(String::new, |segment| {
            segment.to_string_lossy().into_owned()
        });
    let monitor_name = get_monitor_name(hwnd);

    results.push(WindowInfo {
        hwnd,
        title: title.clone(),
        app_id: build_app_id(process_path.as_deref(), &class_name, &title),
        process_name,
        class_name,
        monitor_name,
    });

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

fn get_class_name(hwnd: HWND) -> String {
    let mut buffer = vec![0u16; 256];
    // SAFETY: `buffer` is writable and outlives the call.
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if len == 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..len as usize])
    }
}

fn get_process_image_path(hwnd: HWND) -> Option<String> {
    let mut process_id = 0;
    // SAFETY: valid HWND, process ID output is writable.
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(std::ptr::from_mut(&mut process_id)));
    }
    if process_id == 0 {
        return None;
    }

    // SAFETY: querying limited information on a live process is read-only.
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let mut buffer = vec![0u16; 1_024];
    let mut len = buffer.len() as u32;
    // SAFETY: process handle is valid, output buffer is writable, and `len`
    // contains the initial buffer capacity as required by the API.
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            std::ptr::from_mut(&mut len),
        )
    };
    // SAFETY: `process` was returned by `OpenProcess` in this function.
    unsafe {
        let _ = CloseHandle(process);
    }

    if result.is_err() || len == 0 {
        None
    } else {
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

fn build_app_id(process_path: Option<&str>, class_name: &str, title: &str) -> String {
    if let Some(process_path) = process_path.filter(|path| !path.trim().is_empty()) {
        return format!("exe:{}", process_path.to_ascii_lowercase());
    }

    if !class_name.trim().is_empty() {
        return format!("class:{}", class_name.to_ascii_lowercase());
    }

    format!("title:{}", title.trim().to_ascii_lowercase())
}

fn get_monitor_name(hwnd: HWND) -> String {
    // SAFETY: querying the nearest monitor for a valid top-level window is read-only.
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if monitor.0.is_null() {
        return "Current monitor".to_owned();
    }

    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    // SAFETY: `info` is fully allocated and large enough for `MONITORINFOEXW`.
    let success = unsafe { GetMonitorInfoW(monitor, &raw mut info.monitorInfo) }.as_bool();
    if !success {
        return "Current monitor".to_owned();
    }

    let raw_name = String::from_utf16_lossy(&info.szDevice);
    let trimmed = raw_name.trim_end_matches('\0').trim();
    if trimmed.is_empty() {
        "Current monitor".to_owned()
    } else {
        trimmed.trim_start_matches(r"\\.\").to_owned()
    }
}
