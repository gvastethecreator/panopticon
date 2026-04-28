//! Shared app-level actions that can be dispatched from tray, keyboard, palette, or UI callbacks.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::{DockEdge, ToolbarPosition, WindowGrouping};
use windows::Win32::Foundation::POINT;

use super::command_palette;
use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, restore_floating_style,
    unregister_appbar,
};
use super::native_runtime::apply_configured_main_window_size;
use super::secondary_windows;
use super::tray_actions;
use crate::{
    cycle_layout, queue_exit_request, refresh_ui, refresh_windows, update_settings, AppState,
    MainWindow,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppAction {
    SetLayout(LayoutType),
    ResetLayoutRatios,
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    ToggleAlwaysOnTop,
    ToggleMinimizeToTray,
    ToggleCloseToTray,
    ToggleDefaultAspectRatio,
    ToggleDefaultHideOnSelect,
    ToggleAppIcons,
    ToggleStartInTray,
    ToggleLockedLayout,
    ToggleLockCellResize,
    DismissEmptyStateWelcome,
    CycleRefreshInterval,
    RefreshNow,
    CycleLayout,
    CycleTheme { direction: i32 },
    SetMonitorFilter(Option<String>),
    SetTagFilter(Option<String>),
    SetAppFilter(Option<String>),
    ClearAllFilters,
    RestoreHidden(String),
    RestoreAllHidden,
    HideApp { app_id: String, app_label: String },
    SetDockEdge(Option<DockEdge>),
    SetWindowGrouping(WindowGrouping),
    SetToolbarPosition(ToolbarPosition),
    OpenSettingsWindowAt(Option<POINT>),
    OpenSettingsPage(i32),
    OpenAboutWindowAt(Option<POINT>),
    OpenContextMenu,
    OpenCommandPalette,
    LoadWorkspace(Option<String>),
    OpenWorkspaceInNewInstance(Option<String>),
    Exit,
}

fn mutate_settings_and_refresh(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    mutate: impl FnOnce(&mut panopticon::settings::AppSettings),
) {
    update_settings(state, mutate);
    refresh_ui(state, weak);
}

fn mutate_settings_and_refresh_windows(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    mutate: impl FnOnce(&mut panopticon::settings::AppSettings),
) {
    update_settings(state, mutate);
    let _ = refresh_windows(state);
    refresh_ui(state, weak);
}

fn cycle_theme(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>, direction: i32) {
    let current_idx = {
        let state = state.borrow();
        panopticon::theme::theme_index(state.settings.theme_id.as_deref())
    };
    let total = panopticon::theme::theme_labels().len() as i32;
    let next_idx = (current_idx + direction).rem_euclid(total);
    let new_id = panopticon::theme::theme_id_by_index(next_idx);
    let next_background_hex =
        panopticon::theme::theme_base_background_hex(new_id.as_deref(), "181513");

    update_settings(state, |settings| {
        settings.theme_id = new_id;
        if settings.theme_id.is_some() {
            settings
                .background_color_hex
                .clone_from(&next_background_hex);
        }
    });

    let state_ref = state.borrow();
    apply_window_appearance(state_ref.hwnd, &state_ref.settings);
    drop(state_ref);

    refresh_ui(state, weak);
}

#[expect(
    clippy::too_many_lines,
    reason = "centralized runtime dispatch intentionally keeps shared action behavior in one audited entry point"
)]
pub(crate) fn dispatch_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    action: AppAction,
) {
    match action {
        AppAction::SetLayout(layout) => {
            super::layout_actions::set_layout(state, weak, layout);
        }
        AppAction::ResetLayoutRatios => {
            super::layout_actions::reset_layout_custom(state);
            refresh_ui(state, weak);
        }
        AppAction::ToggleAnimations => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
        }
        AppAction::ToggleToolbar => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
        }
        AppAction::ToggleWindowInfo => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
        }
        AppAction::ToggleAlwaysOnTop => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            let state_ref = state.borrow();
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            secondary_windows::refresh_secondary_window_stacking(state);
        }
        AppAction::ToggleMinimizeToTray => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.minimize_to_tray = !settings.minimize_to_tray;
            });
        }
        AppAction::ToggleCloseToTray => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.close_to_tray = !settings.close_to_tray;
            });
        }
        AppAction::ToggleDefaultAspectRatio => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.preserve_aspect_ratio = !settings.preserve_aspect_ratio;
            });
        }
        AppAction::ToggleDefaultHideOnSelect => {
            if state.borrow().settings.dock_edge.is_none() {
                mutate_settings_and_refresh(state, weak, |settings| {
                    settings.hide_on_select = !settings.hide_on_select;
                });
            }
        }
        AppAction::ToggleAppIcons => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.show_app_icons = !settings.show_app_icons;
            });
        }
        AppAction::ToggleStartInTray => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.start_in_tray = !settings.start_in_tray;
            });
        }
        AppAction::ToggleLockedLayout => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.locked_layout = !settings.locked_layout;
            });
        }
        AppAction::ToggleLockCellResize => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.lock_cell_resize = !settings.lock_cell_resize;
            });
        }
        AppAction::DismissEmptyStateWelcome => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.dismissed_empty_state_welcome = true;
            });
        }
        AppAction::CycleRefreshInterval => {
            mutate_settings_and_refresh(
                state,
                weak,
                panopticon::settings::AppSettings::cycle_refresh_interval,
            );
        }
        AppAction::RefreshNow => {
            let _ = refresh_windows(state);
            refresh_ui(state, weak);
        }
        AppAction::CycleLayout => {
            cycle_layout(state);
            refresh_ui(state, weak);
        }
        AppAction::CycleTheme { direction } => cycle_theme(state, weak, direction),
        AppAction::SetMonitorFilter(filter) => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                settings.set_monitor_filter(filter.as_deref());
            });
        }
        AppAction::SetTagFilter(filter) => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                settings.set_tag_filter(filter.as_deref());
            });
        }
        AppAction::SetAppFilter(filter) => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                settings.set_app_filter(filter.as_deref());
            });
        }
        AppAction::ClearAllFilters => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                settings.set_monitor_filter(None);
                settings.set_tag_filter(None);
                settings.set_app_filter(None);
            });
        }
        AppAction::RestoreHidden(app_id) => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
            });
        }
        AppAction::RestoreAllHidden => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
        }
        AppAction::HideApp { app_id, app_label } => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                let _ = settings.toggle_hidden(&app_id, &app_label);
            });
        }
        AppAction::SetDockEdge(edge) => {
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
            let _ = refresh_windows(state);
            refresh_ui(state, weak);
        }
        AppAction::SetWindowGrouping(grouping) => {
            mutate_settings_and_refresh_windows(state, weak, |settings| {
                settings.group_windows_by = grouping;
            });
        }
        AppAction::SetToolbarPosition(position) => {
            mutate_settings_and_refresh(state, weak, |settings| {
                settings.toolbar_position = position;
            });
        }
        AppAction::OpenSettingsWindowAt(center_point) => {
            secondary_windows::open_settings_window_with_anchor(state, weak, center_point);
        }
        AppAction::OpenSettingsPage(page_index) => {
            secondary_windows::open_settings_window_page(state, weak, page_index);
        }
        AppAction::OpenAboutWindowAt(center_point) => {
            secondary_windows::open_about_window_with_anchor(state, center_point);
        }
        AppAction::OpenContextMenu => {
            tray_actions::open_application_context_menu(state, weak, None, false);
        }
        AppAction::OpenCommandPalette => {
            command_palette::open_command_palette_window(state, weak);
        }
        AppAction::LoadWorkspace(workspace_name) => {
            let _ = secondary_windows::load_workspace_into_current_instance(
                state,
                weak,
                workspace_name,
            );
        }
        AppAction::OpenWorkspaceInNewInstance(workspace_name) => {
            let _ = secondary_windows::open_workspace_in_new_instance(state, workspace_name);
        }
        AppAction::Exit => {
            queue_exit_request();
        }
    }
}
