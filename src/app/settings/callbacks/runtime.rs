use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use panopticon::ui_option_ops::parse_option_value;
use slint::SharedString;

use crate::app::action_execution::replace_settings_snapshot;
use crate::app::runtime_support::{refresh_ui, request_update_check, update_settings};
use crate::app::window_sync::refresh_windows;
use crate::{AppState, MainWindow, SettingsWindow};

use crate::app::secondary_windows::open_about_window;
use crate::app::settings::helpers::stop_shortcut_recording;
use crate::app::settings::{selected_model_value, shortcut_recording_label};

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
            let defaults = AppSettings::default();
            let _ = replace_settings_snapshot(&state, &main_weak, &defaults);
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
            let _ = refresh_windows(&state);
            refresh_ui(&state, &main_weak);
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
            let _ = request_update_check(&state, true);
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

                update_settings(&state, |settings| {
                    let _ = settings.restore_hidden_app(&app_id);
                });
                let _ = refresh_windows(&state);
                refresh_ui(&state, &main_weak);
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
            update_settings(&state, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            let _ = refresh_windows(&state);
            refresh_ui(&state, &main_weak);
        }
    });
}
