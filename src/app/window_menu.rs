//! Native window-level context menu helpers.

use std::collections::HashSet;

use panopticon::i18n;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu,
    MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_NONOTIFY, TPM_RETURNCMD,
};

const CMD_HIDE_APP: u16 = 1;
const CMD_TOGGLE_ASPECT_RATIO: u16 = 2;
const CMD_TOGGLE_HIDE_ON_SELECT: u16 = 3;
const CMD_CREATE_TAG_FROM_APP: u16 = 4;
const CMD_TOGGLE_PIN_POSITION: u16 = 5;
const CMD_CLOSE_WINDOW: u16 = 10;
const CMD_KILL_PROCESS: u16 = 11;
const CMD_TAG_BASE: u16 = 100;
const CMD_USE_THEME_COLOR: u16 = 200;
const CMD_SET_COLOR_BASE: u16 = 210;
const NUM_COLOR_PRESETS: u16 = 6;
const CMD_SET_COLOR_END: u16 = CMD_SET_COLOR_BASE + NUM_COLOR_PRESETS;

const COLOR_PRESET_HEX: [&str; 6] = ["D29A5C", "5CA9FF", "3CCF91", "FF6B8A", "9B7BFF", "F4B740"];
const COLOR_PRESET_KEYS: [&str; 6] = [
    "color.amber",
    "color.sky",
    "color.mint",
    "color.rose",
    "color.violet",
    "color.sun",
];

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct WindowMenuState {
    pub preserve_aspect_ratio: bool,
    pub hide_on_select: bool,
    pub hide_on_select_enabled: bool,
    pub pin_position: bool,
    pub current_color_hex: Option<String>,
    pub known_tags: Vec<String>,
    pub current_tags: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowMenuAction {
    HideApp,
    ToggleAspectRatio,
    ToggleHideOnSelect,
    TogglePinPosition,
    CreateTagFromApp,
    SetColor(Option<String>),
    ToggleTag(String),
    CloseWindow,
    KillProcess,
}

#[must_use]
pub fn show_window_context_menu(
    hwnd: HWND,
    state: &WindowMenuState,
    anchor: Option<POINT>,
) -> Option<WindowMenuAction> {
    // SAFETY: the menu is created, used, and destroyed on the same UI thread.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        populate_window_menu(menu, state);

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

        let _ = DestroyMenu(menu);
        dispatch_window_menu_command(command.0 as u16, state)
    }
}

