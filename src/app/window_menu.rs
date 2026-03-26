//! Native window-level context menu helpers.

use std::collections::HashSet;

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
const CMD_CLOSE_WINDOW: u16 = 10;
const CMD_KILL_PROCESS: u16 = 11;
const CMD_TAG_BASE: u16 = 100;
const CMD_USE_THEME_COLOR: u16 = 200;
const CMD_SET_COLOR_BASE: u16 = 210;
const CMD_SET_COLOR_END: u16 = CMD_SET_COLOR_BASE + COLOR_PRESETS.len() as u16;

const COLOR_PRESETS: [(&str, &str); 6] = [
    ("Usar ámbar", "D29A5C"),
    ("Usar cielo", "5CA9FF"),
    ("Usar menta", "3CCF91"),
    ("Usar rosa", "FF6B8A"),
    ("Usar violeta", "9B7BFF"),
    ("Usar sol", "F4B740"),
];

#[derive(Debug, Clone)]
pub struct WindowMenuState {
    pub preserve_aspect_ratio: bool,
    pub hide_on_select: bool,
    pub hide_on_select_enabled: bool,
    pub current_color_hex: Option<String>,
    pub known_tags: Vec<String>,
    pub current_tags: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowMenuAction {
    HideApp,
    ToggleAspectRatio,
    ToggleHideOnSelect,
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
    // SAFETY: el menú se crea, usa y destruye en el mismo hilo de UI.
    unsafe {
        let menu = CreatePopupMenu().ok()?;

        let hide_app = encode_wide("Ocultar del layout");
        let preserve_aspect_ratio = encode_wide("Respetar relación de aspecto");
        let hide_on_select = encode_wide("Ocultar Panopticon al abrir esta app");
        let create_tag = encode_wide("Crear etiqueta personalizada…");
        let color_title = encode_wide("Color de la celda");
        let use_theme_color = encode_wide("Usar color del theme");
        let close_window = encode_wide("Cerrar ventana");
        let kill_process = encode_wide("Matar proceso");

        let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(state.known_tags.len());
        let mut color_labels: Vec<Vec<u16>> = Vec::with_capacity(COLOR_PRESETS.len());

        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_HIDE_APP as usize,
            PCWSTR(hide_app.as_ptr()),
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
        for (index, (label, hex)) in COLOR_PRESETS.iter().enumerate() {
            let Some(command_id) = CMD_SET_COLOR_BASE.checked_add(index as u16) else {
                break;
            };
            color_labels.push(encode_wide(label));
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

        match command.0 as u16 {
            CMD_HIDE_APP => Some(WindowMenuAction::HideApp),
            CMD_TOGGLE_ASPECT_RATIO => Some(WindowMenuAction::ToggleAspectRatio),
            CMD_TOGGLE_HIDE_ON_SELECT if state.hide_on_select_enabled => {
                Some(WindowMenuAction::ToggleHideOnSelect)
            }
            CMD_CREATE_TAG_FROM_APP => Some(WindowMenuAction::CreateTagFromApp),
            CMD_USE_THEME_COLOR => Some(WindowMenuAction::SetColor(None)),
            CMD_CLOSE_WINDOW => Some(WindowMenuAction::CloseWindow),
            CMD_KILL_PROCESS => Some(WindowMenuAction::KillProcess),
            id if (CMD_SET_COLOR_BASE..CMD_SET_COLOR_END).contains(&id) => COLOR_PRESETS
                .get((id - CMD_SET_COLOR_BASE) as usize)
                .map(|(_, hex)| WindowMenuAction::SetColor(Some((*hex).to_owned()))),
            id if id >= CMD_TAG_BASE => state
                .known_tags
                .get((id - CMD_TAG_BASE) as usize)
                .cloned()
                .map(WindowMenuAction::ToggleTag),
            _ => None,
        }
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
