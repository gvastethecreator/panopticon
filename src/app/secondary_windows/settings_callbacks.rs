use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use panopticon::settings::{AppSettings, ThumbnailRefreshMode};
use panopticon::ui_option_ops::{current_workspace_label, parse_option_value};
use slint::{ComponentHandle, SharedString, Timer, TimerMode};

use crate::{AppState, MainWindow, SettingsWindow};

use super::global_hotkey;
use super::startup;
use super::workspace;
use super::{
    apply_background_color, apply_configured_main_window_size, apply_recorded_shortcut_binding,
    apply_settings_window_to_state, apply_topmost_mode, apply_window_appearance,
    clear_workspace_feedback, collect_runtime_ui_options, known_workspaces_label,
    load_workspace_into_current_instance, normalize_recorded_shortcut, open_about_window,
    parse_rgb_hex, parse_tags_csv, parse_workspace_target_input,
    populate_settings_window_runtime_fields, refresh_mode_from_index,
    select_workspace_in_settings_window, selected_layout_preset_name, selected_model_value,
    selected_workspace_from_settings_window, set_layout_preset_summary, set_workspace_feedback,
    shortcut_recording_label, stop_shortcut_recording, sync_app_rule_tags_editor,
    sync_layout_preset_controls, sync_selected_app_rule_editor, sync_settings_window_from_state,
    sync_workspace_editor_from_selection,
};

