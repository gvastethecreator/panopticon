use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, SharedString, Timer, TimerMode};

use crate::{AppState, MainWindow, SettingsWindow};
use crate::app::runtime_support::refresh_ui;
use crate::app::window_sync::refresh_windows;

use super::super::{
    apply_background_color, apply_recorded_shortcut_binding, apply_settings_window_to_state,
    normalize_recorded_shortcut, parse_rgb_hex, selected_layout_preset_name,
    set_layout_preset_summary, shortcut_recording_label, stop_shortcut_recording,
    sync_layout_preset_controls,
};

pub(super) fn register_editor_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    register_save_layout_preset_callback(settings_window, state);
    register_apply_layout_preset_callback(settings_window, state, main_weak);
    register_delete_layout_preset_callback(settings_window, state);
    register_background_image_callbacks(settings_window);
    register_background_color_callbacks(settings_window);
    register_apply_callback(settings_window, state, main_weak);
    register_key_pressed_callback(settings_window, state, main_weak);
    register_closed_callback(settings_window);
}

fn register_save_layout_preset_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_save_layout_preset({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let preset_name = settings_window.get_layout_preset_name().trim().to_owned();
                if preset_name.is_empty() {
                    set_layout_preset_summary(
                        settings_window,
                        "Enter a preset name before saving.",
                    );
                    return;
                }

                let result = {
                    let mut state_guard = state.borrow_mut();
                    let active_layout = state_guard.window_collection.current_layout;
                    match state_guard
                        .settings
                        .save_layout_preset(&preset_name, active_layout)
                    {
                        Ok(()) => {
                            state_guard.settings = state_guard.settings.normalized();
                            if let Err(error) =
                                state_guard.settings.save(state_guard.workspace_name.as_deref())
                            {
                                tracing::error!(%error, preset = %preset_name, "failed to persist layout preset save");
                                Err("Saved in memory, but failed to persist preset to disk.".to_owned())
                            } else {
                                Ok(())
                            }
                        }
                        Err(reason) => Err(reason),
                    }
                };

                match result {
                    Ok(()) => {
                        let state_guard = state.borrow();
                        sync_layout_preset_controls(settings_window, &state_guard.settings);
                        settings_window.set_layout_preset_name(SharedString::from(preset_name.clone()));
                        set_layout_preset_summary(
                            settings_window,
                            &format!("Saved layout preset '{preset_name}'."),
                        );
                    }
                    Err(reason) => {
                        set_layout_preset_summary(settings_window, &reason);
                    }
                }
            });
        }
    });
}

fn register_apply_layout_preset_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_apply_layout_preset({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let Some(preset_name) = selected_layout_preset_name(settings_window) else {
                    set_layout_preset_summary(settings_window, "Select a preset to apply.");
                    return;
                };

                let apply_outcome = {
                    let mut state_guard = state.borrow_mut();
                    if state_guard.settings.apply_layout_preset(&preset_name) {
                        state_guard.settings = state_guard.settings.normalized();
                        state_guard.window_collection.current_layout = state_guard.settings.effective_layout();
                        if let Err(error) = state_guard
                            .settings
                            .save(state_guard.workspace_name.as_deref())
                        {
                            tracing::error!(%error, preset = %preset_name, "failed to persist layout preset apply");
                            Some(false)
                        } else {
                            Some(true)
                        }
                    } else {
                        None
                    }
                };

                match apply_outcome {
                    None => {
                        set_layout_preset_summary(
                            settings_window,
                            "Could not apply layout preset. It may have been renamed or deleted.",
                        );
                    }
                    Some(false) => {
                        set_layout_preset_summary(
                            settings_window,
                            "Applied in memory, but failed to persist layout preset changes.",
                        );
                    }
                    Some(true) => {
                        settings_window.set_layout_preset_name(SharedString::from(preset_name.clone()));
                        set_layout_preset_summary(
                            settings_window,
                            &format!("Applied layout preset '{preset_name}'."),
                        );
                        let _ = refresh_windows(&state);
                        refresh_ui(&state, &main_weak);
                    }
                }
            });
        }
    });
}

fn register_delete_layout_preset_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_delete_layout_preset({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let Some(preset_name) = selected_layout_preset_name(settings_window) else {
                    set_layout_preset_summary(settings_window, "Select a preset to delete.");
                    return;
                };

                let deleted = {
                    let mut state_guard = state.borrow_mut();
                    let removed = state_guard.settings.delete_layout_preset(&preset_name);
                    if removed {
                        state_guard.settings = state_guard.settings.normalized();
                        if let Err(error) = state_guard
                            .settings
                            .save(state_guard.workspace_name.as_deref())
                        {
                            tracing::error!(%error, preset = %preset_name, "failed to persist layout preset deletion");
                        }
                    }
                    removed
                };

                if deleted {
                    let state_guard = state.borrow();
                    sync_layout_preset_controls(settings_window, &state_guard.settings);
                    set_layout_preset_summary(
                        settings_window,
                        &format!("Deleted layout preset '{preset_name}'."),
                    );
                } else {
                    set_layout_preset_summary(
                        settings_window,
                        "Could not delete layout preset. It may have already been removed.",
                    );
                }
            });
        }
    });
}

