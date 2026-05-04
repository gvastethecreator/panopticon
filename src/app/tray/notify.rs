use std::mem;

use anyhow::{anyhow, Result};
use panopticon::i18n;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::HICON;

use super::{TRAY_ICON_ID, WM_TRAYICON};

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

    /// Refresh the tray tooltip and icon payload in-place.
    pub fn refresh(&mut self, icon: HICON) {
        let nid = notify_data(self.hwnd, icon);
        // SAFETY: same window / icon ID pair used for registration.
        let updated = unsafe { Shell_NotifyIconW(NIM_MODIFY, &raw const nid).as_bool() };
        if !updated {
            tracing::warn!("failed to refresh tray icon metadata");
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
    write_wide_string(&mut nid.szTip, i18n::t("tray.tooltip"));
    nid
}

fn write_wide_string<const N: usize>(buffer: &mut [u16; N], text: &str) {
    let encoded = text.encode_utf16();
    for (slot, value) in buffer.iter_mut().zip(encoded.chain(std::iter::once(0))) {
        *slot = value;
    }
}
