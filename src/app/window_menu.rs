//! Win32 context menu for per-window actions.

use std::collections::HashSet;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
};

const CMD_WINDOW_HIDE_APP: u16 = 1;
const CMD_WINDOW_TOGGLE_ASPECT_RATIO: u16 = 2;
const CMD_WINDOW_TOGGLE_HIDE_ON_SELECT: u16 = 3;
const CMD_WINDOW_CREATE_TAG_FROM_APP: u16 = 4;
const CMD_WINDOW_TAG_BASE: u16 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowMenuAction {
    HideApp,
    ToggleAspectRatio,
    ToggleHideOnSelect,
    CreateTagFromApp,
    ToggleTag(String),
}

pub fn show_window_context_menu(
    parent_hwnd: HWND,
    preserve_aspect_ratio: bool,
    hide_on_select: bool,
    known_tags: &[String],
    current_tags: &HashSet<String>,
) -> Option<WindowMenuAction> {
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let hide_label = wide("Hide from layout");
        let aspect_label = wide("Respect aspect ratio");
        let hide_after_label = wide("Hide Panopticon after opening this app");
        let create_tag_label = wide("Create custom tag…");
        let tags_title = wide("Assign existing tags");

        let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(known_tags.len());
        let mut tag_actions: Vec<(u16, String)> = Vec::with_capacity(known_tags.len());

        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_WINDOW_HIDE_APP as usize,
            PCWSTR(hide_label.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(preserve_aspect_ratio),
            CMD_WINDOW_TOGGLE_ASPECT_RATIO as usize,
            PCWSTR(aspect_label.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(hide_on_select),
            CMD_WINDOW_TOGGLE_HIDE_ON_SELECT as usize,
            PCWSTR(hide_after_label.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_WINDOW_CREATE_TAG_FROM_APP as usize,
            PCWSTR(create_tag_label.as_ptr()),
        );

        if !known_tags.is_empty() {
            let tags_menu = CreatePopupMenu().ok()?;
            for (i, tag) in known_tags.iter().enumerate() {
                let Some(cmd) = CMD_WINDOW_TAG_BASE.checked_add(i as u16) else {
                    break;
                };
                tag_labels.push(wide(tag));
                if let Some(label) = tag_labels.last() {
                    let _ = AppendMenuW(
                        tags_menu,
                        MF_STRING | checked_flag(current_tags.contains(tag)),
                        cmd as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                tag_actions.push((cmd, tag.clone()));
            }
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                tags_menu.0 as usize,
                PCWSTR(tags_title.as_ptr()),
            );
        }

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&raw mut cursor);
        let _ = SetForegroundWindow(parent_hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            cursor.x,
            cursor.y,
            0,
            parent_hwnd,
            None,
        );
        let _ = DestroyMenu(menu);

        match cmd.0 as u16 {
            CMD_WINDOW_HIDE_APP => Some(WindowMenuAction::HideApp),
            CMD_WINDOW_TOGGLE_ASPECT_RATIO => Some(WindowMenuAction::ToggleAspectRatio),
            CMD_WINDOW_TOGGLE_HIDE_ON_SELECT => Some(WindowMenuAction::ToggleHideOnSelect),
            CMD_WINDOW_CREATE_TAG_FROM_APP => Some(WindowMenuAction::CreateTagFromApp),
            dynamic => tag_actions
                .into_iter()
                .find_map(|(command, tag)| {
                    (dynamic == command).then_some(WindowMenuAction::ToggleTag(tag))
                }),
        }
    }
}

fn checked_flag(enabled: bool) -> MENU_ITEM_FLAGS {
    if enabled {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
