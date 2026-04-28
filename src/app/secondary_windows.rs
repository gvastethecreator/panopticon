//! Secondary Slint windows: settings, tag dialog, and workspace helpers.

mod dialogs;
mod placement;
mod settings_callbacks;
mod workspace;

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use panopticon::settings::{
    AppSelectionEntry, AppSettings, HiddenAppEntry, ThumbnailRefreshMode,
    MIN_DOCK_COLUMN_THICKNESS, MIN_DOCK_ROW_THICKNESS, MIN_FIXED_WINDOW_HEIGHT,
    MIN_FIXED_WINDOW_WIDTH,
};
use panopticon::theme as theme_catalog;
use panopticon::ui_option_ops::{
    app_option_label, current_workspace_label, hidden_app_option_label, parse_option_value,
    OPTION_SEPARATOR,
};
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use windows::Win32::Foundation::{HWND, POINT};

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, keep_dialog_above_owner,
    reposition_appbar, restore_floating_style, unregister_appbar,
};
use super::global_hotkey;
use super::native_runtime::apply_configured_main_window_size;
use super::settings_ui::{apply_settings_window_changes, populate_settings_window};
use super::startup;
use super::theme_ui::apply_settings_window_theme_snapshot;
use super::tray::apply_window_icons;
use crate::{AppState, MainWindow, SettingsWindow};

use self::placement::SecondaryWindowPlacement;
use self::workspace::WorkspaceUiSummary;

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

fn available_workspace_summaries(
    state: &AppState,
    runtime: &RuntimeUiOptions,
) -> Vec<WorkspaceUiSummary> {
    workspace::available_workspace_summaries(state, runtime)
}

fn parse_workspace_target_input(value: &str) -> Result<Option<String>, String> {
    workspace::parse_workspace_target_input(value)
}

fn set_workspace_feedback(window: &SettingsWindow, message: &str, is_error: bool) {
    workspace::set_workspace_feedback(window, message, is_error);
}

fn clear_workspace_feedback(window: &SettingsWindow) {
    workspace::clear_workspace_feedback(window);
}

pub(crate) fn resolve_secondary_window_owner(state: &AppState, exclude_hwnd: HWND) -> HWND {
    placement::resolve_secondary_window_owner(state, exclude_hwnd)
}

fn secondary_window_placement(
    state: &AppState,
    center_point: Option<POINT>,
    exclude_hwnd: HWND,
) -> SecondaryWindowPlacement {
    placement::secondary_window_placement(state, center_point, exclude_hwnd)
}

pub(crate) fn default_secondary_window_placement(
    state: &AppState,
    exclude_hwnd: HWND,
) -> SecondaryWindowPlacement {
    placement::default_secondary_window_placement(state, exclude_hwnd)
}

pub(crate) fn apply_secondary_window_placement(
    dialog_hwnd: HWND,
    settings: &AppSettings,
    placement: SecondaryWindowPlacement,
) {
    placement::apply_secondary_window_placement(dialog_hwnd, settings, placement);
}

fn set_layout_preset_summary(window: &SettingsWindow, message: &str) {
    window.set_layout_preset_summary(SharedString::from(message));
}

fn confirm_workspace_action(title: &str, description: &str) -> bool {
    matches!(
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(title)
            .set_description(description)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show(),
        rfd::MessageDialogResult::Yes
    )
}

fn selected_layout_preset_name(window: &SettingsWindow) -> Option<String> {
    selected_model_value(
        &window.get_layout_preset_options(),
        window.get_layout_preset_index(),
    )
    .filter(|value| !value.trim().is_empty())
    .or_else(|| {
        let typed = window.get_layout_preset_name().trim().to_owned();
        (!typed.is_empty()).then_some(typed)
    })
}

fn sync_layout_preset_controls(window: &SettingsWindow, settings: &AppSettings) {
    let names = settings.layout_preset_names();
    let selected_before = selected_model_value(
        &window.get_layout_preset_options(),
        window.get_layout_preset_index(),
    );
    let selected_index = selected_before
        .as_deref()
        .and_then(|name| names.iter().position(|candidate| candidate == name))
        .map_or(0, |index| index as i32);

    let has_presets = !names.is_empty();
    window.set_layout_preset_options(build_string_model(names));
    window.set_layout_preset_index(if has_presets { selected_index } else { 0 });

    if window.get_layout_preset_summary().trim().is_empty() {
        let summary = if has_presets {
            "Select a preset to apply or delete, or save current ratios as a new snapshot."
        } else {
            "No layout presets saved yet. Save current layout ratios to create one."
        };
        window.set_layout_preset_summary(SharedString::from(summary));
    }
}

fn stop_shortcut_recording(window: &SettingsWindow, hint: &str) {
    window.set_shortcut_recording_mode(false);
    window.set_shortcut_recording_target(SharedString::from(""));
    window.set_shortcut_recording_hint(SharedString::from(hint));
}

