//! Window enumeration and filtering.
//!
//! Uses the Win32 [`EnumWindows`] callback to discover all visible,
//! top-level application windows while filtering out tool windows,
//! system chrome, and other non-interactive surfaces.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::path::Path;

use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
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
    /// Owning process identifier captured during enumeration.
    pub process_id: u32,
    /// UTF-16 decoded window title.
    pub title: String,
    /// Best-effort application identifier used for persistent rules.
    pub app_id: String,
    /// Best-effort friendly process / application name.
    pub process_name: String,
    /// Full executable path when it can be queried.
    pub process_path: Option<String>,
    /// Native window class name.
    pub class_name: String,
    /// Best-effort monitor name (for example `DISPLAY1`).
    pub monitor_name: String,
}

#[derive(Debug, Clone)]
struct CachedWindowMetadata {
    class_name: String,
    process_name: String,
    process_path: Option<String>,
}

thread_local! {
    /// Stable metadata keyed by HWND and PID. The PID protects against HWND reuse.
    static WINDOW_METADATA_CACHE: RefCell<HashMap<(isize, u32), CachedWindowMetadata>> =
        RefCell::new(HashMap::new());
}

impl WindowInfo {
    /// Human-friendly label used in menus and badges.
    #[must_use]
    pub fn app_label(&self) -> &str {
        if !self.process_name.is_empty() {
            &self.process_name
        } else if !self.title.is_empty() {
            &self.title
        } else if !self.class_name.is_empty() {
            &self.class_name
        } else {
            "Application"
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

    let live_keys: HashSet<(isize, u32)> = results
        .iter()
        .map(|window| (window.hwnd.0 as isize, window.process_id))
        .collect();
    WINDOW_METADATA_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .retain(|key, _metadata| live_keys.contains(key));
    });

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

    let process_id = get_process_id(hwnd);
    if process_id == 0 || process_id == std::process::id() {
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

    // Use a stack buffer for typical titles; fall back to heap for very long ones.
    let title = if (title_len + 1) <= 512 {
        let mut buf = [0u16; 512];
        let copied = GetWindowTextW(hwnd, &mut buf[..((title_len + 1) as usize)]);
        if copied == 0 {
            return TRUE;
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    } else {
        let mut buf = vec![0u16; (title_len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        if copied == 0 {
            return TRUE;
        }
        String::from_utf16_lossy(&buf[..copied as usize])
    };

    let metadata = cached_window_metadata(hwnd, process_id);
    if !is_eligible_window(
        process_id,
        std::process::id(),
        &title,
        &metadata.class_name,
        &metadata.process_name,
    ) {
        return TRUE;
    }
    let monitor_name = get_monitor_name(hwnd);

    results.push(WindowInfo {
        hwnd,
        process_id,
        title: title.clone(),
        app_id: build_app_id(
            metadata.process_path.as_deref(),
            &metadata.class_name,
            &title,
        ),
        process_name: metadata.process_name,
        process_path: metadata.process_path,
        class_name: metadata.class_name,
        monitor_name,
    });

    TRUE
}

/// Decide whether a discovered top-level window belongs in the user-facing catalog.
///
/// The decision uses stable process/class identity where available. Titles remain a
/// fallback only for legacy shell surfaces that do not expose a useful process name.
#[must_use]
pub fn is_eligible_window(
    process_id: u32,
    current_process_id: u32,
    title: &str,
    _class_name: &str,
    process_name: &str,
) -> bool {
    const BLOCKED_TITLES: &[&str] = &[
        "Program Manager",
        "Windows Input Experience",
        "MSCTFIME UI",
        "Default IME",
    ];
    const BLOCKED_PROCESSES: &[&str] = &["TextInputHost"];

    process_id != 0
        && process_id != current_process_id
        && !BLOCKED_TITLES.contains(&title)
        && !BLOCKED_PROCESSES
            .iter()
            .any(|blocked| process_name.eq_ignore_ascii_case(blocked))
}

fn get_process_id(hwnd: HWND) -> u32 {
    let mut process_id = 0;
    // SAFETY: `process_id` is writable and the query does not mutate the source window.
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&raw mut process_id));
    }
    process_id
}

fn cached_window_metadata(hwnd: HWND, process_id: u32) -> CachedWindowMetadata {
    let key = (hwnd.0 as isize, process_id);
    WINDOW_METADATA_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            if cached.process_path.is_some() && !cached.class_name.is_empty() {
                return cached.clone();
            }
        }

        let class_name = get_class_name(hwnd);
        let process_path = get_process_image_path(process_id);
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
        let metadata = CachedWindowMetadata {
            class_name,
            process_name,
            process_path,
        };
        cache.borrow_mut().insert(key, metadata.clone());
        metadata
    })
}

fn get_class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    // SAFETY: `buffer` is writable and outlives the call.
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if len == 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..len as usize])
    }
}

fn get_process_image_path(process_id: u32) -> Option<String> {
    if process_id == 0 {
        return None;
    }

    // SAFETY: querying limited information on a live process is read-only.
    let process =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let mut buffer = [0u16; 1_024];
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

#[cfg(test)]
mod tests {
    use super::is_eligible_window;

    #[test]
    fn eligibility_rejects_current_process_and_localized_text_input_host() {
        assert!(!is_eligible_window(
            42,
            42,
            "Panopticon — Settings",
            "",
            "panopticon"
        ));
        assert!(!is_eligible_window(
            77,
            42,
            "Experiencia de entrada de Windows",
            "Windows.UI.Core.CoreWindow",
            "TextInputHost",
        ));
    }

    #[test]
    fn eligibility_preserves_regular_application_windows() {
        assert!(is_eligible_window(
            77,
            42,
            "Project — Editor",
            "Chrome_WidgetWin_1",
            "Code",
        ));
    }
}
