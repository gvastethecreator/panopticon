use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use panopticon::ui_option_ops::parse_option_value;
use slint::SharedString;

use crate::app::global_hotkey;
use crate::app::native_runtime::apply_configured_main_window_size;
use crate::app::startup;
use crate::{AppState, MainWindow, SettingsWindow};

use super::super::{
    apply_topmost_mode, apply_window_appearance, open_about_window, selected_model_value,
    shortcut_recording_label, stop_shortcut_recording, sync_settings_window_from_state,
};

pub(super) fn register_runtime_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    register_open_about_callback(settings_window, state);
    register_reset_to_defaults_callback(settings_window, state, main_weak);
    register_refresh_now_callback(settings_window, state, main_weak);
    register_check_updates_now_callback(settings_window, state);
    register_shortcut_start_recording_callback(settings_window);
    register_shortcut_stop_recording_callback(settings_window);
    register_restore_hidden_selected_callback(settings_window, state, main_weak);
    register_restore_hidden_all_callback(settings_window, state, main_weak);
}

fn register_open_about_callback(settings_window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
    settings_window.on_open_about({
        let state = state.clone();
        move || {
            open_about_window(&state);
        }
    });
}

fn register_reset_to_defaults_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_reset_to_defaults({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let (hwnd, settings_snapshot, workspace_name) = {
                let mut state = state.borrow_mut();
                let workspace = state.workspace_name.clone();
                state.settings = AppSettings::default();
                state.settings = state.settings.normalized();
                state.current_layout = state.settings.effective_layout();
                let _ = state.settings.save(workspace.as_deref());
                (state.hwnd, state.settings.clone(), workspace)
            };
            startup::sync_run_at_startup(
                settings_snapshot.run_at_startup,
                workspace_name.as_deref(),
            );
            global_hotkey::sync_activate_hotkey(hwnd, &settings_snapshot);
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                if let Some(settings_window) = guard.as_ref() {
                    let state_ref = state.borrow();
                    sync_settings_window_from_state(settings_window, &state_ref);
                }
            });
            let state_ref = state.borrow();
            apply_window_appearance(state_ref.hwnd, &state_ref.settings);
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            let _ = crate::refresh_windows(&state);
            if let Some(main_window) = main_weak.upgrade() {
                let state_ref = state.borrow();
                let _ = apply_configured_main_window_size(&main_window, &state_ref.settings);
                drop(state_ref);
                crate::recompute_and_update_ui(&state, &main_window);
            }
        }
    });
}

fn register_refresh_now_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_refresh_now({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });
}

fn register_check_updates_now_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_check_updates_now({
        let state = state.clone();
        move || {
            let _ = crate::request_update_check(&state, true);
        }
    });
}

fn register_shortcut_start_recording_callback(settings_window: &SettingsWindow) {
    settings_window.on_shortcut_start_recording(|target| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let target = target.trim().to_string();
            if target.is_empty() {
                stop_shortcut_recording(
                    settings_window,
                    "Click a Rec button beside a shortcut field to start recording.",
                );
                return;
            }

            if target == "global_activate" {
                stop_shortcut_recording(
                    settings_window,
                    "Global activate uses modifier chords (Ctrl/Alt/Shift). Enter that one manually.",
                );
                return;
            }

            settings_window.set_shortcut_recording_mode(true);
            settings_window.set_shortcut_recording_target(SharedString::from(target.clone()));
            settings_window.set_shortcut_recording_hint(SharedString::from(format!(
                "Press a key for '{}'. Press Esc to cancel.",
                shortcut_recording_label(&target)
            )));
        });
    });
}

fn register_shortcut_stop_recording_callback(settings_window: &SettingsWindow) {
    settings_window.on_shortcut_stop_recording(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };
            stop_shortcut_recording(settings_window, "Shortcut recording stopped.");
        });
    });
}

fn register_restore_hidden_selected_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_restore_hidden_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let Some(option) = selected_model_value(
                    &settings_window.get_hidden_app_options(),
                    settings_window.get_hidden_app_index(),
                ) else {
                    return;
                };
                let Some(app_id) = parse_option_value(&option) else {
                    return;
                };

                crate::update_settings(&state, |settings| {
                    let _ = settings.restore_hidden_app(&app_id);
                });
                let _ = crate::refresh_windows(&state);
                crate::refresh_ui(&state, &main_weak);
            });
        }
    });
}

fn register_restore_hidden_all_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_restore_hidden_all({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::update_settings(&state, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });
}
