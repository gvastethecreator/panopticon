//! Secondary Slint windows: settings, tag dialog, and workspace helpers.

mod dialogs;
mod placement;
#[path = "secondary_windows/settings_callbacks/mod.rs"]
mod settings_callbacks;
mod settings_helpers;
mod settings_sync;
mod settings_window;
mod workspace;

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::{AppSelectionEntry, AppSettings, HiddenAppEntry, ThumbnailRefreshMode};
use panopticon::window_enum::WindowInfo;
use slint::{ModelRc, SharedString};
use windows::Win32::Foundation::{HWND, POINT};

use crate::{AppState, MainWindow, SettingsWindow};

use self::placement::SecondaryWindowPlacement;
use self::workspace::WorkspaceUiSummary;

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
    settings_helpers::set_layout_preset_summary(window, message);
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
    settings_helpers::selected_layout_preset_name(window)
}

fn sync_layout_preset_controls(window: &SettingsWindow, settings: &AppSettings) {
    settings_helpers::sync_layout_preset_controls(window, settings);
}

fn stop_shortcut_recording(window: &SettingsWindow, hint: &str) {
    settings_helpers::stop_shortcut_recording(window, hint);
}

fn shortcut_recording_label(target: &str) -> &str {
    settings_helpers::shortcut_recording_label(target)
}

fn normalize_recorded_shortcut(key_text: &str) -> Option<String> {
    settings_helpers::normalize_recorded_shortcut(key_text)
}

fn apply_recorded_shortcut_binding(window: &SettingsWindow, target: &str, binding: &str) -> bool {
    settings_helpers::apply_recorded_shortcut_binding(window, target, binding)
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
    settings_window::open_settings_window(state, main_weak);
}

pub(crate) fn open_settings_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    center_point: Option<POINT>,
) {
    settings_window::open_settings_window_with_anchor(state, main_weak, center_point);
}

fn apply_settings_window_to_state(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window::apply_settings_window_to_state(state, main_weak);
}

pub(crate) fn refresh_open_settings_window(state: &Rc<RefCell<AppState>>) {
    settings_window::refresh_open_settings_window(state);
}

pub(crate) fn open_settings_window_page(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    page_index: i32,
) {
    settings_window::open_settings_window_page(state, main_weak, page_index);
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
    settings_helpers::build_string_model(values)
}

fn selected_model_value(model: &ModelRc<SharedString>, index: i32) -> Option<String> {
    settings_helpers::selected_model_value(model, index)
}

fn apply_background_color(window: &SettingsWindow, red: i32, green: i32, blue: i32) {
    settings_helpers::apply_background_color(window, red, green, blue);
}

fn parse_rgb_hex(input: &str) -> Option<(i32, i32, i32)> {
    settings_helpers::parse_rgb_hex(input)
}
