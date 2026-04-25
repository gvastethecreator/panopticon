//! Secondary Slint windows: settings, tag dialog, and workspace helpers.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;

use panopticon::settings::{
    AppSelectionEntry, AppSettings, HiddenAppEntry, ThumbnailRefreshMode, WorkspaceNameValidation,
    MIN_DOCK_COLUMN_THICKNESS, MIN_DOCK_ROW_THICKNESS, MIN_FIXED_WINDOW_HEIGHT,
    MIN_FIXED_WINDOW_WIDTH,
};
use panopticon::theme as theme_catalog;
use panopticon::ui_option_ops::{
    app_option_label, current_workspace_label, hidden_app_option_label, parse_option_value,
    suggested_tag_name, tag_color_hex, tag_color_index, OPTION_SEPARATOR,
};
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};
use slint::{
    CloseRequestResponse, ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel,
};
use windows::Win32::Foundation::HWND;

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, center_window_on_owner_monitor,
    keep_dialog_above_owner, reposition_appbar, restore_floating_style, unregister_appbar,
};
use super::global_hotkey;
use super::native_runtime::apply_configured_main_window_size;
use super::settings_ui::{apply_settings_window_changes, populate_settings_window};
use super::startup;
use super::theme_ui::{
    apply_about_window_theme_snapshot, apply_main_window_theme_snapshot,
    apply_settings_window_theme_snapshot, apply_tag_dialog_theme_snapshot,
};
use super::tray::apply_window_icons;
use crate::{AboutWindow, AppState, MainWindow, SettingsWindow, TagDialogWindow};