fn register_background_image_callbacks(settings_window: &SettingsWindow) {
    settings_window.on_browse_background_image(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let dialog = rfd::FileDialog::new()
                .add_filter(
                    "Images",
                    &["png", "jpg", "jpeg", "bmp", "gif", "webp", "svg"],
                )
                .set_title(panopticon::i18n::t("dialog.choose_background_image"));

            let dialog = if settings_window.get_bg_image_path().is_empty() {
                dialog
            } else {
                let current_path = settings_window.get_bg_image_path().to_string();
                let start_dir = Path::new(&current_path)
                    .parent()
                    .unwrap_or_else(|| Path::new(&current_path));
                dialog.set_directory(start_dir)
            };

            if let Some(path) = dialog.pick_file() {
                settings_window.set_bg_image_path(SharedString::from(path.display().to_string()));
                if let Ok(image) = slint::Image::load_from_path(path.as_path()) {
                    settings_window.set_bg_image_preview(image);
                }
                settings_window.invoke_apply();
            }
        });
    });

    settings_window.on_clear_background_image(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };
            settings_window.set_bg_image_path(SharedString::from(""));
            settings_window.set_bg_image_preview(slint::Image::default());
            settings_window.invoke_apply();
        });
    });
}

fn register_background_color_callbacks(settings_window: &SettingsWindow) {
    settings_window.on_apply_bg_color_hex(|hex| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            if let Some((red, green, blue)) = parse_rgb_hex(&hex) {
                apply_background_color(settings_window, red, green, blue);
            } else {
                let red = settings_window.get_bg_red_value();
                let green = settings_window.get_bg_green_value();
                let blue = settings_window.get_bg_blue_value();
                apply_background_color(settings_window, red, green, blue);
            }
        });
    });

    settings_window.on_apply_bg_color_rgb(|red, green, blue| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            apply_background_color(settings_window, red, green, blue);
        });
    });
}

fn register_apply_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    let apply_debounce_timer = Rc::new(Timer::default());
    settings_window.on_apply({
        let state = state.clone();
        let main_weak = main_weak.clone();
        let apply_debounce_timer = apply_debounce_timer.clone();
        move || {
            let should_skip = crate::SETTINGS_WIN.with(|handle| {
                handle
                    .borrow()
                    .as_ref()
                    .is_some_and(SettingsWindow::get_suspend_live_apply)
            });
            if should_skip {
                tracing::debug!("skipping apply while settings window sync is suspended");
                return;
            }

            let state = state.clone();
            let main_weak = main_weak.clone();
            apply_debounce_timer.start(
                TimerMode::SingleShot,
                Duration::from_millis(140),
                move || {
                    apply_settings_window_to_state(&state, &main_weak);
                },
            );
        }
    });
}

fn register_key_pressed_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_key_pressed({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move |key_text, shift_pressed| {
            let intercepted = crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return false;
                };

                if !settings_window.get_shortcut_recording_mode() {
                    return false;
                }

                if key_text == "\u{001B}" {
                    stop_shortcut_recording(settings_window, "Shortcut recording cancelled.");
                    return true;
                }

                let Some(binding) = normalize_recorded_shortcut(key_text.as_str()) else {
                    settings_window.set_shortcut_recording_hint(SharedString::from(
                        "Unsupported key for shortcut recording. Try letters, digits, Tab, Enter, Space, or Esc.",
                    ));
                    return true;
                };

                let target = settings_window.get_shortcut_recording_target().to_string();
                if target.trim().is_empty() {
                    stop_shortcut_recording(
                        settings_window,
                        "No shortcut target selected. Click a Rec button first.",
                    );
                    return true;
                }

                if !apply_recorded_shortcut_binding(settings_window, &target, &binding) {
                    stop_shortcut_recording(
                        settings_window,
                        "Unknown shortcut target. Please choose a field and try again.",
                    );
                    return true;
                }

                settings_window.invoke_apply();
                stop_shortcut_recording(
                    settings_window,
                    &format!(
                        "Recorded '{}' for '{}'.",
                        binding,
                        shortcut_recording_label(&target)
                    ),
                );
                true
            });

            if intercepted {
                true
            } else {
                crate::app::keyboard_actions::handle_key(
                    &state,
                    &main_weak,
                    &key_text,
                    shift_pressed,
                )
            }
        }
    });
}

fn register_closed_callback(settings_window: &SettingsWindow) {
    settings_window.on_closed(|| {
        let taken = crate::SETTINGS_WIN.with(|handle| handle.borrow_mut().take());
        if let Some(window) = taken {
            window.hide().ok();
        }
    });
}
