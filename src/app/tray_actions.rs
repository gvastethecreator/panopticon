//! Tray-driven actions and application menu orchestration.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};
use slint::ComponentHandle;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, IsIconic, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
    SW_SHOW,
};

use super::actions::{dispatch_action, AppAction};
use super::dock::{
    apply_dock_mode, apply_topmost_mode, center_window_on_point_monitor,
    current_cursor_screen_point, restore_floating_style, unregister_appbar,
};
use super::native_runtime::apply_configured_main_window_size;
use super::secondary_windows;
use super::tray::{show_application_context_menu_at, TrayAction, TrayMenuState};
use crate::{
    logical_to_screen_point, recompute_and_update_ui, refresh_ui, refresh_windows,
    release_all_thumbnails, update_settings, AppState, MainWindow,
};

pub(crate) fn build_tray_menu_state(state: &mut AppState) -> TrayMenuState {
    let available_windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();
    for window in &available_windows {
        state
            .settings
            .refresh_app_label(&window.app_id, window.app_label());
    }

    TrayMenuState {
        window_visible: unsafe { IsWindowVisible(state.hwnd).as_bool() },
        minimize_to_tray: state.settings.minimize_to_tray,
        close_to_tray: state.settings.close_to_tray,
        refresh_interval_ms: state.settings.refresh_interval_ms,
        animate_transitions: state.settings.animate_transitions,
        preserve_aspect_ratio: state.settings.preserve_aspect_ratio,
        hide_on_select: state.settings.hide_on_select,
        always_on_top: state.settings.always_on_top,
        active_monitor_filter: state.settings.active_monitor_filter.clone(),
        available_monitors: collect_available_monitors(&available_windows),
        active_tag_filter: state.settings.active_tag_filter.clone(),
        available_tags: state.settings.known_tags(),
        active_app_filter: state.settings.active_app_filter.clone(),
        available_apps: collect_available_apps(&available_windows),
        hidden_apps: state.settings.hidden_app_entries(),
        dock_edge: state.settings.dock_edge,
        is_docked: state.is_appbar || state.settings.dock_edge.is_some(),
        show_toolbar: state.settings.show_toolbar,
        show_window_info: state.settings.show_window_info,
        show_app_icons: state.settings.show_app_icons,
        toolbar_position: state.settings.toolbar_position,
        start_in_tray: state.settings.start_in_tray,
        locked_layout: state.settings.locked_layout,
        lock_cell_resize: state.settings.lock_cell_resize,
        group_windows_by: state.settings.group_windows_by,
        current_workspace: state.workspace_name.clone(),
        available_workspaces: panopticon::settings::AppSettings::list_workspaces_with_default()
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "failed to enumerate workspaces for tray menu");
                vec!["default".to_owned()]
            }),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "matches every tray command to its corresponding runtime action"
)]
pub(crate) fn handle_tray_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    action: TrayAction,
    activation_point: Option<POINT>,
) {
    match action {
        TrayAction::Toggle => toggle_visibility(state, weak, activation_point),
        TrayAction::Refresh => dispatch_action(state, weak, AppAction::RefreshNow),
        TrayAction::NextLayout => dispatch_action(state, weak, AppAction::CycleLayout),
        TrayAction::ToggleMinimizeToTray => {
            update_settings(state, |settings| {
                settings.minimize_to_tray = !settings.minimize_to_tray;
            });
        }
        TrayAction::ToggleCloseToTray => {
            update_settings(state, |settings| {
                settings.close_to_tray = !settings.close_to_tray;
            });
        }
        TrayAction::CycleRefreshInterval => {
            update_settings(
                state,
                panopticon::settings::AppSettings::cycle_refresh_interval,
            );
            refresh_ui(state, weak);
        }
        TrayAction::ToggleAnimateTransitions => {
            dispatch_action(state, weak, AppAction::ToggleAnimations);
        }
        TrayAction::ToggleDefaultAspectRatio => {
            update_settings(state, |settings| {
                settings.preserve_aspect_ratio = !settings.preserve_aspect_ratio;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleDefaultHideOnSelect => {
            if state.borrow().settings.dock_edge.is_none() {
                update_settings(state, |settings| {
                    settings.hide_on_select = !settings.hide_on_select;
                });
                refresh_ui(state, weak);
            }
        }
        TrayAction::ToggleAlwaysOnTop => {
            dispatch_action(state, weak, AppAction::ToggleAlwaysOnTop);
        }
        TrayAction::SetMonitorFilter(filter) => {
            update_settings(state, |settings| {
                settings.set_monitor_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetTagFilter(filter) => {
            update_settings(state, |settings| {
                settings.set_tag_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetAppFilter(filter) => {
            update_settings(state, |settings| {
                settings.set_app_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::RestoreHidden(app_id) => {
            update_settings(state, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::RestoreAllHidden => {
            update_settings(state, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetDockEdge(edge) => {
            let mut floating_settings = None;
            {
                let mut state = state.borrow_mut();
                if state.is_appbar {
                    unregister_appbar(state.hwnd);
                    state.is_appbar = false;
                }
                state.settings.dock_edge = edge;
                state.settings = state.settings.normalized();
                state.current_layout = state.settings.effective_layout();
                let _ = state.settings.save(state.workspace_name.as_deref());
                if edge.is_some() {
                    apply_dock_mode(&mut state);
                } else {
                    restore_floating_style(state.hwnd);
                    apply_topmost_mode(state.hwnd, state.settings.always_on_top);
                    floating_settings = Some(state.settings.clone());
                }
            }
            if let Some(settings) = floating_settings {
                if let Some(main_window) = weak.upgrade() {
                    let _ = apply_configured_main_window_size(&main_window, &settings);
                }
            }
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetWindowGrouping(grouping) => {
            update_settings(state, |settings| {
                settings.group_windows_by = grouping;
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::ToggleToolbar => {
            dispatch_action(state, weak, AppAction::ToggleToolbar);
        }
        TrayAction::SetToolbarPosition(position) => {
            update_settings(state, |settings| {
                settings.toolbar_position = position;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleWindowInfo => {
            dispatch_action(state, weak, AppAction::ToggleWindowInfo);
        }
        TrayAction::ToggleAppIcons => {
            update_settings(state, |settings| {
                settings.show_app_icons = !settings.show_app_icons;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleStartInTray => {
            update_settings(state, |settings| {
                settings.start_in_tray = !settings.start_in_tray;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleLockedLayout => {
            update_settings(state, |settings| {
                settings.locked_layout = !settings.locked_layout;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleLockCellResize => {
            update_settings(state, |settings| {
                settings.lock_cell_resize = !settings.lock_cell_resize;
            });
            refresh_ui(state, weak);
        }
        TrayAction::OpenSettingsWindow => {
            secondary_windows::open_settings_window_with_anchor(state, weak, activation_point);
        }
        TrayAction::OpenAboutWindow => {
            secondary_windows::open_about_window_with_anchor(state, activation_point);
        }
        TrayAction::LoadWorkspace(workspace_name) => {
            let _ = secondary_windows::load_workspace_into_current_instance(
                state,
                weak,
                workspace_name,
            );
        }
        TrayAction::Exit => {
            dispatch_action(state, weak, AppAction::Exit);
        }
    }
}

pub(crate) fn open_application_context_menu(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    coords: Option<(f32, f32)>,
    prefer_below_anchor: bool,
) {
    let Some(window) = weak.upgrade() else {
        return;
    };

    let (hwnd, anchor, menu_state) = {
        let mut guard = state.borrow_mut();
        if guard.hwnd.0.is_null() {
            return;
        }
        let anchor = coords.map(|(x, y)| {
            logical_to_screen_point(
                guard.hwnd,
                x * window.window().scale_factor(),
                y * window.window().scale_factor(),
            )
        });
        (guard.hwnd, anchor, build_tray_menu_state(&mut guard))
    };

    let anchor = anchor.or_else(current_cursor_screen_point);

    if let Some(action) =
        show_application_context_menu_at(hwnd, &menu_state, anchor, prefer_below_anchor)
    {
        handle_tray_action(state, weak, action, anchor);
    }
}

fn toggle_visibility(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    activation_point: Option<POINT>,
) {
    let visible = state.borrow().hwnd != HWND::default()
        && unsafe {
            // SAFETY: read-only visibility/minimized queries for a live top-level window.
            IsWindowVisible(state.borrow().hwnd).as_bool()
                && !IsIconic(state.borrow().hwnd).as_bool()
        };
    if visible {
        release_all_thumbnails(state);
        if let Some(window) = weak.upgrade() {
            window.hide().ok();
        }
    } else {
        activate_main_window_with_anchor(state, weak, activation_point);
    }
}

pub(crate) fn activate_main_window(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    activate_main_window_with_anchor(state, weak, None);
}

fn activate_main_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    activation_point: Option<POINT>,
) {
    if let Some(window) = weak.upgrade() {
        let hwnd = state.borrow().hwnd;
        let was_visible = hwnd != HWND::default()
            && unsafe {
                // SAFETY: read-only visibility query for a live top-level window.
                IsWindowVisible(hwnd).as_bool()
            };
        let was_minimized = hwnd != HWND::default()
            && unsafe {
                // SAFETY: read-only iconic-state query for a live top-level window.
                IsIconic(hwnd).as_bool()
            };

        window.show().ok();

        unsafe {
            // SAFETY: foreground/z-order restoration for the application's own top-level window.
            let _ = ShowWindow(hwnd, if was_minimized { SW_RESTORE } else { SW_SHOW });
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }

        if !was_visible && !was_minimized && state.borrow().settings.dock_edge.is_none() {
            if let Some(point) = activation_point.or_else(current_cursor_screen_point) {
                center_window_on_point_monitor(hwnd, point);
            }
        }

        if !was_visible || was_minimized {
            refresh_windows(state);
            recompute_and_update_ui(state, &window);
        }
    }
}