thread_local! {
    static SETTINGS_APPLY_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
    static BG_COLOR_SYNC_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

struct SettingsApplyGuard;

impl SettingsApplyGuard {
    fn enter() -> Option<Self> {
        let already_running = SETTINGS_APPLY_IN_PROGRESS.with(|flag| {
            if flag.get() {
                true
            } else {
                flag.set(true);
                false
            }
        });

        if already_running {
            None
        } else {
            Some(Self)
        }
    }
}

impl Drop for SettingsApplyGuard {
    fn drop(&mut self) {
        SETTINGS_APPLY_IN_PROGRESS.with(|flag| flag.set(false));
    }
}

fn available_workspace_options() -> Vec<String> {
    AppSettings::list_workspaces_with_default().unwrap_or_else(|error| {
        tracing::warn!(%error, "failed to enumerate available workspaces");
        vec!["default".to_owned()]
    })
}

fn selected_workspace_from_settings_window(window: &SettingsWindow) -> Option<String> {
    selected_model_value(
        &window.get_available_profile_options(),
        window.get_available_profile_index(),
    )
    .and_then(|value| panopticon::ui_option_ops::selected_workspace_name(&value))
}

struct RuntimeUiOptions {
    monitors: Vec<String>,
    tags: Vec<String>,
    apps: Vec<AppSelectionEntry>,
    hidden_apps: Vec<HiddenAppEntry>,
}

struct AppRuleListEntry {
    option: AppSelectionEntry,
    is_running: bool,
    has_saved_rule: bool,
    is_hidden: bool,
    has_tags: bool,
    has_custom_refresh: bool,
    is_pinned: bool,
    searchable_blob: String,
}

pub(crate) fn ensure_default_workspaces_exist(settings: &AppSettings) {
    match AppSettings::list_workspaces() {
        Ok(workspaces) if workspaces.is_empty() => {
            for workspace_name in ["workspace-1", "workspace-2"] {
                if let Err(error) = settings.save(Some(workspace_name)) {
                    tracing::error!(%error, workspace = workspace_name, "failed to seed default workspace");
                }
            }
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "failed to inspect saved workspaces"),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "coordinates the SettingsWindow lifecycle and its callback wiring in one place"
)]
pub(crate) fn open_settings_window(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    let already_open = crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                apply_window_icons(hwnd, &state.icons);
                keep_dialog_above_owner(hwnd, state.hwnd, &state.settings);
                center_window_on_owner_monitor(hwnd, state.hwnd);
            }
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let settings_window = match SettingsWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create settings window");
            return;
        }
    };
    crate::populate_tr_global(&settings_window);

    {
        let state = state.borrow();
        sync_settings_window_from_state(&settings_window, &state);
    }

    settings_window.on_save_profile({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let workspace_name = match panopticon::settings::validate_workspace_name_input(
                    &settings_window.get_profile_name(),
                ) {
                    WorkspaceNameValidation::Valid(workspace_name) => workspace_name,
                    WorkspaceNameValidation::Empty => {
                        tracing::warn!("ignoring empty workspace save request");
                        return;
                    }
                    WorkspaceNameValidation::Invalid(reason) => {
                        tracing::warn!(%reason, "ignoring invalid workspace save request");
                        return;
                    }
                };

                let settings_snapshot = state.borrow().settings.normalized();
                if save_settings_as_workspace(&settings_snapshot, &workspace_name) {
                    settings_window
                        .set_known_profiles_label(SharedString::from(known_workspaces_label()));
                }
            });
        }
    });

    settings_window.on_open_profile_instance({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let current_workspace = selected_workspace_from_settings_window(settings_window)
                    .or_else(|| state.borrow().workspace_name.clone());
                let requested = match panopticon::settings::validate_workspace_name_input(
                    &settings_window.get_profile_name(),
                ) {
                    WorkspaceNameValidation::Valid(workspace_name) => Some(workspace_name),
                    WorkspaceNameValidation::Empty => current_workspace,
                    WorkspaceNameValidation::Invalid(reason) => {
                        tracing::warn!(%reason, "ignoring invalid extra-instance workspace request");
                        return;
                    }
                };

                let settings_snapshot = state.borrow().settings.normalized();
                if let Some(workspace_name) = requested.as_deref() {
                    let _ = save_settings_as_workspace(&settings_snapshot, workspace_name);
                } else if let Err(error) = settings_snapshot.save(None) {
                    tracing::error!(%error, "failed to save default workspace before launching instance");
                }

                let _ = launch_additional_instance(requested.as_deref());
                settings_window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
            });
        }
    });

    settings_window.on_load_selected_profile({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let requested = selected_workspace_from_settings_window(settings_window);
                drop(guard);
                let _ = load_workspace_into_current_instance(&state, &main_weak, requested);
            });
        }
    });

    settings_window.on_open_about({
        let state = state.clone();
        move || {
            open_about_window(&state);
        }
    });

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

    settings_window.on_refresh_now({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });

    settings_window.on_check_updates_now({
        let state = state.clone();
        move || {
            let _ = crate::request_update_check(&state, true);
        }
    });

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

    settings_window.on_app_rules_select_app({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let state_guard = state.borrow();
                sync_selected_app_rule_editor(settings_window, &state_guard.settings);
            });
        }
    });

    settings_window.on_app_rules_refresh_list({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let state_guard = state.borrow();
                settings_window.set_suspend_live_apply(true);
                populate_settings_window_runtime_fields(settings_window, &state_guard);
                settings_window.set_suspend_live_apply(false);
            });
        }
    });

    settings_window.on_app_rules_apply_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let selected = selected_model_value(
                    &settings_window.get_app_rules_options(),
                    settings_window.get_app_rules_index(),
                );
                let Some(selected_option) = selected else {
                    return;
                };
                let Some(app_id) = parse_option_value(&selected_option) else {
                    return;
                };

                let display_name = settings_window
                    .get_app_rules_selected_app_label()
                    .to_string();
                let hidden = settings_window.get_app_rules_hidden();
                let preserve_aspect = settings_window.get_app_rules_preserve_aspect();
                let hide_on_select = settings_window.get_app_rules_hide_on_select();
                let refresh_mode =
                    refresh_mode_from_index(settings_window.get_app_rules_refresh_mode_index());
                let refresh_interval_ms = settings_window
                    .get_app_rules_refresh_interval_ms()
                    .clamp(500, 60_000) as u32;
                let tags_csv = settings_window.get_app_rules_tags().to_string();
                let color_hex = settings_window.get_app_rules_color_hex().to_string();

                crate::update_settings(&state, |settings| {
                    let default_preserve = settings.preserve_aspect_ratio;
                    let default_hide = settings.hide_on_select;
                    let rule = settings.app_rules.entry(app_id.clone()).or_default();

                    if !display_name.trim().is_empty() {
                        rule.display_name = display_name.trim().to_owned();
                    }

                    rule.hidden = hidden;
                    rule.preserve_aspect_ratio = preserve_aspect;
                    rule.preserve_aspect_ratio_override =
                        (preserve_aspect != default_preserve).then_some(preserve_aspect);

                    let effective_hide = if settings.dock_edge.is_some() {
                        false
                    } else {
                        hide_on_select
                    };
                    rule.hide_on_select = effective_hide;
                    rule.hide_on_select_override =
                        (effective_hide != default_hide).then_some(effective_hide);

                    rule.thumbnail_refresh_mode = refresh_mode;
                    rule.thumbnail_refresh_interval_ms = (refresh_mode
                        == ThumbnailRefreshMode::Interval)
                        .then_some(refresh_interval_ms.max(500));

                    rule.tags = parse_tags_csv(&tags_csv);

                    let color = color_hex.trim().trim_start_matches('#');
                    rule.color_hex = if color.is_empty() {
                        None
                    } else {
                        Some(color.to_owned())
                    };
                });

                let _ = crate::refresh_windows(&state);
                crate::refresh_ui(&state, &main_weak);
            });
        }
    });

    settings_window.on_app_rules_reset_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let selected = selected_model_value(
                    &settings_window.get_app_rules_options(),
                    settings_window.get_app_rules_index(),
                );
                let Some(selected_option) = selected else {
                    return;
                };
                let Some(app_id) = parse_option_value(&selected_option) else {
                    return;
                };

                crate::update_settings(&state, |settings| {
                    settings.app_rules.remove(&app_id);
                });

                let _ = crate::refresh_windows(&state);
                crate::refresh_ui(&state, &main_weak);
            });
        }
    });

    settings_window.on_app_rules_clear_unused({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let running_app_ids: BTreeSet<String> = {
                let state_guard = state.borrow();
                collect_runtime_ui_options(&state_guard)
                    .apps
                    .into_iter()
                    .map(|entry| entry.app_id)
                    .collect()
            };

            crate::update_settings(&state, |settings| {
                settings
                    .app_rules
                    .retain(|app_id, _| running_app_ids.contains(app_id));
            });

            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });

    settings_window.on_app_rules_add_tag(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(&settings_window.get_app_rules_tags().to_string());
            let draft = settings_window
                .get_app_rules_tag_input()
                .trim()
                .to_ascii_lowercase();
            if draft.is_empty() {
                return;
            }

            tags.push(draft);
            tags.sort();
            tags.dedup();
            sync_app_rule_tags_editor(settings_window, &tags, true);
        });
    });

    settings_window.on_app_rules_remove_tag(|tag| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(&settings_window.get_app_rules_tags().to_string());
            tags.retain(|candidate| candidate != tag.as_str());
            sync_app_rule_tags_editor(settings_window, &tags, false);
        });
    });

    settings_window.on_app_rules_apply_tag_suggestion(|suggestion| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(&settings_window.get_app_rules_tags().to_string());
            let normalized = suggestion.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                return;
            }

            tags.push(normalized);
            tags.sort();
            tags.dedup();
            sync_app_rule_tags_editor(settings_window, &tags, false);
        });
    });

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

    settings_window.on_key_pressed({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move |key_text, shift_pressed| {
            super::keyboard_actions::handle_key(&state, &main_weak, &key_text, shift_pressed)
        }
    });

    settings_window.on_closed(|| {
        let taken = crate::SETTINGS_WIN.with(|handle| handle.borrow_mut().take());
        if let Some(window) = taken {
            window.hide().ok();
        }
    });

    if let Err(error) = settings_window.show() {
        tracing::error!(%error, "failed to show settings window");
        return;
    }
    if let Some(settings_hwnd) = crate::get_hwnd(settings_window.window()) {
        let state = state.borrow();
        apply_window_icons(settings_hwnd, &state.icons);
        apply_window_appearance(settings_hwnd, &state.settings);
        apply_settings_window_theme_snapshot(&settings_window, &state.current_theme);
        keep_dialog_above_owner(settings_hwnd, state.hwnd, &state.settings);
        center_window_on_owner_monitor(settings_hwnd, state.hwnd);
    }
    crate::SETTINGS_WIN.with(|handle| *handle.borrow_mut() = Some(settings_window));
}

