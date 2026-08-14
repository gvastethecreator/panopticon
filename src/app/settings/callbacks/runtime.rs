use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use panopticon::ui_option_ops::parse_option_value;
use slint::SharedString;

use crate::app::action_execution::{persist_settings_snapshot, replace_settings_snapshot};
use crate::app::runtime_support::{refresh_ui, request_update_check, update_settings};
use crate::app::window_sync::refresh_windows;
use crate::{AppState, MainWindow, SettingsWindow};

use crate::app::secondary_windows::{
    confirm_workspace_action, open_about_window, show_action_result,
};
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
    register_cancel_update_check_callback(settings_window, state);
    register_persistence_recovery_callbacks(settings_window, state);
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
            if !confirm_workspace_action(
                panopticon::i18n::t("dialog.reset_defaults.title"),
                panopticon::i18n::t("dialog.reset_defaults.description"),
            ) {
                return;
            }

            let defaults = AppSettings::default();
            let changed = replace_settings_snapshot(&state, &main_weak, &defaults);
            let persisted = if changed {
                state.borrow().persistence_status == crate::PersistenceStatus::Clean
            } else {
                persist_settings_snapshot(&mut state.borrow_mut())
            };

            let (title_key, message_key) = if persisted {
                (
                    "dialog.reset_defaults.success_title",
                    "dialog.reset_defaults.success",
                )
            } else {
                (
                    "dialog.reset_defaults.failed_title",
                    "dialog.reset_defaults.failed",
                )
            };
            show_action_result(
                panopticon::i18n::t(title_key),
                panopticon::i18n::t(message_key),
                persisted,
            );
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

fn register_cancel_update_check_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_cancel_update_check({
        let state = state.clone();
        move || {
            crate::app::updates::cancel_latest_release_check();
            state.borrow_mut().update_status = crate::UpdateStatus::Idle;
            crate::app::secondary_windows::refresh_open_settings_window(&state);
            crate::app::secondary_windows::refresh_open_about_window(&state);
        }
    });
}

fn register_persistence_recovery_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_retry_persistence({
        let state = state.clone();
        move || {
            let persisted = persist_settings_snapshot(&mut state.borrow_mut());
            crate::app::secondary_windows::refresh_open_settings_window(&state);
            if persisted {
                tracing::info!("manual settings persistence retry succeeded");
            }
        }
    });
    settings_window.on_open_log_folder(|| {
        let log_directory = panopticon::logging::log_directory();
        if let Err(error) = std::fs::create_dir_all(&log_directory) {
            tracing::warn!(%error, path = %log_directory.display(), "failed to create log directory");
            return;
        }
        if let Err(error) = std::process::Command::new("explorer.exe")
            .arg(&log_directory)
            .spawn()
        {
            tracing::warn!(%error, path = %log_directory.display(), "failed to open log directory");
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
                    panopticon::i18n::t("settings.shortcut.feedback.select_target"),
                );
                return;
            }

            if target == "global_activate" {
                stop_shortcut_recording(
                    settings_window,
                    panopticon::i18n::t("settings.shortcut.feedback.global_manual"),
                );
                return;
            }

            settings_window.set_shortcut_recording_mode(true);
            settings_window.set_shortcut_recording_target(SharedString::from(target.clone()));
            settings_window.set_shortcut_recording_hint(SharedString::from(
                panopticon::i18n::t_fmt(
                    "settings.shortcut.feedback.press_key",
                    shortcut_recording_label(&target),
                ),
            ));
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
            stop_shortcut_recording(
                settings_window,
                panopticon::i18n::t("settings.shortcut.feedback.stopped"),
            );
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