fn shortcut_recording_label(target: &str) -> &str {
    match target {
        "layout_column" => "Layout column",
        "reset_layout" => "Reset layout",
        "cycle_layout" => "Cycle layout",
        "toggle_toolbar" => "Toggle toolbar",
        "toggle_animations" => "Toggle animations",
        "toggle_window_info" => "Toggle window info",
        "open_settings" => "Open settings",
        "open_menu" => "Open menu",
        "open_command_palette" => "Open command palette",
        "refresh_now" => "Refresh now",
        "exit_app" => "Exit app",
        "toggle_always_on_top" => "Always on top",
        "global_activate" => "Global activate",
        _ => "Shortcut",
    }
}

fn normalize_recorded_shortcut(key_text: &str) -> Option<String> {
    match key_text {
        "\t" => Some("Tab".to_owned()),
        "\n" | "\r" => Some("Enter".to_owned()),
        " " => Some("Space".to_owned()),
        _ => {
            let mut chars = key_text.chars();
            let first = chars.next()?;
            if chars.next().is_some() || first.is_control() {
                return None;
            }

            if first.is_ascii_alphanumeric() {
                Some(first.to_ascii_uppercase().to_string())
            } else {
                Some(first.to_string())
            }
        }
    }
}

fn apply_recorded_shortcut_binding(window: &SettingsWindow, target: &str, binding: &str) -> bool {
    let binding = SharedString::from(binding);
    match target {
        "layout_column" => window.set_shortcut_layout_column(binding),
        "reset_layout" => window.set_shortcut_reset_layout(binding),
        "cycle_layout" => window.set_shortcut_cycle_layout(binding),
        "toggle_toolbar" => window.set_shortcut_toggle_toolbar(binding),
        "toggle_animations" => window.set_shortcut_toggle_animations(binding),
        "toggle_window_info" => window.set_shortcut_toggle_window_info(binding),
        "open_settings" => window.set_shortcut_open_settings(binding),
        "open_menu" => window.set_shortcut_open_menu(binding),
        "open_command_palette" => window.set_shortcut_open_command_palette(binding),
        "refresh_now" => window.set_shortcut_refresh_now(binding),
        "exit_app" => window.set_shortcut_exit_app(binding),
        "toggle_always_on_top" => window.set_shortcut_toggle_always_on_top(binding),
        "global_activate" => window.set_shortcut_global_activate(binding),
        _ => return false,
    }

    true
}

fn selected_workspace_from_settings_window(window: &SettingsWindow) -> Option<String> {
    workspace::selected_workspace_from_settings_window(window)
}

fn select_workspace_in_settings_window(window: &SettingsWindow, workspace: Option<&str>) {
    workspace::select_workspace_in_settings_window(window, workspace);
}

struct RuntimeUiOptions {
    monitors: Vec<String>,
    tags: Vec<String>,
    apps: Vec<AppSelectionEntry>,
    hidden_apps: Vec<HiddenAppEntry>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "UI filters need explicit boolean flags to drive quick predicates without extra allocations"
)]
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
    workspace::ensure_default_workspaces_exist(settings);
}

pub(crate) fn open_settings_window(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    open_settings_window_with_anchor(state, main_weak, None);
}

pub(crate) fn open_settings_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    center_point: Option<POINT>,
) {
    let already_open = crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                let placement = secondary_window_placement(&state, center_point, hwnd);
                apply_window_icons(hwnd, &state.icons);
                apply_secondary_window_placement(hwnd, &state.settings, placement);
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

    settings_callbacks::register_settings_window_callbacks(&settings_window, state, main_weak);

    if let Err(error) = settings_window.show() {
        tracing::error!(%error, "failed to show settings window");
        return;
    }
    if let Some(settings_hwnd) = crate::get_hwnd(settings_window.window()) {
        let state = state.borrow();
        let placement = secondary_window_placement(&state, center_point, settings_hwnd);
        apply_window_icons(settings_hwnd, &state.icons);
        apply_window_appearance(settings_hwnd, &state.settings);
        apply_settings_window_theme_snapshot(&settings_window, &state.current_theme);
        apply_secondary_window_placement(settings_hwnd, &state.settings, placement);
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
        refresh_secondary_window_stacking(state);
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

pub(crate) fn open_settings_window_page(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    page_index: i32,
) {
    open_settings_window(state, main_weak);

    crate::SETTINGS_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            window.set_current_page(page_index.clamp(0, 5));
        }
    });
}

pub(crate) fn open_about_window(state: &Rc<RefCell<AppState>>) {
    dialogs::open_about_window(state);
}

pub(crate) fn open_about_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    center_point: Option<POINT>,
) {
    dialogs::open_about_window_with_anchor(state, center_point);
}

pub(crate) fn refresh_open_about_window(state: &Rc<RefCell<AppState>>) {
    dialogs::refresh_open_about_window(state);
}

