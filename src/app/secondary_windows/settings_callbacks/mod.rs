mod app_rules;
mod editor;
mod profiles;
mod runtime;

use std::cell::RefCell;
use std::rc::Rc;

use crate::{AppState, MainWindow, SettingsWindow};

pub(super) fn register_settings_window_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    profiles::register_profile_callbacks(settings_window, state, main_weak);
    runtime::register_runtime_callbacks(settings_window, state, main_weak);
    app_rules::register_app_rule_callbacks(settings_window, state, main_weak);
    editor::register_editor_callbacks(settings_window, state, main_weak);
}