fn apply_settings_window_to_state(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    let Some(_guard) = SettingsApplyGuard::enter() else {
        tracing::debug!("skipping nested apply_settings_window_to_state invocation");
        return;
    };

    crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(settings_window) = guard.as_ref() else {
            return;
        };
        let mut state_guard = state.borrow_mut();
        let previous_settings = state_guard.settings.clone();
        let prev_dock_edge = previous_settings.dock_edge;
        let prev_language = previous_settings.language;

        let mut next_settings = previous_settings.clone();
        apply_settings_window_changes(settings_window, &mut next_settings);
        apply_runtime_settings_window_changes(settings_window, &mut next_settings);
        next_settings = next_settings.normalized();

        if next_settings == previous_settings {
            return;
        }

        state_guard.settings = next_settings;
        state_guard.current_layout = state_guard.settings.effective_layout();
        let _ = state_guard
            .settings
            .save(state_guard.workspace_name.as_deref());
        let hwnd = state_guard.hwnd;
        let always_on_top = state_guard.settings.always_on_top;
        let new_dock_edge = state_guard.settings.dock_edge;
        let new_language = state_guard.settings.language;
        let locale_changed = prev_language != new_language;
        let settings_clone = state_guard.settings.clone();
        let workspace_name = state_guard.workspace_name.clone();

        if prev_dock_edge != new_dock_edge {
            if state_guard.is_appbar {
                unregister_appbar(hwnd);
                state_guard.is_appbar = false;
            }
            if new_dock_edge.is_some() {
                apply_dock_mode(&mut state_guard);
            } else {
                restore_floating_style(hwnd);
            }
        } else if state_guard.is_appbar {
            reposition_appbar(&mut state_guard);
        }

        drop(state_guard);
        startup::sync_run_at_startup(settings_clone.run_at_startup, workspace_name.as_deref());
        global_hotkey::sync_activate_hotkey(hwnd, &settings_clone);
        let _ = crate::refresh_windows(state);
        if locale_changed {
            let _ = panopticon::i18n::set_locale(new_language);
            if let Some(main_window) = main_weak.upgrade() {
                crate::populate_tr_global(&main_window);
            }
            refresh_open_about_window(state);
            refresh_open_tag_dialog_window(state);
            refresh_tray_locale(state);
        }
        apply_window_appearance(hwnd, &settings_clone);
        apply_topmost_mode(hwnd, always_on_top);
        settings_window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
        settings_window.set_current_profile_label(SharedString::from(current_workspace_label(
            workspace_name.as_deref(),
        )));
        {
            let refreshed = state.borrow();
            sync_settings_window_from_state(settings_window, &refreshed);
        }
        if let Some(main_window) = main_weak.upgrade() {
            let _ = apply_configured_main_window_size(&main_window, &settings_clone);
            crate::recompute_and_update_ui(state, &main_window);
        }

        crate::TAG_DIALOG_WIN.with(|dialog| {
            if let Some(dialog) = dialog.borrow().as_ref() {
                if let Some(dialog_hwnd) = crate::get_hwnd(dialog.window()) {
                    keep_dialog_above_owner(dialog_hwnd, hwnd, &settings_clone);
                }
            }
        });
    });
}