pub(crate) fn open_create_tag_dialog(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
) {
    dialogs::open_create_tag_dialog(state, weak, info);
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
    let workspace_summaries = available_workspace_summaries(state, &runtime);
    let workspace_options: Vec<String> = workspace_summaries
        .iter()
        .map(|summary| summary.option_label.clone())
        .collect();
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
    window.set_profile_display_name(SharedString::from(
        state.settings.workspace.display_name.clone(),
    ));
    window.set_profile_description(SharedString::from(
        state.settings.workspace.description.clone(),
    ));
    window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
    let requested_workspace =
        selected_workspace_from_settings_window(window).or_else(|| state.workspace_name.clone());
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

    sync_layout_preset_controls(window, &state.settings);

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
    let selected_slot = settings.pinned_position_for(&app_id);
    let pinned_slot = selected_slot.map_or(0, |slot| {
        i32::try_from(slot.saturating_add(1)).unwrap_or(i32::MAX)
    });
    let conflict_summary = settings
        .pinned_slot_conflicts()
        .into_iter()
        .find(|(slot, labels)| {
            selected_slot.is_some_and(|current| current == *slot) && labels.len() > 1
        })
        .map_or_else(String::new, |(slot, labels)| {
            format!("Slot #{} is shared by {}", slot + 1, labels.join(", "))
        });

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
    window.set_app_rules_pin_slot(pinned_slot);
    window.set_app_rules_pin_conflict_summary(SharedString::from(conflict_summary));
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
    window.set_app_rules_pin_slot(0);
    window.set_app_rules_pin_conflict_summary(SharedString::from(""));
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
        .map(str::to_ascii_lowercase)
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

fn sync_workspace_editor_from_selection(
    window: &SettingsWindow,
    fallback_workspace: Option<String>,
    state: &AppState,
) {
    workspace::sync_workspace_editor_from_selection(window, fallback_workspace, state);
}

fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    let draft_profile_name = window.get_profile_name();
    let draft_profile_display_name = window.get_profile_display_name();
    let draft_profile_description = window.get_profile_description();
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
    if !draft_profile_display_name.is_empty() {
        window.set_profile_display_name(draft_profile_display_name);
    }
    if !draft_profile_description.is_empty() {
        window.set_profile_description(draft_profile_description);
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

fn known_workspaces_label() -> String {
    workspace::known_workspaces_label()
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
    workspace::load_workspace_into_current_instance(state, main_weak, requested_workspace)
}

pub(crate) fn open_workspace_in_new_instance(
    state: &Rc<RefCell<AppState>>,
    requested_workspace: Option<String>,
) -> bool {
    workspace::open_workspace_in_new_instance(state, requested_workspace)
}

fn refresh_open_tag_dialog_window(state: &Rc<RefCell<AppState>>) {
    dialogs::refresh_open_tag_dialog_window(state);
}

pub(crate) fn refresh_secondary_window_stacking(state: &Rc<RefCell<AppState>>) {
    placement::refresh_secondary_window_stacking(state);
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

#[cfg(test)]
mod tests {
    use super::{normalize_recorded_shortcut, parse_rgb_hex, parse_tags_csv};

    #[test]
    fn recorded_shortcut_normalizes_special_keys() {
        assert_eq!(normalize_recorded_shortcut("\t"), Some("Tab".to_owned()));
        assert_eq!(normalize_recorded_shortcut("\n"), Some("Enter".to_owned()));
        assert_eq!(normalize_recorded_shortcut(" "), Some("Space".to_owned()));
    }

    #[test]
    fn recorded_shortcut_uppercases_single_alphanumeric_keys() {
        assert_eq!(normalize_recorded_shortcut("a"), Some("A".to_owned()));
        assert_eq!(normalize_recorded_shortcut("7"), Some("7".to_owned()));
    }

    #[test]
    fn recorded_shortcut_rejects_control_and_multi_character_inputs() {
        assert_eq!(normalize_recorded_shortcut(""), None);
        assert_eq!(normalize_recorded_shortcut("ab"), None);
        assert_eq!(normalize_recorded_shortcut("\u{0001}"), None);
    }

    #[test]
    fn parse_tags_csv_normalizes_sorts_and_deduplicates() {
        assert_eq!(
            parse_tags_csv(" Work ,alpha,work, Beta ,,ALPHA "),
            vec!["alpha".to_owned(), "beta".to_owned(), "work".to_owned()]
        );
    }

    #[test]
    fn parse_rgb_hex_accepts_plain_and_prefixed_values() {
        assert_eq!(parse_rgb_hex("#112233"), Some((17, 34, 51)));
        assert_eq!(parse_rgb_hex("A0B1C2"), Some((160, 177, 194)));
    }

    #[test]
    fn parse_rgb_hex_rejects_invalid_shapes() {
        assert_eq!(parse_rgb_hex("#123"), None);
        assert_eq!(parse_rgb_hex("#GG1122"), None);
    }
}
