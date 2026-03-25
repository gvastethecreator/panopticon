//! Tray icon, popup menu, and icon-generation helpers for the Panopticon UI.

use std::mem;

use anyhow::{anyhow, Result};
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
    IMAGE_FLAGS, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY,
    TPM_RETURNCMD, WM_APP, WM_GETICON, WM_LBUTTONUP, WM_RBUTTONUP,
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
const CMD_TRAY_EXIT: u16 = 7;

/// Snapshot of UI preferences needed to render the tray menu.
#[derive(Debug, Clone, Copy)]
pub struct TrayMenuState {
    /// Whether the main window is currently visible.
    pub window_visible: bool,
    /// Whether minimizing should hide to the tray.
    pub minimize_to_tray: bool,
    /// Whether closing should hide to the tray.
    pub close_to_tray: bool,
    /// Current refresh interval in milliseconds.
    pub refresh_interval_ms: u32,
}

/// Commands emitted by the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        if !added {
            return Err(anyhow!("failed to add tray icon"));
        }

        Ok(Self { hwnd, active: true })
    }

    /// Remove the tray icon if it is currently registered.
    pub fn remove(&mut self) {
        if self.active {
            let nid = notify_data(self.hwnd, HICON::default());
            // SAFETY: same window / icon ID pair used for registration.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
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
pub fn handle_tray_message(hwnd: HWND, lparam: LPARAM, state: TrayMenuState) -> Option<TrayAction> {
    match lparam.0 as u32 {
        WM_LBUTTONUP => Some(TrayAction::Toggle),
        WM_RBUTTONUP => show_tray_menu(hwnd, state),
        _ => None,
    }
}

/// Draw a window icon inside `rect`, centered and scaled.
pub fn draw_window_icon(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    hwnd: HWND,
    rect: windows::Win32::Foundation::RECT,
    size: i32,
) {
    if let Some(icon) = resolve_window_icon(hwnd) {
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
#[must_use]
pub fn resolve_window_icon(hwnd: HWND) -> Option<HICON> {
    // SAFETY: message send / class queries are read-only operations on a live
    // window handle. Returned icons are borrowed; callers must not destroy them.
    unsafe {
        for icon_type in [ICON_SMALL2, ICON_SMALL, ICON_BIG] {
            let icon = SendMessageW(hwnd, WM_GETICON, WPARAM(icon_type as usize), LPARAM(0));
            if icon.0 != 0 {
                return Some(HICON(icon.0 as *mut _));
            }
        }

        let class_small = GetClassLongPtrW(hwnd, GCLP_HICONSM);
        if class_small != 0 {
            return Some(HICON(class_small as *mut _));
        }

        let class_big = GetClassLongPtrW(hwnd, GCLP_HICON);
        if class_big != 0 {
            return Some(HICON(class_big as *mut _));
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

fn show_tray_menu(hwnd: HWND, state: TrayMenuState) -> Option<TrayAction> {
    // SAFETY: menu is created, populated, and destroyed on the same thread.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let toggle_label = if state.window_visible {
            "Hide to tray"
        } else {
            "Show Panopticon"
        };

        let toggle = encode_wide(toggle_label);
        let refresh = encode_wide("Refresh windows");
        let next_layout = encode_wide("Next layout");
        let minimize_to_tray = encode_wide("Hide on minimize");
        let close_to_tray = encode_wide("Hide on close");
        let refresh_interval = encode_wide(&format!(
            "Cycle refresh interval ({})",
            format_refresh_interval_label(state.refresh_interval_ms)
        ));
        let exit = encode_wide("Exit");

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
            CMD_TRAY_NEXT_LAYOUT as usize,
            PCWSTR(next_layout.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
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
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_EXIT as usize,
            PCWSTR(exit.as_ptr()),
        );

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
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

        let _ = DestroyMenu(menu);

        match command.0 as u16 {
            CMD_TRAY_TOGGLE => Some(TrayAction::Toggle),
            CMD_TRAY_REFRESH => Some(TrayAction::Refresh),
            CMD_TRAY_NEXT_LAYOUT => Some(TrayAction::NextLayout),
            CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY => Some(TrayAction::ToggleMinimizeToTray),
            CMD_TRAY_TOGGLE_CLOSE_TO_TRAY => Some(TrayAction::ToggleCloseToTray),
            CMD_TRAY_CYCLE_REFRESH => Some(TrayAction::CycleRefreshInterval),
            CMD_TRAY_EXIT => Some(TrayAction::Exit),
            _ => None,
        }
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

    // SAFETY: `bytes` contains a valid in-memory ICO resource with a single
    // 32-bit image; the buffer outlives the call.
    let icon = unsafe {
        CreateIconFromResourceEx(
            &bytes,
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

fn write_wide_string<const N: usize>(buffer: &mut [u16; N], text: &str) {
    let encoded = text.encode_utf16();
    for (slot, value) in buffer.iter_mut().zip(encoded.chain(std::iter::once(0))) {
        *slot = value;
    }
}

fn encode_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
