//! Workspace / profile and hidden-apps builders for the settings window.
//!
//! Owns the profile dropdown model, layout-preset sync, hidden-apps list,
//! and the fixed-size / dock-thickness dimension fields.

use panopticon::settings::{
    MIN_DOCK_COLUMN_THICKNESS, MIN_DOCK_ROW_THICKNESS,
    MIN_FIXED_WINDOW_HEIGHT, MIN_FIXED_WINDOW_WIDTH,
};
use panopticon::theme as theme_catalog;
use panopticon::ui_option_ops::{current_workspace_label, hidden_app_option_label};
use slint::SharedString;

use crate::{AppState, SettingsWindow};
use super::settings_helpers::build_string_model;

/// Populate dimension, theme, version, and update-status fields.
pub(super) fn populate_dimension_and_theme_options(window: &SettingsWindow, state: &AppState) {
    let fallback_fixed_width = u32::try_from(state.shell.last_size.0)
        .ok()
        .filter(|value| *value > 0)
        .map_or(MIN_FIXED_WINDOW_WIDTH, |value| {
            value.max(MIN_FIXED_WINDOW_WIDTH)
        });
    let fallback_fixed_height = u32::try_from(state.shell.last_size.1)
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
    window.set_current_profile_label(SharedString::from(
        current_workspace_label(state.workspace_name.as_deref()),
    ));
    window.set_profile_name(SharedString::from(
        state.workspace_name.clone().unwrap_or_default(),
    ));
    window.set_profile_display_name(SharedString::from(
        state.settings.workspace.display_name.clone(),
    ));
    window.set_profile_description(SharedString::from(
        state.settings.workspace.description.clone(),
    ));
    window.set_known_profiles_label(SharedString::from(super::known_workspaces_label()));
}

/// Populate workspace profile options and related metadata fields.
pub(super) fn populate_preset_options(
    window: &SettingsWindow,
    state: &AppState,
    runtime: &super::RuntimeUiOptions,
) {
    populate_dimension_and_theme_options(window, state);

    let workspace_summaries = super::available_workspace_summaries(state, runtime);
    let workspace_options: Vec<String> = workspace_summaries
        .iter()
        .map(|summary| summary.option_label.clone())
        .collect();

    let requested_workspace =
        super::selected_workspace_from_settings_window(window).or_else(|| state.workspace_name.clone());
    let profile_index = workspace_summaries
        .iter()
        .position(|summary| summary.workspace_name == requested_workspace)
        .or_else(|| {
            workspace_summaries
                .iter()
                .position(|summary| summary.workspace_name.is_none())
        })
        .map_or(0, |index| index as i32);
    window.set_available_profile_options(build_string_model(workspace_options));
    window.set_available_profile_index(profile_index);

    if let Some(selected_summary) = workspace_summaries.get(profile_index as usize) {
        window.set_profile_name(SharedString::from(
            selected_summary.workspace_name.clone().unwrap_or_default(),
        ));
        window.set_profile_display_name(SharedString::from(selected_summary.display_name.clone()));
        window.set_profile_description(SharedString::from(selected_summary.description.clone()));
        window.set_selected_profile_name(SharedString::from(selected_summary.option_label.clone()));
        window.set_selected_profile_is_default(selected_summary.is_default);
        window.set_selected_profile_created_at(SharedString::from(
            selected_summary.created_at_label.clone(),
        ));
        window.set_selected_profile_updated_at(SharedString::from(
            selected_summary.updated_at_label.clone(),
        ));
        window.set_selected_profile_running(selected_summary.is_running);
        window.set_selected_profile_modified(selected_summary.is_modified);
        window.set_selected_profile_missing_apps(selected_summary.missing_apps as i32);
        window.set_selected_profile_status_summary(SharedString::from(
            selected_summary.status_summary.clone(),
        ));
    } else {
        window.set_selected_profile_name(SharedString::from("default"));
        window.set_selected_profile_is_default(true);
        window.set_selected_profile_created_at(SharedString::from(""));
        window.set_selected_profile_updated_at(SharedString::from(""));
        window.set_selected_profile_running(false);
        window.set_selected_profile_modified(false);
        window.set_selected_profile_missing_apps(0);
        window.set_selected_profile_status_summary(SharedString::from(""));
    }

    super::sync_layout_preset_controls(window, &state.settings);
}

/// Populate the hidden-apps list model.
pub(super) fn populate_hidden_apps(window: &SettingsWindow, runtime: &super::RuntimeUiOptions) {
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