pub(crate) fn refresh_open_settings_window(state: &Rc<RefCell<AppState>>) {
    crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let Ok(state) = state.try_borrow() else {
            tracing::debug!("skipping settings window refresh while app state is busy");
            return;
        };
        sync_settings_window_from_state(window, &state);
        if let Some(dialog_hwnd) = crate::get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
        }
    });
}

pub(crate) fn open_about_window(state: &Rc<RefCell<AppState>>) {
    let already_open = crate::ABOUT_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                apply_window_icons(hwnd, &state.icons);
                keep_dialog_above_owner(hwnd, state.hwnd, &state.settings);
                center_window_on_owner_monitor(hwnd, state.hwnd);
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
    crate::populate_tr_global(&about_window);

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

    if let Some(about_hwnd) = crate::get_hwnd(about_window.window()) {
        let state = state.borrow();
        apply_window_icons(about_hwnd, &state.icons);
        apply_window_appearance(about_hwnd, &state.settings);
        apply_about_window_theme_snapshot(&about_window, &state.current_theme);
        keep_dialog_above_owner(about_hwnd, state.hwnd, &state.settings);
        center_window_on_owner_monitor(about_hwnd, state.hwnd);
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
        if let Some(dialog_hwnd) = crate::get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
            center_window_on_owner_monitor(dialog_hwnd, state.hwnd);
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
            if let Some(dialog_hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                apply_window_icons(dialog_hwnd, &state.icons);
                keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
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
    crate::populate_tr_global(&dialog);

    dialog.set_app_label(SharedString::from(info.app_label()));
    dialog.set_tag_name(SharedString::from(suggested_name));
    dialog.set_color_index(tag_color_index(&suggested_color));
    {
        let state = state.borrow();
        apply_tag_dialog_theme_snapshot(&dialog, &state.current_theme);
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

    if let Some(dialog_hwnd) = crate::get_hwnd(dialog.window()) {
        let state = state.borrow();
        apply_window_icons(dialog_hwnd, &state.icons);
        apply_window_appearance(dialog_hwnd, &state.settings);
        apply_tag_dialog_theme_snapshot(&dialog, &state.current_theme);
        keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
    }

    crate::TAG_DIALOG_WIN.with(|dialog_cell| *dialog_cell.borrow_mut() = Some(dialog));
}

fn apply_runtime_settings_window_changes(window: &SettingsWindow, settings: &mut AppSettings) {
    let monitor = selected_model_value(
        &window.get_monitor_filter_options(),
        window.get_monitor_filter_index(),
    );
    settings.set_monitor_filter(
        monitor
            .as_deref()
            .filter(|value| *value != panopticon::i18n::t("tray.all_monitors")),
    );

    let tag = selected_model_value(
        &window.get_tag_filter_options(),
        window.get_tag_filter_index(),
    );
    settings.set_tag_filter(
        tag.as_deref()
            .filter(|value| *value != panopticon::i18n::t("tray.all_tags")),
    );

    let app = selected_model_value(
        &window.get_app_filter_options(),
        window.get_app_filter_index(),
    )
    .and_then(|value| parse_option_value(&value));

    if let Some(app) = app.as_deref() {
        settings.set_tag_filter(None);
        settings.set_app_filter(Some(app));
    } else {
        settings.set_app_filter(None);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "runtime settings population intentionally centralizes all combo/model synchronization"
)]
fn populate_settings_window_runtime_fields(window: &SettingsWindow, state: &AppState) {
    let runtime = collect_runtime_ui_options(state);
    let app_rule_entries = collect_app_rule_entries(state, &runtime);
    let workspaces = available_workspace_options();
    let fallback_fixed_width = u32::try_from(state.last_size.0)
        .ok()
        .filter(|value| *value > 0)
        .map_or(MIN_FIXED_WINDOW_WIDTH, |value| {
            value.max(MIN_FIXED_WINDOW_WIDTH)
        });
    let fallback_fixed_height = u32::try_from(state.last_size.1)
        .ok()
        .filter(|value| *value > 0)
        .map_or(MIN_FIXED_WINDOW_HEIGHT, |value| {
            value.max(MIN_FIXED_WINDOW_HEIGHT)
        });

    window.set_theme_options(build_string_model(theme_catalog::theme_labels()));
    window.set_app_version_text(SharedString::from(state.app_version.clone()));
    window.set_update_status_text(SharedString::from(localized_update_status_text(
        &state.update_status,
    )));
    window.set_update_check_running(matches!(state.update_status, crate::UpdateStatus::Checking));
    window.set_fixed_width_value(
        state
            .settings
            .fixed_width
            .unwrap_or(fallback_fixed_width)
            .max(MIN_FIXED_WINDOW_WIDTH) as i32,
    );
    window.set_fixed_height_value(
        state
            .settings
            .fixed_height
            .unwrap_or(fallback_fixed_height)
            .max(MIN_FIXED_WINDOW_HEIGHT) as i32,
    );
    window.set_dock_column_thickness_value(
        state
            .settings
            .dock_column_thickness
            .unwrap_or(MIN_DOCK_COLUMN_THICKNESS)
            .max(MIN_DOCK_COLUMN_THICKNESS) as i32,
    );
    window.set_dock_row_thickness_value(
        state
            .settings
            .dock_row_thickness
            .unwrap_or(MIN_DOCK_ROW_THICKNESS)
            .max(MIN_DOCK_ROW_THICKNESS) as i32,
    );
    window.set_current_profile_label(SharedString::from(current_workspace_label(
        state.workspace_name.as_deref(),
    )));
    window.set_profile_name(SharedString::from(
        state.workspace_name.clone().unwrap_or_default(),
    ));
    window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
    let selected_workspace_label = current_workspace_label(state.workspace_name.as_deref());
    let profile_index = workspaces
        .iter()
        .position(|workspace| workspace == &selected_workspace_label)
        .map_or(0, |index| index as i32);
    window.set_available_profile_options(build_string_model(workspaces));
    window.set_available_profile_index(profile_index);

    let mut monitor_options = vec![panopticon::i18n::t("tray.all_monitors").to_owned()];
    monitor_options.extend(runtime.monitors.iter().cloned());
    let monitor_index = state
        .settings
        .active_monitor_filter
        .as_deref()
        .and_then(|current| {
            runtime
                .monitors
                .iter()
                .position(|monitor| monitor == current)
        })
        .map_or(0, |index| index as i32 + 1);
    window.set_monitor_filter_options(build_string_model(monitor_options));
    window.set_monitor_filter_index(monitor_index);

    let mut tag_options = vec![panopticon::i18n::t("tray.all_tags").to_owned()];
    tag_options.extend(runtime.tags.iter().cloned());
    let tag_index = state
        .settings
        .active_tag_filter
        .as_deref()
        .and_then(|current| runtime.tags.iter().position(|tag| tag == current))
        .map_or(0, |index| index as i32 + 1);
    window.set_tag_filter_options(build_string_model(tag_options));
    window.set_tag_filter_index(tag_index);

    let mut app_options = vec![panopticon::i18n::t("tray.all_apps").to_owned()];
    app_options.extend(runtime.apps.iter().map(app_option_label));
    let app_index = state
        .settings
        .active_app_filter
        .as_deref()
        .and_then(|current| runtime.apps.iter().position(|app| app.app_id == current))
        .map_or(0, |index| index as i32 + 1);
    window.set_app_filter_options(build_string_model(app_options));
    window.set_app_filter_index(app_index);

    let previous_app_rule_selection = selected_model_value(
        &window.get_app_rules_options(),
        window.get_app_rules_index(),
    )
    .and_then(|value| parse_option_value(&value));

    let app_rule_search = window.get_app_rules_search().trim().to_ascii_lowercase();
    let app_rule_filter = window.get_app_rules_filter_index();
    let filtered_app_rule_entries =
        filter_app_rule_entries(app_rule_entries, app_rule_filter, app_rule_search.as_str());

    let mut app_rule_options =
        vec![panopticon::i18n::t("settings.app_rules.select_option").to_owned()];
    app_rule_options.extend(
        filtered_app_rule_entries
            .iter()
            .map(|entry| app_option_label(&entry.option)),
    );
    let app_rule_index = previous_app_rule_selection
        .as_deref()
        .and_then(|selected| {
            filtered_app_rule_entries
                .iter()
                .position(|entry| entry.option.app_id == selected)
        })
        .map_or(0, |index| index as i32 + 1);
    window.set_app_rules_options(build_string_model(app_rule_options));
    window.set_app_rules_index(app_rule_index);

    let running_app_ids: BTreeSet<&str> =
        runtime.apps.iter().map(|app| app.app_id.as_str()).collect();
    let inactive_rule_count = state
        .settings
        .app_rules
        .keys()
        .filter(|app_id| !running_app_ids.contains(app_id.as_str()))
        .count();
    let cleanup_summary = if inactive_rule_count == 0 {
        panopticon::i18n::t("settings.app_rules.cleanup.none").to_owned()
    } else {
        panopticon::i18n::t_fmt(
            "settings.app_rules.cleanup.count",
            &inactive_rule_count.to_string(),
        )
    };
    window.set_app_rules_can_clear_unused(inactive_rule_count > 0);
    window.set_app_rules_unused_summary(SharedString::from(cleanup_summary));

    sync_selected_app_rule_editor(window, &state.settings);

    if runtime.hidden_apps.is_empty() {
        window.set_hidden_app_options(build_string_model(vec![panopticon::i18n::t(
            "settings.no_hidden",
        )
        .to_owned()]));
        window.set_hidden_app_index(0);
        window.set_can_restore_hidden(false);
        window.set_hidden_apps_summary(SharedString::from(panopticon::i18n::t(
            "settings.no_hidden",
        )));
    } else {
        let hidden_options: Vec<String> = runtime
            .hidden_apps
            .iter()
            .map(hidden_app_option_label)
            .collect();
        let summary = if runtime.hidden_apps.len() == 1 {
            panopticon::i18n::t("settings.hidden_one").to_owned()
        } else {
            panopticon::i18n::t_fmt(
                "settings.hidden_many",
                &runtime.hidden_apps.len().to_string(),
            )
        };
        window.set_hidden_app_options(build_string_model(hidden_options));
        window.set_hidden_app_index(0);
        window.set_can_restore_hidden(true);
        window.set_hidden_apps_summary(SharedString::from(summary));
    }
}

fn collect_app_rule_entries(state: &AppState, runtime: &RuntimeUiOptions) -> Vec<AppRuleListEntry> {
    let mut by_id: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    for app in &runtime.apps {
        by_id.insert(app.app_id.clone(), app.label.clone());
    }

    for (app_id, rule) in &state.settings.app_rules {
        if app_id.trim().is_empty() {
            continue;
        }
        by_id.entry(app_id.clone()).or_insert_with(|| {
            if rule.display_name.trim().is_empty() {
                app_id.clone()
            } else {
                rule.display_name.clone()
            }
        });
    }

    let mut entries: Vec<AppRuleListEntry> = by_id
        .into_iter()
        .map(|(app_id, label)| {
            let rule = state.settings.app_rules.get(&app_id);
            let tags = state.settings.tags_for(&app_id);
            let searchable_blob = format!(
                "{} {} {}",
                label.to_ascii_lowercase(),
                app_id.to_ascii_lowercase(),
                tags.join(" ").to_ascii_lowercase()
            );

            AppRuleListEntry {
                option: AppSelectionEntry {
                    app_id: app_id.clone(),
                    label,
                },
                is_running: runtime.apps.iter().any(|app| app.app_id == app_id),
                has_saved_rule: rule.is_some(),
                is_hidden: rule.is_some_and(|saved| saved.hidden),
                has_tags: !tags.is_empty(),
                has_custom_refresh: rule.is_some_and(|saved| {
                    matches!(
                        saved.thumbnail_refresh_mode,
                        ThumbnailRefreshMode::Frozen | ThumbnailRefreshMode::Interval
                    )
                }),
                is_pinned: rule.is_some_and(|saved| saved.pinned_position.is_some()),
                searchable_blob,
            }
        })
        .collect();

    entries.sort_by(|left, right| {
        left.option
            .label
            .to_ascii_lowercase()
            .cmp(&right.option.label.to_ascii_lowercase())
            .then_with(|| left.option.app_id.cmp(&right.option.app_id))
    });
    entries
}

fn filter_app_rule_entries(
    entries: Vec<AppRuleListEntry>,
    filter_index: i32,
    search_query: &str,
) -> Vec<AppRuleListEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            let matches_filter = match filter_index {
                1 => entry.is_running,
                2 => entry.has_saved_rule,
                3 => entry.is_hidden,
                4 => entry.has_tags,
                5 => entry.has_custom_refresh,
                6 => entry.is_pinned,
                _ => true,
            };

            if !matches_filter {
                return false;
            }

            if search_query.is_empty() {
                return true;
            }

            entry.searchable_blob.contains(search_query)
        })
        .collect()
}

