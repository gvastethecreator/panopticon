use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use panopticon::ui_option_ops::{suggested_tag_name, tag_color_hex, tag_color_index};
use panopticon::window_enum::WindowInfo;
use slint::{CloseRequestResponse, ComponentHandle, SharedString};
use windows::Win32::Foundation::POINT;

use crate::app::dock::{apply_window_appearance, keep_dialog_above_owner};
use crate::app::theme_ui::{apply_about_window_theme_snapshot, apply_tag_dialog_theme_snapshot};
use crate::app::tray::apply_window_icons;
use crate::{AboutWindow, AppState, MainWindow, TagDialogWindow};
use crate::app::native_runtime::get_hwnd;
use crate::app::runtime_support::{refresh_ui, update_settings};
use crate::app::ui_translations::populate_tr_global;
use crate::app::window_sync::refresh_windows;

use super::placement::{
    apply_secondary_window_placement, default_secondary_window_placement,
    resolve_secondary_window_owner, secondary_window_placement,
};

pub(crate) fn open_about_window(state: &Rc<RefCell<AppState>>) {
    open_about_window_with_anchor(state, None);
}

pub(crate) fn open_about_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    center_point: Option<POINT>,
) {
    let already_open = crate::ABOUT_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = get_hwnd(existing.window()) {
                let state = state.borrow();
                let placement = secondary_window_placement(&state, center_point, hwnd);
                apply_window_icons(hwnd, &state.shell.icons);
                apply_secondary_window_placement(hwnd, &state.settings, placement);
            }
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let about_window = match AboutWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create about window");
            return;
        }
    };
    populate_tr_global(&about_window);

    {
        let state = state.borrow();
        sync_about_window_from_state(&about_window, &state);
    }

    about_window.on_open_github(|| {
        open_external_url("https://github.com/gvastethecreator");
    });

    about_window.on_open_x(|| {
        open_external_url("https://x.com/gvastethecreator");
    });

    about_window.on_closed(close_about_window);

    about_window.window().on_close_requested(|| {
        close_about_window();
        CloseRequestResponse::HideWindow
    });

    if let Err(error) = about_window.show() {
        tracing::error!(%error, "failed to show about window");
        return;
    }

    if let Some(about_hwnd) = get_hwnd(about_window.window()) {
        let state = state.borrow();
        let placement = secondary_window_placement(&state, center_point, about_hwnd);
        apply_window_icons(about_hwnd, &state.shell.icons);
        apply_window_appearance(about_hwnd, &state.settings);
        apply_about_window_theme_snapshot(&about_window, &state.theme.current_theme);
        apply_secondary_window_placement(about_hwnd, &state.settings, placement);
    }

    crate::ABOUT_WIN.with(|handle| *handle.borrow_mut() = Some(about_window));
}

pub(crate) fn refresh_open_about_window(state: &Rc<RefCell<AppState>>) {
    crate::ABOUT_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let Ok(state) = state.try_borrow() else {
            tracing::debug!("skipping about window refresh while app state is busy");
            return;
        };
        sync_about_window_from_state(window, &state);
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            let owner_hwnd = resolve_secondary_window_owner(&state, dialog_hwnd);
            keep_dialog_above_owner(dialog_hwnd, owner_hwnd, &state.settings);
        }
    });
}