/// Build all items for the per-window context menu.
///
/// # Safety
///
/// `menu` must be a valid `HMENU` created by `CreatePopupMenu`.
#[allow(clippy::too_many_lines)]
unsafe fn populate_window_menu(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    state: &WindowMenuState,
) {
    let hide_app = encode_wide(i18n::t("menu.hide_from_layout"));
    let pin_position = encode_wide(i18n::t("menu.pin_position"));
    let preserve_aspect_ratio = encode_wide(i18n::t("menu.preserve_aspect"));
    let hide_on_select = encode_wide(i18n::t("menu.hide_on_select"));
    let create_tag = encode_wide(i18n::t("menu.create_tag"));
    let color_title = encode_wide(i18n::t("menu.cell_color"));
    let use_theme_color = encode_wide(i18n::t("menu.use_theme_color"));
    let close_window = encode_wide(i18n::t("menu.close_window"));
    let kill_process = encode_wide(i18n::t("menu.kill_process"));

    let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(state.known_tags.len());
    let mut color_labels: Vec<Vec<u16>> = Vec::with_capacity(NUM_COLOR_PRESETS as usize);

    let _ = AppendMenuW(
        menu,
        MF_STRING,
        CMD_HIDE_APP as usize,
        PCWSTR(hide_app.as_ptr()),
    );
    let _ = AppendMenuW(
        menu,
        MF_STRING | checked_flag(state.pin_position),
        CMD_TOGGLE_PIN_POSITION as usize,
        PCWSTR(pin_position.as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        MF_STRING | checked_flag(state.preserve_aspect_ratio),
        CMD_TOGGLE_ASPECT_RATIO as usize,
        PCWSTR(preserve_aspect_ratio.as_ptr()),
    );
    let _ = AppendMenuW(
        menu,
        MF_STRING
            | checked_flag(state.hide_on_select)
            | disabled_flag(!state.hide_on_select_enabled),
        CMD_TOGGLE_HIDE_ON_SELECT as usize,
        PCWSTR(hide_on_select.as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        CMD_CREATE_TAG_FROM_APP as usize,
        PCWSTR(create_tag.as_ptr()),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(color_title.as_ptr()));
    let _ = AppendMenuW(
        menu,
        MF_STRING | checked_flag(state.current_color_hex.is_none()),
        CMD_USE_THEME_COLOR as usize,
        PCWSTR(use_theme_color.as_ptr()),
    );
    for (index, (key, hex)) in COLOR_PRESET_KEYS
        .iter()
        .zip(COLOR_PRESET_HEX.iter())
        .enumerate()
    {
        let Some(command_id) = CMD_SET_COLOR_BASE.checked_add(index as u16) else {
            break;
        };
        color_labels.push(encode_wide(i18n::t(key)));
        if let Some(color_label) = color_labels.last() {
            let checked = state.current_color_hex.as_deref() == Some(*hex);
            let _ = AppendMenuW(
                menu,
                MF_STRING | checked_flag(checked),
                command_id as usize,
                PCWSTR(color_label.as_ptr()),
            );
        }
    }

    if !state.known_tags.is_empty() {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        for (index, tag) in state.known_tags.iter().enumerate() {
            let Some(command_id) = CMD_TAG_BASE.checked_add(index as u16) else {
                break;
            };
            tag_labels.push(encode_wide(tag));
            if let Some(label) = tag_labels.last() {
                let _ = AppendMenuW(
                    menu,
                    MF_STRING | checked_flag(state.current_tags.contains(tag)),
                    command_id as usize,
                    PCWSTR(label.as_ptr()),
                );
            }
        }
    }

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        CMD_CLOSE_WINDOW as usize,
        PCWSTR(close_window.as_ptr()),
    );
    let _ = AppendMenuW(
        menu,
        MF_STRING,
        CMD_KILL_PROCESS as usize,
        PCWSTR(kill_process.as_ptr()),
    );
}

/// Map a Win32 menu command ID back to a [`WindowMenuAction`].
fn dispatch_window_menu_command(id: u16, state: &WindowMenuState) -> Option<WindowMenuAction> {
    match id {
        CMD_HIDE_APP => Some(WindowMenuAction::HideApp),
        CMD_TOGGLE_PIN_POSITION => Some(WindowMenuAction::TogglePinPosition),
        CMD_TOGGLE_ASPECT_RATIO => Some(WindowMenuAction::ToggleAspectRatio),
        CMD_TOGGLE_HIDE_ON_SELECT if state.hide_on_select_enabled => {
            Some(WindowMenuAction::ToggleHideOnSelect)
        }
        CMD_CREATE_TAG_FROM_APP => Some(WindowMenuAction::CreateTagFromApp),
        CMD_USE_THEME_COLOR => Some(WindowMenuAction::SetColor(None)),
        CMD_CLOSE_WINDOW => Some(WindowMenuAction::CloseWindow),
        CMD_KILL_PROCESS => Some(WindowMenuAction::KillProcess),
        id if (CMD_SET_COLOR_BASE..CMD_SET_COLOR_END).contains(&id) => COLOR_PRESET_HEX
            .get((id - CMD_SET_COLOR_BASE) as usize)
            .map(|hex| WindowMenuAction::SetColor(Some((*hex).to_owned()))),
        id if id >= CMD_TAG_BASE => state
            .known_tags
            .get((id - CMD_TAG_BASE) as usize)
            .cloned()
            .map(WindowMenuAction::ToggleTag),
        _ => None,
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

fn encode_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