fn sync_selected_app_rule_editor(window: &SettingsWindow, settings: &AppSettings) {
    let selected = selected_model_value(
        &window.get_app_rules_options(),
        window.get_app_rules_index(),
    );
    let Some(selected_option) = selected else {
        clear_app_rule_editor(window);
        return;
    };
    let Some(app_id) = parse_option_value(&selected_option) else {
        clear_app_rule_editor(window);
        return;
    };

    let label = selected_option
        .split_once(OPTION_SEPARATOR)
        .map_or_else(|| app_id.clone(), |(display, _)| display.trim().to_owned());
    let tags = settings.tags_for(&app_id).join(", ");
    let tags_vec = settings.tags_for(&app_id);
    let color_hex = settings.app_color_hex(&app_id).unwrap_or_default();

    window.set_app_rules_has_selection(true);
    window.set_app_rules_selected_app_label(SharedString::from(label));
    window.set_app_rules_hidden(settings.is_hidden(&app_id));
    window.set_app_rules_preserve_aspect(settings.preserve_aspect_ratio_for(&app_id));
    window.set_app_rules_hide_on_select(settings.hide_on_select_for(&app_id));
    window.set_app_rules_refresh_mode_index(refresh_mode_to_index(
        settings.thumbnail_refresh_mode_for(&app_id),
    ));
    window.set_app_rules_refresh_interval_ms(
        settings.thumbnail_refresh_interval_ms_for(&app_id) as i32
    );
    window.set_app_rules_tags(SharedString::from(tags));
    sync_app_rule_tags_editor(window, &tags_vec, false);
    window.set_app_rules_color_hex(SharedString::from(color_hex));
}

