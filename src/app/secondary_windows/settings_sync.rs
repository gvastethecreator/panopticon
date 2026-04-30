//! Settings-window sync orchestrator.
//!
//! Coordinates the focused builders (filter, app-rules, preset) to
//! populate and synchronise the settings window without owning any
//! domain logic itself.

use panopticon::window_enum::{enumerate_windows, WindowInfo};

use crate::{AppState, SettingsWindow};
use crate::app::settings_ui::populate_settings_window;
use crate::app::theme_ui::apply_settings_window_theme_snapshot;
use crate::app::ui_translations::populate_tr_global;

use super::{
    settings_preset_sync, settings_filter_sync, settings_app_rules_sync,
};

/// Collect the runtime-derived options (monitors, tags, apps, hidden apps)
/// that feed the settings-window dropdowns and lists.
pub(super) fn collect_runtime_ui_options(state: &AppState) -> super::RuntimeUiOptions {
    let windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.shell.hwnd)
        .collect();

    super::RuntimeUiOptions {
        monitors: panopticon::window_ops::collect_available_monitors(&windows),
        tags: state.settings.known_tags(),
        apps: panopticon::window_ops::collect_available_apps(&windows),
        hidden_apps: state.settings.hidden_app_entries(),
    }
}

/// Top-level synchronisation: rebuild the entire settings window from `AppState`.
pub(super) fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    let draft_profile_name = window.get_profile_name();
    let draft_profile_display_name = window.get_profile_display_name();
    let draft_profile_description = window.get_profile_description();
    populate_tr_global(window);
    window.set_suspend_live_apply(true);
    populate_settings_window(window, &state.settings);
    populate_settings_window_runtime_fields(window, state);
    let resolved_theme = panopticon::theme::resolve_ui_theme(
        state.settings.theme_id.as_deref(),
        &state.settings.background_color_hex,
        &state.settings.theme_color_overrides,
    );
    apply_settings_window_theme_snapshot(window, &resolved_theme);
    if !draft_profile_name.is_empty() {
        window.set_profile_name(draft_profile_name);
    }
    if !draft_profile_display_name.is_empty() {
        window.set_profile_display_name(draft_profile_display_name);
    }
    if !draft_profile_description.is_empty() {
        window.set_profile_description(draft_profile_description);
    }
    window.set_suspend_live_apply(false);
}

/// Populate all runtime-derived fields in the settings window.
pub(super) fn populate_settings_window_runtime_fields(
    window: &SettingsWindow,
    state: &AppState,
) {
    let runtime = collect_runtime_ui_options(state);

    settings_preset_sync::populate_preset_options(window, state, &runtime);
    settings_filter_sync::populate_filter_options(window, state, &runtime);
    settings_app_rules_sync::populate_app_rules_list(window, state, &runtime);
    settings_app_rules_sync::sync_selected_app_rule_editor(window, &state.settings);
    settings_preset_sync::populate_hidden_apps(window, &runtime);
}

/// Apply the current filter selections from the settings window back to `AppSettings`.
pub(super) fn apply_runtime_settings_window_changes(
    window: &SettingsWindow,
    settings: &mut panopticon::settings::AppSettings,
) {
    settings_filter_sync::apply_runtime_settings_window_changes(window, settings);
}
