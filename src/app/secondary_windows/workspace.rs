use std::cell::RefCell;
use std::collections::BTreeSet;
use std::process::Command;
use std::rc::Rc;

use panopticon::settings::{AppSettings, WorkspaceNameValidation};
use panopticon::theme as theme_catalog;
use panopticon::ui_option_ops::current_workspace_label;
use slint::{Model, SharedString};
use windows::Win32::Foundation::HWND;

use crate::app::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, center_window_on_owner_monitor,
    restore_floating_style, unregister_appbar,
};
use crate::app::global_hotkey;
use crate::app::native_runtime::apply_configured_main_window_size;
use crate::app::startup;
use crate::app::theme_ui::apply_main_window_theme_snapshot;
use crate::{AppState, MainWindow, SettingsWindow};

use super::dialogs::{refresh_open_about_window, refresh_open_tag_dialog_window};
use super::placement::refresh_secondary_window_stacking;
use super::{collect_runtime_ui_options, refresh_open_settings_window, refresh_tray_locale};
use super::{selected_model_value, RuntimeUiOptions};

#[derive(Debug, Clone)]
pub(super) struct WorkspaceUiSummary {
    pub(super) workspace_name: Option<String>,
    pub(super) option_label: String,
    pub(super) display_name: String,
    pub(super) description: String,
    pub(super) created_at_label: String,
    pub(super) updated_at_label: String,
    pub(super) is_default: bool,
    pub(super) is_running: bool,
    pub(super) is_modified: bool,
    pub(super) missing_apps: usize,
    pub(super) status_summary: String,
}

fn workspace_status_summary(is_running: bool, is_modified: bool, missing_apps: usize) -> String {
    let mut parts = Vec::new();
    if is_running {
        parts.push("Loaded in this instance".to_owned());
    }
    if is_modified {
        parts.push("Unsaved changes".to_owned());
    }
    if missing_apps > 0 {
        let label = if missing_apps == 1 {
            "1 app rule not currently running".to_owned()
        } else {
            format!("{missing_apps} app rules not currently running")
        };
        parts.push(label);
    }

    if parts.is_empty() {
        "Ready".to_owned()
    } else {
        parts.join(" · ")
    }
}

pub(super) fn available_workspace_summaries(
    state: &AppState,
    runtime: &RuntimeUiOptions,
) -> Vec<WorkspaceUiSummary> {
    let workspaces = AppSettings::list_workspaces_with_default().unwrap_or_else(|error| {
        tracing::warn!(%error, "failed to enumerate available workspaces");
        vec!["default".to_owned()]
    });

    let running_apps: BTreeSet<&str> = runtime
        .apps
        .iter()
        .map(|entry| entry.app_id.as_str())
        .collect();
    let current_workspace = state.workspace_name.clone();
    let current_settings = state.settings.normalized();

    workspaces
        .into_iter()
        .map(|option_label| {
            let workspace_name =
                panopticon::ui_option_ops::selected_workspace_name(&option_label);
            let settings = AppSettings::load_or_default(workspace_name.as_deref())
                .unwrap_or_else(|error| {
                    tracing::warn!(%error, workspace = ?workspace_name, "failed to load workspace metadata summary");
                    AppSettings::default()
                });
            let metadata = settings.workspace.clone();

            let display_name = if metadata.display_name.trim().is_empty() {
                option_label.clone()
            } else {
                metadata.display_name.trim().to_owned()
            };

            let missing_apps = settings
                .app_rules
                .keys()
                .filter(|app_id| !running_apps.contains(app_id.as_str()))
                .count();
            let is_running = workspace_name == current_workspace;
            let is_modified = is_running && settings.normalized() != current_settings;
            let status_summary = workspace_status_summary(is_running, is_modified, missing_apps);

            WorkspaceUiSummary {
                is_default: workspace_name.is_none(),
                workspace_name,
                option_label,
                display_name,
                description: metadata.description.trim().to_owned(),
                created_at_label: format_workspace_timestamp(metadata.created_unix_ms),
                updated_at_label: format_workspace_timestamp(metadata.updated_unix_ms),
                is_running,
                is_modified,
                missing_apps,
                status_summary,
            }
        })
        .collect()
}

fn format_workspace_timestamp(value: Option<u64>) -> String {
    value
        .map(|timestamp| format!("unix-ms:{timestamp}"))
        .unwrap_or_default()
}

pub(super) fn parse_workspace_target_input(value: &str) -> Result<Option<String>, String> {
    match panopticon::settings::validate_workspace_name_input(value) {
        WorkspaceNameValidation::Valid(workspace_name)
            if workspace_name.eq_ignore_ascii_case("default") =>
        {
            Ok(None)
        }
        WorkspaceNameValidation::Valid(workspace_name) => Ok(Some(workspace_name)),
        WorkspaceNameValidation::Empty => Ok(None),
        WorkspaceNameValidation::Invalid(reason) => Err(reason),
    }
}

