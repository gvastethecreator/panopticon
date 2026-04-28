use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use panopticon::ui_option_ops::current_workspace_label;
use slint::SharedString;

use crate::{AppState, MainWindow, SettingsWindow};

use super::super::workspace;
use super::super::{
    clear_workspace_feedback, confirm_workspace_action, known_workspaces_label,
    load_workspace_into_current_instance, parse_workspace_target_input,
    select_workspace_in_settings_window, selected_workspace_from_settings_window,
    set_workspace_feedback, sync_settings_window_from_state, sync_workspace_editor_from_selection,
};

pub(super) fn register_profile_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    register_save_profile_callback(settings_window, state);
    register_open_profile_instance_callback(settings_window, state);
    register_profile_selection_changed_callback(settings_window, state);
    register_duplicate_selected_profile_callback(settings_window, state);
    register_rename_selected_profile_callback(settings_window, state);
    register_delete_selected_profile_callback(settings_window, state, main_weak);
    register_load_selected_profile_callback(settings_window, state, main_weak);
}

fn register_save_profile_callback(settings_window: &SettingsWindow, state: &Rc<RefCell<AppState>>) {
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
}

fn register_open_profile_instance_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
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
}

fn register_profile_selection_changed_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
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
}

fn register_duplicate_selected_profile_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
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
}

fn register_rename_selected_profile_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
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
                if !confirm_workspace_action("Rename workspace", &confirmation_message) {
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
}

fn register_delete_selected_profile_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
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
                if !confirm_workspace_action("Delete workspace", &confirmation_message) {
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
}

fn register_load_selected_profile_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
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
}
