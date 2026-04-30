use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use slint::ComponentHandle;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindowVisible};

use crate::app::dock::{
    center_window_on_owner_monitor, center_window_on_point_monitor, keep_dialog_above_owner,
};
use crate::AppState;
use crate::app::native_runtime::get_hwnd;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SecondaryWindowPlacement {
    owner_hwnd: HWND,
    center_point: Option<POINT>,
}

fn visible_component_hwnd<Component>(window: &Component, exclude_hwnd: HWND) -> Option<HWND>
where
    Component: ComponentHandle,
{
    let hwnd = get_hwnd(window.window())?;
    if hwnd.0.is_null() || hwnd == exclude_hwnd {
        return None;
    }

    let is_visible = unsafe {
        // SAFETY: this is a read-only visibility query for a live top-level
        // window owned by this process.
        IsWindowVisible(hwnd).as_bool()
    };

    is_visible.then_some(hwnd)
}

fn collect_visible_panopticon_window_hwnds(state: &AppState, exclude_hwnd: HWND) -> Vec<HWND> {
    let mut owners = Vec::new();

    crate::SETTINGS_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            if let Some(hwnd) = visible_component_hwnd(window, exclude_hwnd) {
                owners.push(hwnd);
            }
        }
    });
    crate::COMMAND_PALETTE_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            if let Some(hwnd) = visible_component_hwnd(window, exclude_hwnd) {
                owners.push(hwnd);
            }
        }
    });
    crate::ABOUT_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            if let Some(hwnd) = visible_component_hwnd(window, exclude_hwnd) {
                owners.push(hwnd);
            }
        }
    });
    crate::TAG_DIALOG_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            if let Some(hwnd) = visible_component_hwnd(window, exclude_hwnd) {
                owners.push(hwnd);
            }
        }
    });

    if state.shell.hwnd != exclude_hwnd && !state.shell.hwnd.0.is_null() {
        let main_is_visible = unsafe {
            // SAFETY: this is a read-only visibility query for the live main
            // application window.
            IsWindowVisible(state.shell.hwnd).as_bool()
        };
        if main_is_visible {
            owners.push(state.shell.hwnd);
        }
    }

    owners
}

pub(crate) fn resolve_secondary_window_owner(state: &AppState, exclude_hwnd: HWND) -> HWND {
    let owners = collect_visible_panopticon_window_hwnds(state, exclude_hwnd);
    let foreground = unsafe {
        // SAFETY: foreground-window lookup is a read-only Win32 query.
        GetForegroundWindow()
    };

    owners
        .iter()
        .copied()
        .find(|candidate| *candidate == foreground)
        .or_else(|| owners.first().copied())
        .unwrap_or(state.shell.hwnd)
}

pub(super) fn secondary_window_placement(
    state: &AppState,
    center_point: Option<POINT>,
    exclude_hwnd: HWND,
) -> SecondaryWindowPlacement {
    SecondaryWindowPlacement {
        owner_hwnd: resolve_secondary_window_owner(state, exclude_hwnd),
        center_point,
    }
}

pub(crate) fn default_secondary_window_placement(
    state: &AppState,
    exclude_hwnd: HWND,
) -> SecondaryWindowPlacement {
    secondary_window_placement(state, None, exclude_hwnd)
}

pub(crate) fn apply_secondary_window_placement(
    dialog_hwnd: HWND,
    settings: &AppSettings,
    placement: SecondaryWindowPlacement,
) {
    keep_dialog_above_owner(dialog_hwnd, placement.owner_hwnd, settings);
    if settings.center_secondary_windows {
        if let Some(center_point) = placement.center_point {
            center_window_on_point_monitor(dialog_hwnd, center_point);
        } else {
            center_window_on_owner_monitor(dialog_hwnd, placement.owner_hwnd);
        }
    }
}

pub(crate) fn refresh_secondary_window_stacking(state: &Rc<RefCell<AppState>>) {
    crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let Ok(state) = state.try_borrow() else {
            return;
        };
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.shell.hwnd, &state.settings);
        }
    });

    crate::ABOUT_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let Ok(state) = state.try_borrow() else {
            return;
        };
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            let owner_hwnd = resolve_secondary_window_owner(&state, dialog_hwnd);
            keep_dialog_above_owner(dialog_hwnd, owner_hwnd, &state.settings);
        }
    });

    crate::TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let Ok(state) = state.try_borrow() else {
            return;
        };
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            let owner_hwnd = resolve_secondary_window_owner(&state, dialog_hwnd);
            keep_dialog_above_owner(dialog_hwnd, owner_hwnd, &state.settings);
        }
    });

    crate::app::command_palette::refresh_open_command_palette_window_stacking(state);
}
