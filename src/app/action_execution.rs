//! Shared execution path for settings-backed runtime changes.
//!
//! This module centralizes the choreography for:
//! - mutating persisted settings,
//! - deriving runtime effects,
//! - persisting the updated snapshot, and
//! - applying native/UI side effects in one audited place.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::AppSettings;

use crate::{AppState, MainWindow};

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, reposition_appbar,
    restore_floating_style, unregister_appbar,
};
use super::global_hotkey;
use super::native_runtime::apply_configured_main_window_size;
use super::runtime_support::refresh_ui;
use super::secondary_windows::{
    refresh_open_about_window, refresh_open_settings_window, refresh_open_tag_dialog_window,
    refresh_secondary_window_stacking, refresh_tray_locale,
};
use super::settings::apply_effects::SettingsApplyEffects;
use super::startup;
use super::ui_translations::populate_tr_global;
use super::window_sync::refresh_windows;

#[derive(Debug, Clone)]
pub(crate) struct SettingsRuntimeUpdate {
    pub(crate) effects: SettingsApplyEffects,
    pub(crate) settings: AppSettings,
    pub(crate) workspace_name: Option<String>,
}

fn persist_settings_snapshot(state: &AppState) {
    if let Err(error) = state.settings.save(state.workspace_name.as_deref()) {
        tracing::warn!(
            %error,
            workspace = ?state.workspace_name,
            "failed to persist settings change"
        );
    }
}

#[must_use]
pub(crate) fn finalize_settings_change(
    state: &mut AppState,
    effects: SettingsApplyEffects,
) -> SettingsRuntimeUpdate {
    state.window_collection.current_layout = state.settings.runtime().effective_layout;
    persist_settings_snapshot(state);

    SettingsRuntimeUpdate {
        effects,
        settings: state.settings.snapshot(),
        workspace_name: state.workspace_name.clone(),
    }
}

pub(crate) fn replace_settings_snapshot(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    next: &AppSettings,
) -> bool {
    let runtime_update = {
        let mut state = state.borrow_mut();
        let Some(change) = state.settings.replace_persisted(next) else {
            return false;
        };

        finalize_settings_change(&mut state, change.effects)
    };

    apply_settings_runtime_update(state, weak, &runtime_update);
    true
}

pub(crate) fn execute_settings_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    mutate: impl FnOnce(&mut AppSettings),
) -> bool {
    let runtime_update = {
        let mut state = state.borrow_mut();
        let Some(change) = state.settings.update_persisted(mutate) else {
            return false;
        };

        finalize_settings_change(&mut state, change.effects)
    };

    apply_settings_runtime_update(state, weak, &runtime_update);
    true
}

pub(crate) fn apply_settings_runtime_update(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    runtime_update: &SettingsRuntimeUpdate,
) {
    let hwnd;
    let mut needs_floating_size_restore = false;
    {
        let mut state = state.borrow_mut();
        hwnd = state.shell.hwnd;

        if runtime_update.effects.dock_changed {
            if state.shell.is_appbar {
                unregister_appbar(hwnd);
                state.shell.is_appbar = false;
            }

            if runtime_update.settings.dock_edge.is_some() {
                apply_dock_mode(&mut state);
            } else {
                restore_floating_style(hwnd);
                needs_floating_size_restore = true;
            }
        } else if state.shell.is_appbar {
            reposition_appbar(&mut state);
        }
    }

    if runtime_update.effects.startup_changed {
        startup::sync_run_at_startup(
            runtime_update.settings.run_at_startup,
            runtime_update.workspace_name.as_deref(),
        );
    }

    if runtime_update.effects.hotkey_changed {
        global_hotkey::sync_activate_hotkey(hwnd, &runtime_update.settings);
    }

    if runtime_update.effects.refresh_windows {
        let _ = refresh_windows(state);
    }

    if runtime_update.effects.locale_changed {
        let _ = panopticon::i18n::set_locale(runtime_update.settings.language);
        if let Some(main_window) = weak.upgrade() {
            populate_tr_global(&main_window);
        }
        refresh_open_about_window(state);
        refresh_open_tag_dialog_window(state);
        refresh_tray_locale(state);
    }

    if runtime_update.effects.window_appearance {
        apply_window_appearance(hwnd, &runtime_update.settings);
    }

    apply_topmost_mode(hwnd, runtime_update.settings.always_on_top);

    if needs_floating_size_restore {
        if let Some(main_window) = weak.upgrade() {
            let _ = apply_configured_main_window_size(&main_window, &runtime_update.settings);
        }
    }

    if runtime_update.effects.recompute_ui {
        refresh_ui(state, weak);
    } else {
        refresh_open_settings_window(state);
    }

    refresh_secondary_window_stacking(state);
}
