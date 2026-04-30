//! Thumbnail-specific UI interactions and contextual window actions.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::settings::ToolbarPosition;
use panopticon::window_enum::WindowInfo;
use slint::ComponentHandle;
use windows::Win32::Foundation::HWND;

use super::icon::populate_cached_icon;
use super::window_menu::{show_window_context_menu, WindowMenuAction, WindowMenuState};
use super::dwm::{release_all_thumbnails, release_thumbnail};
use super::model_sync::recompute_and_update_ui;
use super::runtime_support::{
    logical_to_screen_point, refresh_ui, schedule_deferred_refresh, update_settings,
};
use super::window_sync::refresh_windows;
use crate::{AppState, MainWindow};

pub(crate) fn handle_thumbnail_click(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    index: usize,
) {
    let mut state_guard = state.borrow_mut();
    let Some(managed_window) = state_guard.window_collection.windows.get(index) else {
        return;
    };
    let info = managed_window.info.clone();
    let hide_on_select = state_guard.settings.hide_on_select_for(&info.app_id);
    state_guard.window_collection.active_hwnd = Some(info.hwnd);
    drop(state_guard);

    tracing::info!(title = %info.title, app_id = %info.app_id, "activating window");
    activate_window(info.hwnd);

    if hide_on_select {
        if let Some(window) = weak.upgrade() {
            release_all_thumbnails(state);
            window.hide().ok();
        }
    }
}

pub(crate) fn handle_thumbnail_right_click(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    index: usize,
    x: f32,
    y: f32,
) {
    let Some(window) = weak.upgrade() else {
        return;
    };
    let mut state_guard = state.borrow_mut();
    if state_guard.shell.hwnd.0.is_null() {
        return;
    }
    let viewport_x = window.get_viewport_x();
    let viewport_y = window.get_viewport_y();
    let content_offset_y = if state_guard.settings.show_toolbar
        && matches!(state_guard.settings.toolbar_position, ToolbarPosition::Top)
    {
        TOOLBAR_HEIGHT as f32
    } else {
        0.0
    };
    let host_hwnd = state_guard.shell.hwnd;
    let scale = window.window().scale_factor();
    let Some((info, screen_point)) = state_guard.window_collection.windows.get_mut(index).map(|managed_window| {
        populate_cached_icon(managed_window);
        (
            managed_window.info.clone(),
            logical_to_screen_point(
                host_hwnd,
                (managed_window.display_rect.left as f32 + viewport_x + x) * scale,
                (managed_window.display_rect.top as f32 + viewport_y + content_offset_y + y)
                    * scale,
            ),
        )
    }) else {
        return;
    };

    let menu_state = WindowMenuState {
        preserve_aspect_ratio: state_guard.settings.preserve_aspect_ratio_for(&info.app_id),
        hide_on_select: state_guard.settings.hide_on_select_for(&info.app_id),
        hide_on_select_enabled: state_guard.settings.dock_edge.is_none(),
        pin_position: state_guard.settings.is_pinned_position(&info.app_id),
        thumbnail_refresh_mode: state_guard
            .settings
            .thumbnail_refresh_mode_for(&info.app_id),
        current_color_hex: state_guard
            .settings
            .app_color_hex(&info.app_id)
            .map(str::to_owned),
        known_tags: state_guard.settings.known_tags(),
        current_tags: state_guard
            .settings
            .tags_for(&info.app_id)
            .into_iter()
            .collect(),
    };
    drop(state_guard);

    if let Some(action) = show_window_context_menu(host_hwnd, &menu_state, Some(screen_point)) {
        handle_window_menu_action(state, weak, &info, index, action);
    }
}

pub(crate) fn handle_thumbnail_close(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    index: usize,
) {
    let info = {
        let state = state.borrow();
        let Some(managed_window) = state.window_collection.windows.get(index) else {
            return;
        };
        managed_window.info.clone()
    };

    tracing::info!(title = %info.title, app_id = %info.app_id, "closing window from thumbnail button");
    close_target_window(info.hwnd);
    schedule_deferred_refresh(state, weak);
}

pub(crate) fn handle_thumbnail_drag_ended(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    src_idx: usize,
    drop_x: f64,
    drop_y: f64,
) {
    let needs_refresh = {
        let mut state = state.borrow_mut();
        if src_idx >= state.window_collection.windows.len() {
            return;
        }

        let target_idx = state.window_collection.windows.iter().position(|managed_window| {
            let rect = managed_window.target_rect;
            drop_x >= rect.left as f64
                && drop_x <= rect.right as f64
                && drop_y >= rect.top as f64
                && drop_y <= rect.bottom as f64
        });

        if let Some(target_idx) = target_idx {
            if target_idx == src_idx {
                false
            } else {
                let moved_window = state.window_collection.windows.remove(src_idx);
                state.window_collection.windows.insert(target_idx, moved_window);

                let mut seen_apps = std::collections::HashSet::new();
                let mut rules_to_update = Vec::new();
                for (index, window) in state.window_collection.windows.iter().enumerate() {
                    let app_id = window.info.app_id.clone();
                    if !seen_apps.contains(&app_id) {
                        seen_apps.insert(app_id.clone());
                        let app_label = window.info.app_label().to_owned();
                        rules_to_update.push((app_id, app_label, index));
                    }
                }

                for (app_id, app_label, index) in rules_to_update {
                    let rule = state.settings.app_rules.entry(app_id).or_default();
                    rule.display_name = app_label;
                    rule.pinned_position = Some(index);
                }

                let profile = state.workspace_name.clone();
                let _ = state.settings.save(profile.as_deref());

                true
            }
        } else {
            false
        }
    };

    if needs_refresh {
        refresh_windows(state);
        if let Some(window) = weak.upgrade() {
            recompute_and_update_ui(state, &window);
        }
    }
}

