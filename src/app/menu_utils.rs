//! Shared utility functions for native Win32 popup menus.

use windows::Win32::UI::WindowsAndMessaging::{
    MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED, MF_UNCHECKED,
};

/// Return `MF_CHECKED` or `MF_UNCHECKED` depending on `enabled`.
pub const fn checked_flag(enabled: bool) -> MENU_ITEM_FLAGS {
    if enabled {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    }
}

/// Return `MF_GRAYED` when `disabled` is true, or an empty flag otherwise.
pub const fn disabled_flag(disabled: bool) -> MENU_ITEM_FLAGS {
    if disabled {
        MF_GRAYED
    } else {
        MENU_ITEM_FLAGS(0)
    }
}

/// Encode a `&str` as a null-terminated UTF-16 buffer for Win32 APIs.
pub fn encode_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