fn clear_app_rule_editor(window: &SettingsWindow) {
    window.set_app_rules_has_selection(false);
    window.set_app_rules_selected_app_label(SharedString::from(""));
    window.set_app_rules_hidden(false);
    window.set_app_rules_preserve_aspect(false);
    window.set_app_rules_hide_on_select(false);
    window.set_app_rules_refresh_mode_index(0);
    window.set_app_rules_refresh_interval_ms(5_000);
    window.set_app_rules_tags(SharedString::from(""));
    window.set_app_rules_tag_chips(build_string_model(Vec::new()));
    window.set_app_rules_tag_input(SharedString::from(""));
    window.set_app_rules_color_hex(SharedString::from(""));
}

fn sync_app_rule_tags_editor(window: &SettingsWindow, tags: &[String], clear_input: bool) {
    let chips: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();

    window.set_app_rules_tag_chips(build_string_model(chips.clone()));
    window.set_app_rules_tags(SharedString::from(chips.join(", ")));
    if clear_input {
        window.set_app_rules_tag_input(SharedString::from(""));
    }
}

fn refresh_mode_to_index(mode: ThumbnailRefreshMode) -> i32 {
    match mode {
        ThumbnailRefreshMode::Realtime => 0,
        ThumbnailRefreshMode::Frozen => 1,
        ThumbnailRefreshMode::Interval => 2,
    }
}