pub(crate) fn handle_window_menu_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
    index: usize,
    action: WindowMenuAction,
) {
    let mut needs_window_refresh = false;
    let mut needs_ui_refresh = false;

    match action {
        WindowMenuAction::HideApp => {
            update_settings(state, |settings| {
                let _ = settings.toggle_hidden(&info.app_id, info.app_label());
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
        WindowMenuAction::TogglePinPosition => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_pinned_position(&info.app_id, info.app_label(), index);
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
        WindowMenuAction::ToggleAspectRatio => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_preserve_aspect_ratio(&info.app_id, info.app_label());
            });
            needs_ui_refresh = true;
        }
        WindowMenuAction::ToggleHideOnSelect => {
            if state.borrow().settings.dock_edge.is_none() {
                update_settings(state, |settings| {
                    let _ = settings.toggle_app_hide_on_select(&info.app_id, info.app_label());
                });
                needs_ui_refresh = true;
            }
        }
        WindowMenuAction::SetThumbnailRefreshMode(mode) => {
            update_settings(state, |settings| {
                let _ =
                    settings.set_app_thumbnail_refresh_mode(&info.app_id, info.app_label(), mode);
            });
            release_thumbnails_for_app(state, &info.app_id);
            needs_ui_refresh = true;
        }
        WindowMenuAction::CreateTagFromApp => {
            super::secondary_windows::open_create_tag_dialog(state, weak, info);
        }
        WindowMenuAction::SetColor(color_hex) => {
            update_settings(state, |settings| {
                let _ = settings.set_app_color_hex(
                    &info.app_id,
                    info.app_label(),
                    color_hex.as_deref(),
                );
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
        WindowMenuAction::ToggleTag(tag) => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_tag(&info.app_id, info.app_label(), &tag);
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
        WindowMenuAction::CloseWindow => {
            close_target_window(info.hwnd);
            schedule_deferred_refresh(state, weak);
        }
        WindowMenuAction::KillProcess => {
            kill_target_process(info.hwnd);
            schedule_deferred_refresh(state, weak);
        }
    }

    if needs_window_refresh {
        let _ = refresh_windows(state);
    }

    if needs_ui_refresh {
        refresh_ui(state, weak);
    }
}

fn release_thumbnails_for_app(state: &Rc<RefCell<AppState>>, app_id: &str) {
    let mut state = state.borrow_mut();
    for managed_window in &mut state.window_collection.windows {
        if managed_window.info.app_id == app_id {
            release_thumbnail(managed_window);
        }
    }
}

fn activate_window(hwnd: HWND) {
    unsafe {
        if windows::Win32::UI::WindowsAndMessaging::IsIconic(hwnd).as_bool() {
            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::SW_RESTORE,
            );
        }
        let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(hwnd);
    }
}

fn close_target_window(hwnd: HWND) {
    if hwnd.0.is_null() {
        return;
    }
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(hwnd),
            windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
            windows::Win32::Foundation::WPARAM(0),
            windows::Win32::Foundation::LPARAM(0),
        );
    }
}

fn kill_target_process(hwnd: HWND) {
    use windows::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_TERMINATE,
    };

    if hwnd.0.is_null() {
        return;
    }
    let mut pid: u32 = 0;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
    }
    if pid == 0 {
        return;
    }
    unsafe {
        match OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        ) {
            Ok(process) => {
                if let Err(error) = TerminateProcess(process, 1) {
                    tracing::warn!(%error, pid, "TerminateProcess failed");
                } else {
                    match WaitForSingleObject(process, 1_000) {
                        WAIT_OBJECT_0 => {
                            tracing::info!(pid, "terminated process after thumbnail kill request");
                        }
                        WAIT_TIMEOUT => {
                            tracing::warn!(pid, "timed out waiting for terminated process");
                        }
                        status => tracing::warn!(
                            pid,
                            wait_status = status.0,
                            "unexpected wait status after process termination"
                        ),
                    }
                }
                let _ = CloseHandle(process);
            }
            Err(error) => tracing::warn!(%error, pid, "OpenProcess failed for termination"),
        }
    }
}