pub(crate) fn open_create_tag_dialog(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
) {
    let already_open = crate::TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(dialog_hwnd) = get_hwnd(existing.window()) {
                let state = state.borrow();
                let placement = default_secondary_window_placement(&state, dialog_hwnd);
                apply_window_icons(dialog_hwnd, &state.shell.icons);
                apply_secondary_window_placement(dialog_hwnd, &state.settings, placement);
            }
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let suggested_name = suggested_tag_name(info.app_label());
    let suggested_color = state.borrow().settings.tag_color_hex(&suggested_name);

    let dialog = match TagDialogWindow::new() {
        Ok(dialog) => dialog,
        Err(error) => {
            tracing::error!(%error, app_id = %info.app_id, "failed to create tag dialog");
            return;
        }
    };
    populate_tr_global(&dialog);

    dialog.set_app_label(SharedString::from(info.app_label()));
    dialog.set_tag_name(SharedString::from(suggested_name));
    dialog.set_color_index(tag_color_index(&suggested_color));
    {
        let state = state.borrow();
        apply_tag_dialog_theme_snapshot(&dialog, &state.theme.current_theme);
    }

    dialog.on_create({
        let state = state.clone();
        let weak = weak.clone();
        let app_id = info.app_id.clone();
        let display_name = info.app_label().to_owned();
        move || {
            crate::TAG_DIALOG_WIN.with(|dialog_cell| {
                let guard = dialog_cell.borrow();
                let Some(dialog) = guard.as_ref() else {
                    return;
                };
                let tag_name = dialog.get_tag_name().to_string();
                let color_hex = tag_color_hex(dialog.get_color_index());
                drop(guard);

                apply_tag_creation(&state, &weak, &app_id, &display_name, &tag_name, &color_hex);
                close_tag_dialog_window();
            });
        }
    });

    dialog.on_closed(close_tag_dialog_window);

    dialog.window().on_close_requested(|| {
        close_tag_dialog_window();
        CloseRequestResponse::HideWindow
    });

    if let Err(error) = dialog.show() {
        tracing::error!(%error, app_id = %info.app_id, "failed to show tag dialog");
        return;
    }

    if let Some(dialog_hwnd) = get_hwnd(dialog.window()) {
        let state = state.borrow();
        let placement = default_secondary_window_placement(&state, dialog_hwnd);
        apply_window_icons(dialog_hwnd, &state.shell.icons);
        apply_window_appearance(dialog_hwnd, &state.settings);
        apply_tag_dialog_theme_snapshot(&dialog, &state.theme.current_theme);
        apply_secondary_window_placement(dialog_hwnd, &state.settings, placement);
    }

    crate::TAG_DIALOG_WIN.with(|dialog_cell| *dialog_cell.borrow_mut() = Some(dialog));
}

pub(super) fn refresh_open_tag_dialog_window(state: &Rc<RefCell<AppState>>) {
    crate::TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        populate_tr_global(window);
        let state = state.borrow();
        apply_tag_dialog_theme_snapshot(window, &state.theme.current_theme);
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            let owner_hwnd = resolve_secondary_window_owner(&state, dialog_hwnd);
            keep_dialog_above_owner(dialog_hwnd, owner_hwnd, &state.settings);
        }
    });
}

fn sync_about_window_from_state(window: &AboutWindow, state: &AppState) {
    populate_tr_global(window);
    window.set_version_text(SharedString::from(state.app_version.clone()));
    let (show_update_badge, latest_version_text) = match &state.update_status {
        crate::UpdateStatus::Available { latest_version, .. } => (true, latest_version.clone()),
        _ => (false, String::new()),
    };
    window.set_show_update_badge(show_update_badge);
    window.set_latest_version_text(SharedString::from(latest_version_text));
    apply_about_window_theme_snapshot(window, &state.theme.current_theme);
}

fn apply_tag_creation(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    app_id: &str,
    display_name: &str,
    tag_name: &str,
    color_hex: &str,
) {
    update_settings(state, |settings| {
        let _ = settings.assign_tag_with_color(app_id, display_name, tag_name, color_hex);
    });
    let _ = refresh_windows(state);
    refresh_ui(state, weak);
}

fn close_tag_dialog_window() {
    let taken = crate::TAG_DIALOG_WIN.with(|dialog| dialog.borrow_mut().take());
    if let Some(dialog) = taken {
        dialog.hide().ok();
    }
}

fn close_about_window() {
    let taken = crate::ABOUT_WIN.with(|handle| handle.borrow_mut().take());
    if let Some(window) = taken {
        window.hide().ok();
    }
}

fn open_external_url(url: &str) {
    if let Err(error) = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(url)
        .spawn()
    {
        tracing::warn!(%error, %url, "failed to open external url");
    }
}
