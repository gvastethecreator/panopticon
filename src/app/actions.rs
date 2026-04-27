//! Shared app-level actions that can be dispatched from tray, keyboard, or UI callbacks.

use std::cell::RefCell;
use std::rc::Rc;

use super::dock::apply_topmost_mode;
use super::secondary_windows;
use super::tray_actions;
use crate::{
    cycle_layout, queue_exit_request, refresh_ui, refresh_windows, update_settings, AppState,
    MainWindow,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppAction {
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    ToggleAlwaysOnTop,
    RefreshNow,
    CycleLayout,
    OpenSettingsWindow,
    OpenContextMenu,
    Exit,
}

pub(crate) fn dispatch_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    action: AppAction,
) {
    match action {
        AppAction::ToggleAnimations => {
            update_settings(state, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
            refresh_ui(state, weak);
        }
        AppAction::ToggleToolbar => {
            update_settings(state, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            refresh_ui(state, weak);
        }
        AppAction::ToggleWindowInfo => {
            update_settings(state, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
            refresh_ui(state, weak);
        }
        AppAction::ToggleAlwaysOnTop => {
            update_settings(state, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            let state_ref = state.borrow();
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            secondary_windows::refresh_secondary_window_stacking(state);
            refresh_ui(state, weak);
        }
        AppAction::RefreshNow => {
            let _ = refresh_windows(state);
            refresh_ui(state, weak);
        }
        AppAction::CycleLayout => {
            cycle_layout(state);
            refresh_ui(state, weak);
        }
        AppAction::OpenSettingsWindow => {
            secondary_windows::open_settings_window(state, weak);
        }
        AppAction::OpenContextMenu => {
            tray_actions::open_application_context_menu(state, weak, None, false);
        }
        AppAction::Exit => {
            queue_exit_request();
        }
    }
}