pub(super) fn set_workspace_feedback(window: &SettingsWindow, message: &str, is_error: bool) {
    window.set_workspace_feedback_error(is_error);
    window.set_workspace_feedback_message(SharedString::from(message));
}

pub(super) fn clear_workspace_feedback(window: &SettingsWindow) {
    window.set_workspace_feedback_error(false);
    window.set_workspace_feedback_message(SharedString::from(""));
}

pub(super) fn selected_workspace_from_settings_window(window: &SettingsWindow) -> Option<String> {
    selected_model_value(
        &window.get_available_profile_options(),
        window.get_available_profile_index(),
    )
    .and_then(|value| panopticon::ui_option_ops::selected_workspace_name(&value))
}

pub(super) fn select_workspace_in_settings_window(
    window: &SettingsWindow,
    workspace: Option<&str>,
) {
    let options = window.get_available_profile_options();
    let mut target_index = 0;

    for index in 0..options.row_count() {
        let Some(option) = options.row_data(index) else {
            continue;
        };
        let option_workspace = panopticon::ui_option_ops::selected_workspace_name(option.as_str());
        if option_workspace.as_deref() == workspace {
            target_index = i32::try_from(index).unwrap_or_default();
            break;
        }
    }

    window.set_available_profile_index(target_index);
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

pub(super) fn sync_workspace_editor_from_selection(
    window: &SettingsWindow,
    fallback_workspace: Option<String>,
    state: &AppState,
) {
    let selected_workspace = selected_workspace_from_settings_window(window).or(fallback_workspace);
    let workspace_settings = AppSettings::load_or_default(selected_workspace.as_deref())
        .unwrap_or_else(|error| {
            tracing::warn!(%error, workspace = ?selected_workspace, "failed to load selected workspace metadata");
            AppSettings::default()
        });

    let runtime = collect_runtime_ui_options(state);
    let running_apps: BTreeSet<&str> = runtime
        .apps
        .iter()
        .map(|entry| entry.app_id.as_str())
        .collect();
    let missing_apps = workspace_settings
        .app_rules
        .keys()
        .filter(|app_id| !running_apps.contains(app_id.as_str()))
        .count();
    let is_running = selected_workspace == state.workspace_name;
    let is_modified = is_running && workspace_settings.normalized() != state.settings.normalized();
    let status_summary = workspace_status_summary(is_running, is_modified, missing_apps);

    window.set_profile_name(SharedString::from(
        selected_workspace.clone().unwrap_or_default(),
    ));
    window.set_profile_display_name(SharedString::from(
        workspace_settings.workspace.display_name.clone(),
    ));
    window.set_profile_description(SharedString::from(
        workspace_settings.workspace.description.clone(),
    ));
    window.set_selected_profile_name(SharedString::from(current_workspace_label(
        selected_workspace.as_deref(),
    )));
    window.set_selected_profile_is_default(selected_workspace.is_none());
    window.set_selected_profile_created_at(SharedString::from(format_workspace_timestamp(
        workspace_settings.workspace.created_unix_ms,
    )));
    window.set_selected_profile_updated_at(SharedString::from(format_workspace_timestamp(
        workspace_settings.workspace.updated_unix_ms,
    )));
    window.set_selected_profile_running(is_running);
    window.set_selected_profile_modified(is_modified);
    window.set_selected_profile_missing_apps(missing_apps as i32);
    window.set_selected_profile_status_summary(SharedString::from(status_summary));
}

pub(super) fn known_workspaces_label() -> String {
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
    refresh_secondary_window_stacking(state);
    true
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "call sites naturally own the optional workspace value and this API keeps that flow ergonomic"
)]
pub(crate) fn open_workspace_in_new_instance(
    state: &Rc<RefCell<AppState>>,
    requested_workspace: Option<String>,
) -> bool {
    let settings_snapshot = state.borrow().settings.normalized();

    if let Some(workspace_name) = requested_workspace.as_deref() {
        let _ = save_settings_as_workspace(&settings_snapshot, workspace_name);
    } else if let Err(error) = settings_snapshot.save(None) {
        tracing::error!(%error, "failed to save default workspace before launching instance");
        return false;
    }

    launch_additional_instance(requested_workspace.as_deref())
}

pub(super) fn save_settings_as_workspace(settings: &AppSettings, workspace_name: &str) -> bool {
    match settings.save(Some(workspace_name)) {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, workspace = workspace_name, "failed to save workspace");
            false
        }
    }
}

pub(super) fn launch_additional_instance(workspace_name: Option<&str>) -> bool {
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