#[expect(
    clippy::too_many_lines,
    reason = "SettingsWindow wiring keeps all generated callback registrations together in one dedicated module while the next refactor pass splits domains more finely"
)]
pub(super) fn register_settings_window_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_save_profile({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let requested_workspace = if settings_window.get_profile_name().trim().is_empty() {
                    state.borrow().workspace_name.clone()
                } else {
                    match parse_workspace_target_input(&settings_window.get_profile_name()) {
                        Ok(workspace_name) => workspace_name,
                        Err(reason) => {
                            tracing::warn!(%reason, "ignoring invalid workspace save request");
                            set_workspace_feedback(settings_window, &reason, true);
                            return;
                        }
                    }
                };

                let display_name = settings_window.get_profile_display_name().to_string();
                let description = settings_window.get_profile_description().to_string();

                let settings_snapshot = {
                    let mut state_guard = state.borrow_mut();
                    state_guard
                        .settings
                        .set_workspace_metadata(&display_name, &description);
                    state_guard.settings = state_guard.settings.normalized();
                    state_guard.settings.clone()
                };

                match settings_snapshot.save(requested_workspace.as_deref()) {
                    Ok(()) => {
                        settings_window
                            .set_known_profiles_label(SharedString::from(known_workspaces_label()));
                        let feedback = format!(
                            "Workspace {} saved successfully.",
                            current_workspace_label(requested_workspace.as_deref())
                        );
                        set_workspace_feedback(settings_window, &feedback, false);
                        let state_guard = state.borrow();
                        sync_settings_window_from_state(settings_window, &state_guard);
                    }
                    Err(error) => {
                        tracing::error!(
                            %error,
                            workspace = ?requested_workspace,
                            "failed to save workspace snapshot"
                        );
                        set_workspace_feedback(
                            settings_window,
                            "Failed to save workspace snapshot. Check logs for details.",
                            true,
                        );
                    }
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
                let requested = if settings_window.get_profile_name().trim().is_empty() {
                    current_workspace
                } else {
                    match parse_workspace_target_input(&settings_window.get_profile_name()) {
                        Ok(workspace_name) => workspace_name,
                        Err(reason) => {
                            tracing::warn!(%reason, "ignoring invalid extra-instance workspace request");
                            set_workspace_feedback(settings_window, &reason, true);
                            return;
                        }
                    }
                };

                let display_name = settings_window.get_profile_display_name().to_string();
                let description = settings_window.get_profile_description().to_string();
                let settings_snapshot = {
                    let mut state_guard = state.borrow_mut();
                    state_guard
                        .settings
                        .set_workspace_metadata(&display_name, &description);
                    state_guard.settings = state_guard.settings.normalized();
                    state_guard.settings.clone()
                };
                if let Some(workspace_name) = requested.as_deref() {
                    let _ = workspace::save_settings_as_workspace(&settings_snapshot, workspace_name);
                } else if let Err(error) = settings_snapshot.save(None) {
                    tracing::error!(%error, "failed to save default workspace before launching instance");
                }

                let launched = workspace::launch_additional_instance(requested.as_deref());
                settings_window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
                if launched {
                    let feedback = format!(
                        "Opened a new instance for {}.",
                        current_workspace_label(requested.as_deref())
                    );
                    set_workspace_feedback(settings_window, &feedback, false);
                } else {
                    set_workspace_feedback(
                        settings_window,
                        "Could not open a new instance for this workspace.",
                        true,
                    );
                }
            });
        }
    });

    settings_window.on_profile_selection_changed({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                clear_workspace_feedback(settings_window);
                let state_guard = state.borrow();
                let fallback_workspace = state_guard.workspace_name.clone();
                sync_workspace_editor_from_selection(
                    settings_window,
                    fallback_workspace,
                    &state_guard,
                );
            });
        }
    });

    settings_window.on_duplicate_selected_profile({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let source_workspace = selected_workspace_from_settings_window(settings_window)
                    .or_else(|| state.borrow().workspace_name.clone());
                let target_workspace = match parse_workspace_target_input(&settings_window.get_profile_name()) {
                    Ok(Some(workspace_name)) => workspace_name,
                    Ok(None) => {
                        tracing::warn!("duplicate requires a non-default target workspace name");
                        set_workspace_feedback(
                            settings_window,
                            "Duplicate requires a non-default workspace name.",
                            true,
                        );
                        return;
                    }
                    Err(reason) => {
                        tracing::warn!(%reason, "ignoring invalid duplicate workspace request");
                        set_workspace_feedback(settings_window, &reason, true);
                        return;
                    }
                };

                if let Err(error) = AppSettings::duplicate_workspace(source_workspace.as_deref(), &target_workspace) {
                    tracing::error!(%error, source = ?source_workspace, target = %target_workspace, "failed to duplicate workspace");
                    set_workspace_feedback(
                        settings_window,
                        "Failed to duplicate workspace. Check logs for details.",
                        true,
                    );
                    return;
                }

                if let Ok(mut duplicated) = AppSettings::load_or_default(Some(&target_workspace)) {
                    duplicated.set_workspace_metadata(
                        &settings_window.get_profile_display_name(),
                        &settings_window.get_profile_description(),
                    );
                    if let Err(error) = duplicated.save(Some(&target_workspace)) {
                        tracing::warn!(%error, workspace = %target_workspace, "failed to persist duplicated workspace metadata");
                    }
                }

                let fallback_workspace = {
                    let state_guard = state.borrow();
                    sync_settings_window_from_state(settings_window, &state_guard);
                    state_guard.workspace_name.clone()
                };
                select_workspace_in_settings_window(settings_window, Some(&target_workspace));
                let state_guard = state.borrow();
                sync_workspace_editor_from_selection(settings_window, fallback_workspace, &state_guard);
                let feedback = format!(
                    "Workspace duplicated into {}.",
                    current_workspace_label(Some(&target_workspace))
                );
                set_workspace_feedback(settings_window, &feedback, false);
            });
        }
    });

    settings_window.on_rename_selected_profile({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let Some(source_workspace) = selected_workspace_from_settings_window(settings_window) else {
                    tracing::warn!("default workspace cannot be renamed");
                    set_workspace_feedback(settings_window, "Default workspace cannot be renamed.", true);
                    return;
                };

                let target_workspace = match parse_workspace_target_input(&settings_window.get_profile_name()) {
                    Ok(Some(workspace_name)) => workspace_name,
                    Ok(None) => {
                        tracing::warn!("rename requires a non-default target workspace name");
                        set_workspace_feedback(
                            settings_window,
                            "Rename requires a non-default workspace name.",
                            true,
                        );
                        return;
                    }
                    Err(reason) => {
                        tracing::warn!(%reason, "ignoring invalid rename workspace request");
                        set_workspace_feedback(settings_window, &reason, true);
                        return;
                    }
                };

                if source_workspace == target_workspace {
                    set_workspace_feedback(
                        settings_window,
                        "Source and target workspace names are identical.",
                        true,
                    );
                    return;
                }

                let confirmation_message =
                    format!("Rename workspace '{source_workspace}' to '{target_workspace}' ?");
                if !super::confirm_workspace_action("Rename workspace", &confirmation_message) {
                    set_workspace_feedback(settings_window, "Rename cancelled.", false);
                    return;
                }

                if let Err(error) = AppSettings::rename_workspace(&source_workspace, &target_workspace) {
                    tracing::error!(%error, source = %source_workspace, target = %target_workspace, "failed to rename workspace");
                    set_workspace_feedback(
                        settings_window,
                        "Failed to rename workspace. Check logs for details.",
                        true,
                    );
                    return;
                }

                if let Ok(mut renamed) = AppSettings::load_or_default(Some(&target_workspace)) {
                    renamed.set_workspace_metadata(
                        &settings_window.get_profile_display_name(),
                        &settings_window.get_profile_description(),
                    );
                    if let Err(error) = renamed.save(Some(&target_workspace)) {
                        tracing::warn!(%error, workspace = %target_workspace, "failed to persist renamed workspace metadata");
                    }
                }

                let should_switch_current = state
                    .borrow()
                    .workspace_name
                    .as_deref()
                    .is_some_and(|workspace| workspace == source_workspace.as_str());
                if should_switch_current {
                    state.borrow_mut().workspace_name = Some(target_workspace.clone());
                }

                let fallback_workspace = {
                    let state_guard = state.borrow();
                    sync_settings_window_from_state(settings_window, &state_guard);
                    state_guard.workspace_name.clone()
                };
                select_workspace_in_settings_window(settings_window, Some(&target_workspace));
                let state_guard = state.borrow();
                sync_workspace_editor_from_selection(settings_window, fallback_workspace, &state_guard);
                let feedback = format!(
                    "Workspace renamed to {}.",
                    current_workspace_label(Some(&target_workspace))
                );
                set_workspace_feedback(settings_window, &feedback, false);
            });
        }
    });

    settings_window.on_delete_selected_profile({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let Some(selected_workspace) = selected_workspace_from_settings_window(settings_window) else {
                    tracing::warn!("default workspace cannot be deleted");
                    set_workspace_feedback(settings_window, "Default workspace cannot be deleted.", true);
                    return;
                };

                let confirmation_message =
                    format!("Delete workspace '{selected_workspace}' ? This action cannot be undone.");
                if !super::confirm_workspace_action("Delete workspace", &confirmation_message) {
                    set_workspace_feedback(settings_window, "Delete cancelled.", false);
                    return;
                }

                if let Err(error) = AppSettings::delete_workspace(&selected_workspace) {
                    tracing::error!(%error, workspace = %selected_workspace, "failed to delete workspace");
                    set_workspace_feedback(
                        settings_window,
                        "Failed to delete workspace. Check logs for details.",
                        true,
                    );
                    return;
                }

                let deleted_current = state
                    .borrow()
                    .workspace_name
                    .as_deref()
                    .is_some_and(|workspace| workspace == selected_workspace);

                if deleted_current {
                    let _ = load_workspace_into_current_instance(&state, &main_weak, None);
                }

                let fallback_workspace = {
                    let state_guard = state.borrow();
                    sync_settings_window_from_state(settings_window, &state_guard);
                    state_guard.workspace_name.clone()
                };
                select_workspace_in_settings_window(settings_window, fallback_workspace.as_deref());
                let state_guard = state.borrow();
                sync_workspace_editor_from_selection(settings_window, fallback_workspace, &state_guard);
                let feedback = format!("Workspace '{selected_workspace}' deleted.");
                set_workspace_feedback(settings_window, &feedback, false);
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
                let loaded =
                    load_workspace_into_current_instance(&state, &main_weak, requested.clone());
                crate::SETTINGS_WIN.with(|handle| {
                    let guard = handle.borrow();
                    let Some(settings_window) = guard.as_ref() else {
                        return;
                    };

                    if loaded {
                        let feedback = format!(
                            "Loaded {} in this instance.",
                            current_workspace_label(requested.as_deref())
                        );
                        set_workspace_feedback(settings_window, &feedback, false);
                    } else {
                        set_workspace_feedback(
                            settings_window,
                            "Failed to load selected workspace.",
                            true,
                        );
                    }
                });
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

    settings_window.on_shortcut_stop_recording(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };
            stop_shortcut_recording(settings_window, "Shortcut recording stopped.");
        });
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
                let pin_slot_value = settings_window.get_app_rules_pin_slot().max(0) as usize;

                crate::update_settings(&state, |settings| {
                    let default_preserve = settings.preserve_aspect_ratio;
                    let default_hide = settings.hide_on_select;
                    let rule = settings.app_rules.entry(app_id.clone()).or_default();

                    if !display_name.trim().is_empty() {
                        display_name.trim().clone_into(&mut rule.display_name);
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

                    rule.pinned_position = if hidden || pin_slot_value == 0 {
                        None
                    } else {
                        Some(pin_slot_value - 1)
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

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
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

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
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

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
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
                    let active_layout = state_guard.current_layout;
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
                        state_guard.current_layout = state_guard.settings.effective_layout();
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
                        let _ = crate::refresh_windows(&state);
                        crate::refresh_ui(&state, &main_weak);
                    }
                }
            });
        }
    });

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
                super::super::keyboard_actions::handle_key(
                    &state,
                    &main_weak,
                    &key_text,
                    shift_pressed,
                )
            }
        }
    });

    settings_window.on_closed(|| {
        let taken = crate::SETTINGS_WIN.with(|handle| handle.borrow_mut().take());
        if let Some(window) = taken {
            window.hide().ok();
        }
    });
}
