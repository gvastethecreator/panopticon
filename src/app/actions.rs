//! Shared app-level actions that can be dispatched from tray, keyboard, palette, or UI callbacks.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::{DockEdge, ToolbarPosition, WindowGrouping};
use windows::Win32::Foundation::POINT;

use super::action_execution::execute_settings_action;
use super::action_handlers::{
    ActionContext, ActionHandler, CycleThemeHandler, SetDockEdgeHandler, ToggleAlwaysOnTopHandler,
};
use super::command_palette;
use super::layout_actions::cycle_layout;
use super::runtime_effects::{apply_runtime_effects, RuntimeEffect};
use super::secondary_windows;
use super::tray_actions;
use crate::{AppState, MainWindow};

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
            apply_runtime_effects(state, weak, [RuntimeEffect::RefreshUi]);
        }
        AppAction::ToggleAnimations => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
        }
        AppAction::ToggleToolbar => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
        }
        AppAction::ToggleWindowInfo => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
        }
        AppAction::ToggleAlwaysOnTop => {
            ToggleAlwaysOnTopHandler.handle(&mut ActionContext { state, weak });
        }
        AppAction::ToggleMinimizeToTray => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.minimize_to_tray = !settings.minimize_to_tray;
            });
        }
        AppAction::ToggleCloseToTray => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.close_to_tray = !settings.close_to_tray;
            });
        }
        AppAction::ToggleDefaultAspectRatio => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.preserve_aspect_ratio = !settings.preserve_aspect_ratio;
            });
        }
        AppAction::ToggleDefaultHideOnSelect => {
            if state.borrow().settings.dock_edge.is_none() {
                let _ = execute_settings_action(state, weak, |settings| {
                    settings.hide_on_select = !settings.hide_on_select;
                });
            }
        }
        AppAction::ToggleAppIcons => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.show_app_icons = !settings.show_app_icons;
            });
        }
        AppAction::ToggleStartInTray => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.start_in_tray = !settings.start_in_tray;
            });
        }
        AppAction::ToggleLockedLayout => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.locked_layout = !settings.locked_layout;
            });
        }
        AppAction::ToggleLockCellResize => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.lock_cell_resize = !settings.lock_cell_resize;
            });
        }
        AppAction::DismissEmptyStateWelcome => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.dismissed_empty_state_welcome = true;
            });
        }
        AppAction::CycleRefreshInterval => {
            let _ = execute_settings_action(
                state,
                weak,
                panopticon::settings::AppSettings::cycle_refresh_interval,
            );
        }
        AppAction::RefreshNow => {
            apply_runtime_effects(
                state,
                weak,
                [RuntimeEffect::RefreshWindows, RuntimeEffect::RefreshUi],
            );
        }
        AppAction::CycleLayout => {
            cycle_layout(state);
            apply_runtime_effects(state, weak, [RuntimeEffect::RefreshUi]);
        }
        AppAction::CycleTheme { direction } => {
            CycleThemeHandler { direction }.handle(&mut ActionContext { state, weak });
        }
        AppAction::SetMonitorFilter(filter) => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.set_monitor_filter(filter.as_deref());
            });
        }
        AppAction::SetTagFilter(filter) => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.set_tag_filter(filter.as_deref());
            });
        }
        AppAction::SetAppFilter(filter) => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.set_app_filter(filter.as_deref());
            });
        }
        AppAction::ClearAllFilters => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.set_monitor_filter(None);
                settings.set_tag_filter(None);
                settings.set_app_filter(None);
            });
        }
        AppAction::RestoreHidden(app_id) => {
            let _ = execute_settings_action(state, weak, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
            });
        }
        AppAction::RestoreAllHidden => {
            let _ = execute_settings_action(state, weak, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
        }
        AppAction::HideApp { app_id, app_label } => {
            let _ = execute_settings_action(state, weak, |settings| {
                let _ = settings.toggle_hidden(&app_id, &app_label);
            });
        }
        AppAction::SetDockEdge(edge) => {
            SetDockEdgeHandler(edge).handle(&mut ActionContext { state, weak });
        }
        AppAction::SetWindowGrouping(grouping) => {
            let _ = execute_settings_action(state, weak, |settings| {
                settings.group_windows_by = grouping;
            });
        }
        AppAction::SetToolbarPosition(position) => {
            let _ = execute_settings_action(state, weak, |settings| {
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
            let _ =
                super::workspace::load_workspace_into_current_instance(state, weak, workspace_name);
        }
        AppAction::OpenWorkspaceInNewInstance(workspace_name) => {
            let _ = super::workspace::open_workspace_in_new_instance(state, workspace_name);
        }
        AppAction::Exit => {
            apply_runtime_effects(state, weak, [RuntimeEffect::Exit]);
        }
    }
}