fn refresh_mode_from_index(index: i32) -> ThumbnailRefreshMode {
    match index {
        1 => ThumbnailRefreshMode::Frozen,
        2 => ThumbnailRefreshMode::Interval,
        _ => ThumbnailRefreshMode::Realtime,
    }
}

fn parse_tags_csv(raw: &str) -> Vec<String> {
    let mut tags: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_ascii_lowercase())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    let draft_profile_name = window.get_profile_name();
    crate::populate_tr_global(window);
    window.set_suspend_live_apply(true);
    populate_settings_window(window, &state.settings);
    populate_settings_window_runtime_fields(window, state);
    let resolved_theme = theme_catalog::resolve_ui_theme(
        state.settings.theme_id.as_deref(),
        &state.settings.background_color_hex,
        &state.settings.theme_color_overrides,
    );
    apply_settings_window_theme_snapshot(window, &resolved_theme);
    if !draft_profile_name.is_empty() {
        window.set_profile_name(draft_profile_name);
    }
    window.set_suspend_live_apply(false);
}

fn collect_runtime_ui_options(state: &AppState) -> RuntimeUiOptions {
    let windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();

    RuntimeUiOptions {
        monitors: collect_available_monitors(&windows),
        tags: state.settings.known_tags(),
        apps: collect_available_apps(&windows),
        hidden_apps: state.settings.hidden_app_entries(),
    }
}

fn sync_about_window_from_state(window: &AboutWindow, state: &AppState) {
    crate::populate_tr_global(window);
    window.set_version_text(SharedString::from(state.app_version.clone()));
    let (show_update_badge, latest_version_text) = match &state.update_status {
        crate::UpdateStatus::Available { latest_version, .. } => (true, latest_version.clone()),
        _ => (false, String::new()),
    };
    window.set_show_update_badge(show_update_badge);
    window.set_latest_version_text(SharedString::from(latest_version_text));
    apply_about_window_theme_snapshot(window, &state.current_theme);
}

fn known_workspaces_label() -> String {
    use panopticon::i18n;
    match AppSettings::list_workspaces_with_default() {
        Ok(workspaces) if workspaces.is_empty() => {
            i18n::t("settings.no_saved_workspaces").to_owned()
        }
        Ok(workspaces) => i18n::t_fmt("settings.saved_workspaces_fmt", &workspaces.join(", ")),
        Err(error) => {
            tracing::warn!(%error, "failed to list saved workspaces");
            i18n::t("settings.no_saved_workspaces").to_owned()
        }
    }
}

fn localized_update_status_text(status: &crate::UpdateStatus) -> String {
    match status {
        crate::UpdateStatus::Idle => panopticon::i18n::t("settings.update_status.idle").to_owned(),
        crate::UpdateStatus::Checking => {
            panopticon::i18n::t("settings.update_status.checking").to_owned()
        }
        crate::UpdateStatus::UpToDate { latest_version } => {
            panopticon::i18n::t_fmt("settings.update_status.up_to_date", latest_version)
        }
        crate::UpdateStatus::Available { latest_version, .. } => {
            panopticon::i18n::t_fmt("settings.update_status.available", latest_version)
        }
        crate::UpdateStatus::Failed => {
            panopticon::i18n::t("settings.update_status.failed").to_owned()
        }
    }
}

