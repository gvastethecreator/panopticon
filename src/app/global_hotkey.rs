//! Global hotkey registration for activating the main dashboard window.

use panopticon::settings::{
    parse_global_hotkey_binding, AppSettings, GlobalHotkeyBinding, GlobalHotkeyKey,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, VK_ESCAPE, VK_F1, VK_RETURN, VK_SPACE, VK_TAB,
};

pub(crate) const GLOBAL_ACTIVATE_HOTKEY_ID: i32 = 0x5041;

pub(crate) fn sync_activate_hotkey(hwnd: HWND, settings: &AppSettings) {
    if hwnd.0.is_null() {
        return;
    }

    unregister_activate_hotkey(hwnd);

    let Some(binding) = settings
        .shortcuts
        .global_activate
        .as_deref()
        .and_then(parse_global_hotkey_binding)
    else {
        tracing::info!("global activate hotkey disabled");
        return;
    };

    let modifiers = hotkey_modifiers(binding);
    let virtual_key = hotkey_virtual_key(binding);

    // SAFETY: `hwnd` is the live main window owned by this process; the hotkey
    // ID is process-local and we register at most one activation binding.
    let registration = unsafe {
        RegisterHotKey(
            Some(hwnd),
            GLOBAL_ACTIVATE_HOTKEY_ID,
            modifiers,
            virtual_key,
        )
    };

    if let Err(error) = registration {
        tracing::warn!(
            %error,
            hotkey = %binding.canonical_string(),
            "failed to register global activate hotkey"
        );
    } else {
        tracing::info!(hotkey = %binding.canonical_string(), "global activate hotkey registered");
    }
}

pub(crate) fn unregister_activate_hotkey(hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }

    // SAFETY: unregistering a process-local hotkey ID for our own window is
    // idempotent and safe even when the binding is currently absent.
    unsafe {
        let _ = UnregisterHotKey(Some(hwnd), GLOBAL_ACTIVATE_HOTKEY_ID);
    }
}

#[must_use]
pub(crate) const fn is_activate_hotkey(id: usize) -> bool {
    id as i32 == GLOBAL_ACTIVATE_HOTKEY_ID
}

fn hotkey_modifiers(binding: GlobalHotkeyBinding) -> HOT_KEY_MODIFIERS {
    let mut modifiers = MOD_NOREPEAT;
    if binding.ctrl {
        modifiers |= MOD_CONTROL;
    }
    if binding.alt {
        modifiers |= MOD_ALT;
    }
    if binding.shift {
        modifiers |= MOD_SHIFT;
    }
    modifiers
}

fn hotkey_virtual_key(binding: GlobalHotkeyBinding) -> u32 {
    match binding.key {
        GlobalHotkeyKey::Character(character) => u32::from(character),
        GlobalHotkeyKey::Function(index) => u32::from(VK_F1.0) + u32::from(index - 1),
        GlobalHotkeyKey::Tab => u32::from(VK_TAB.0),
        GlobalHotkeyKey::Esc => u32::from(VK_ESCAPE.0),
        GlobalHotkeyKey::Enter => u32::from(VK_RETURN.0),
        GlobalHotkeyKey::Space => u32::from(VK_SPACE.0),
    }
}
