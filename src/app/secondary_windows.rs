//! Secondary Slint windows: settings, tag dialog, and placement helpers.

mod dialogs;
mod placement;
mod settings_window;

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::{AppSelectionEntry, AppSettings, HiddenAppEntry, ThumbnailRefreshMode};
use panopticon::window_enum::WindowInfo;

use windows::Win32::Foundation::{HWND, POINT};

use crate::{AppState, MainWindow, SettingsWindow};

use self::placement::SecondaryWindowPlacement;

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

pub(crate) fn confirm_workspace_action(title: &str, description: &str) -> bool {
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

pub(crate) struct RuntimeUiOptions {
    pub(crate) monitors: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) apps: Vec<AppSelectionEntry>,
    pub(crate) hidden_apps: Vec<HiddenAppEntry>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "UI filters need explicit boolean flags to drive quick predicates without extra allocations"
)]
pub(crate) struct AppRuleListEntry {
    pub(crate) option: AppSelectionEntry,
    pub(crate) is_running: bool,
    pub(crate) has_saved_rule: bool,
    pub(crate) is_hidden: bool,
    pub(crate) has_tags: bool,
    pub(crate) has_custom_refresh: bool,
    pub(crate) is_pinned: bool,
    pub(crate) searchable_blob: String,
}

pub(crate) use self::settings_window::{
    apply_settings_window_to_state, open_settings_window_page, open_settings_window_with_anchor,
    refresh_open_settings_window,
};

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

pub(crate) fn apply_runtime_settings_window_changes(
    window: &SettingsWindow,
    settings: &mut AppSettings,
) {
    crate::app::settings::filter_sync::apply_runtime_settings_window_changes(window, settings);
}

pub(crate) fn populate_settings_window_runtime_fields(window: &SettingsWindow, state: &AppState) {
    crate::app::settings::sync::populate_settings_window_runtime_fields(window, state);
}

pub(crate) fn sync_selected_app_rule_editor(window: &SettingsWindow, settings: &AppSettings) {
    crate::app::settings::app_rules_sync::sync_selected_app_rule_editor(window, settings);
}

pub(crate) fn sync_app_rule_tags_editor(
    window: &SettingsWindow,
    tags: &[String],
    clear_input: bool,
) {
    crate::app::settings::app_rules_sync::sync_app_rule_tags_editor(window, tags, clear_input);
}

pub(crate) fn refresh_mode_from_index(index: i32) -> ThumbnailRefreshMode {
    crate::app::settings::app_rules_sync::refresh_mode_from_index(index)
}

pub(crate) fn parse_tags_csv(raw: &str) -> Vec<String> {
    crate::app::settings::app_rules_sync::parse_tags_csv(raw)
}

pub(crate) fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    crate::app::settings::sync::sync_settings_window_from_state(window, state);
}

pub(crate) fn collect_runtime_ui_options(state: &AppState) -> RuntimeUiOptions {
    crate::app::settings::sync::collect_runtime_ui_options(state)
}

pub(crate) fn refresh_open_tag_dialog_window(state: &Rc<RefCell<AppState>>) {
    dialogs::refresh_open_tag_dialog_window(state);
}

pub(crate) fn refresh_secondary_window_stacking(state: &Rc<RefCell<AppState>>) {
    placement::refresh_secondary_window_stacking(state);
}

pub(crate) fn refresh_tray_locale(state: &Rc<RefCell<AppState>>) {
    let mut state = state.borrow_mut();
    let icon = state.shell.icons.small;
    if let Some(tray) = state.shell.tray_icon.as_mut() {
        tray.refresh(icon);
    }
}
