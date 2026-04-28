//! Secondary Slint windows: settings, tag dialog, and workspace helpers.

mod dialogs;
mod placement;
#[path = "secondary_windows/settings_callbacks/mod.rs"]
mod settings_callbacks;
mod settings_sync;
mod workspace;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use panopticon::settings::{AppSelectionEntry, AppSettings, HiddenAppEntry, ThumbnailRefreshMode};
use panopticon::ui_option_ops::current_workspace_label;
use panopticon::window_enum::WindowInfo;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use windows::Win32::Foundation::{HWND, POINT};

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, keep_dialog_above_owner,
    reposition_appbar, restore_floating_style, unregister_appbar,
};
use super::global_hotkey;
use super::native_runtime::apply_configured_main_window_size;
use super::settings_ui::apply_settings_window_changes;
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
    settings_sync::apply_runtime_settings_window_changes(window, settings);
}

fn populate_settings_window_runtime_fields(window: &SettingsWindow, state: &AppState) {
    settings_sync::populate_settings_window_runtime_fields(window, state);
}

fn sync_selected_app_rule_editor(window: &SettingsWindow, settings: &AppSettings) {
    settings_sync::sync_selected_app_rule_editor(window, settings);
}

fn sync_app_rule_tags_editor(window: &SettingsWindow, tags: &[String], clear_input: bool) {
    settings_sync::sync_app_rule_tags_editor(window, tags, clear_input);
}

fn refresh_mode_from_index(index: i32) -> ThumbnailRefreshMode {
    settings_sync::refresh_mode_from_index(index)
}

fn parse_tags_csv(raw: &str) -> Vec<String> {
    settings_sync::parse_tags_csv(raw)
}

fn sync_workspace_editor_from_selection(
    window: &SettingsWindow,
    fallback_workspace: Option<String>,
    state: &AppState,
) {
    workspace::sync_workspace_editor_from_selection(window, fallback_workspace, state);
}

fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    settings_sync::sync_settings_window_from_state(window, state);
}

fn collect_runtime_ui_options(state: &AppState) -> RuntimeUiOptions {
    settings_sync::collect_runtime_ui_options(state)
}

fn known_workspaces_label() -> String {
    workspace::known_workspaces_label()
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