pub(crate) fn load_workspace_into_current_instance(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    requested_workspace: Option<String>,
) -> bool {
    let loaded_settings = match AppSettings::load_or_default(requested_workspace.as_deref()) {
        Ok(settings) => settings.normalized(),
        Err(error) => {
            tracing::error!(%error, workspace = ?requested_workspace, "failed to load workspace");
            return false;
        }
    };

    let (hwnd, previous_language, settings_snapshot, workspace_name) = {
        let mut guard = state.borrow_mut();
        let previous_language = guard.settings.language;
        if guard.is_appbar {
            unregister_appbar(guard.hwnd);
            guard.is_appbar = false;
        }
        guard.workspace_name = requested_workspace;
        guard.settings = loaded_settings;
        guard.current_layout = guard.settings.effective_layout();
        guard.loaded_background_path = None;
        guard.current_theme = theme_catalog::resolve_ui_theme(
            guard.settings.theme_id.as_deref(),
            &guard.settings.background_color_hex,
            &guard.settings.theme_color_overrides,
        );
        guard.theme_animation = None;
        (
            guard.hwnd,
            previous_language,
            guard.settings.clone(),
            guard.workspace_name.clone(),
        )
    };

    startup::sync_run_at_startup(settings_snapshot.run_at_startup, workspace_name.as_deref());
    global_hotkey::sync_activate_hotkey(hwnd, &settings_snapshot);
    apply_window_appearance(hwnd, &settings_snapshot);

    if let Some(main_window) = main_weak.upgrade() {
        if settings_snapshot.dock_edge.is_some() {
            let mut guard = state.borrow_mut();
            apply_dock_mode(&mut guard);
        } else {
            restore_floating_style(hwnd);
            apply_topmost_mode(hwnd, settings_snapshot.always_on_top);
            let _ = apply_configured_main_window_size(&main_window, &settings_snapshot);
            center_window_on_owner_monitor(hwnd, HWND::default());
        }

        apply_main_window_theme_snapshot(&main_window, &state.borrow().current_theme);
        let _ = crate::refresh_windows(state);
        crate::recompute_and_update_ui(state, &main_window);
    }

    if previous_language != settings_snapshot.language {
        let _ = panopticon::i18n::set_locale(settings_snapshot.language);
        if let Some(main_window) = main_weak.upgrade() {
            crate::populate_tr_global(&main_window);
        }
        refresh_open_about_window(state);
        refresh_open_tag_dialog_window(state);
        refresh_tray_locale(state);
    }

    refresh_open_settings_window(state);
    true
}

fn refresh_open_tag_dialog_window(state: &Rc<RefCell<AppState>>) {
    crate::TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        crate::populate_tr_global(window);
        let state = state.borrow();
        apply_tag_dialog_theme_snapshot(window, &state.current_theme);
        if let Some(dialog_hwnd) = crate::get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
        }
    });
}

fn refresh_tray_locale(state: &Rc<RefCell<AppState>>) {
    let mut state = state.borrow_mut();
    let icon = state.icons.small;
    if let Some(tray) = state.tray_icon.as_mut() {
        tray.refresh(icon);
    }
}

fn build_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let values: Vec<SharedString> = values.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(values))
}

fn selected_model_value(model: &ModelRc<SharedString>, index: i32) -> Option<String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.row_data(index))
        .map(|value| value.to_string())
}

fn save_settings_as_workspace(settings: &AppSettings, workspace_name: &str) -> bool {
    match settings.save(Some(workspace_name)) {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, workspace = workspace_name, "failed to save workspace");
            false
        }
    }
}

fn launch_additional_instance(workspace_name: Option<&str>) -> bool {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            tracing::error!(%error, "failed to resolve executable path for new instance");
            return false;
        }
    };

    let mut command = Command::new(executable);
    if let Some(workspace_name) = workspace_name {
        command.arg("--workspace").arg(workspace_name);
    }

    match command.spawn() {
        Ok(_) => true,
        Err(error) => {
            tracing::error!(%error, workspace = ?workspace_name, "failed to launch extra instance");
            false
        }
    }
}

fn apply_tag_creation(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    app_id: &str,
    display_name: &str,
    tag_name: &str,
    color_hex: &str,
) {
    crate::update_settings(state, |settings| {
        let _ = settings.assign_tag_with_color(app_id, display_name, tag_name, color_hex);
    });
    let _ = crate::refresh_windows(state);
    crate::refresh_ui(state, weak);
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

fn apply_background_color(window: &SettingsWindow, red: i32, green: i32, blue: i32) {
    let already_syncing = BG_COLOR_SYNC_IN_PROGRESS.with(|flag| {
        if flag.get() {
            true
        } else {
            flag.set(true);
            false
        }
    });
    if already_syncing {
        tracing::debug!("skipping re-entrant background color sync");
        return;
    }

    let red = clamp_rgb(red);
    let green = clamp_rgb(green);
    let blue = clamp_rgb(blue);
    let hex = format!("{red:02X}{green:02X}{blue:02X}");

    window.set_bg_red_value(red);
    window.set_bg_green_value(green);
    window.set_bg_blue_value(blue);
    window.set_bg_color_hex(SharedString::from(hex));
    window.set_bg_preview_color(slint::Color::from_rgb_u8(
        red as u8,
        green as u8,
        blue as u8,
    ));
    if !window.get_suspend_live_apply() {
        window.invoke_apply();
    }

    BG_COLOR_SYNC_IN_PROGRESS.with(|flag| flag.set(false));
}

fn clamp_rgb(value: i32) -> i32 {
    value.clamp(0, 255)
}

fn parse_rgb_hex(input: &str) -> Option<(i32, i32, i32)> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let red = i32::from(u8::from_str_radix(&hex[0..2], 16).ok()?);
    let green = i32::from(u8::from_str_radix(&hex[2..4], 16).ok()?);
    let blue = i32::from(u8::from_str_radix(&hex[4..6], 16).ok()?);
    Some((red, green, blue))
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
