use std::cell::{Cell, RefCell};
use std::rc::Rc;

use panopticon::ui_option_ops::current_workspace_label;
use slint::{ComponentHandle, SharedString};
use windows::Win32::Foundation::POINT;

use crate::app::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, keep_dialog_above_owner,
    reposition_appbar, restore_floating_style, unregister_appbar,
};
use crate::app::global_hotkey;
use crate::app::native_runtime::apply_configured_main_window_size;
use crate::app::startup;
use crate::{AppState, MainWindow, SettingsWindow};
use crate::app::model_sync::recompute_and_update_ui;
use crate::app::native_runtime::get_hwnd;
use crate::app::ui_translations::populate_tr_global;
use crate::app::window_sync::refresh_windows;

use super::settings_callbacks;
use super::{
    apply_runtime_settings_window_changes, apply_secondary_window_placement,
    known_workspaces_label, refresh_open_about_window, refresh_open_tag_dialog_window,
    refresh_secondary_window_stacking, refresh_tray_locale, secondary_window_placement,
    sync_settings_window_from_state,
};

thread_local! {
    static SETTINGS_APPLY_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
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

pub(super) fn open_settings_window(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    open_settings_window_with_anchor(state, main_weak, None);
}

pub(super) fn open_settings_window_with_anchor(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
    center_point: Option<POINT>,
) {
    let already_open = crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = get_hwnd(existing.window()) {
                let state = state.borrow();
                let placement = secondary_window_placement(&state, center_point, hwnd);
                crate::app::tray::apply_window_icons(hwnd, &state.shell.icons);
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
    populate_tr_global(&settings_window);

    {
        let state = state.borrow();
        sync_settings_window_from_state(&settings_window, &state);
    }

    settings_callbacks::register_settings_window_callbacks(&settings_window, state, main_weak);

    if let Err(error) = settings_window.show() {
        tracing::error!(%error, "failed to show settings window");
        return;
    }
    if let Some(settings_hwnd) = get_hwnd(settings_window.window()) {
        let state = state.borrow();
        let placement = secondary_window_placement(&state, center_point, settings_hwnd);
        crate::app::tray::apply_window_icons(settings_hwnd, &state.shell.icons);
        apply_window_appearance(settings_hwnd, &state.settings);
        crate::app::theme_ui::apply_settings_window_theme_snapshot(
            &settings_window,
            &state.theme.current_theme,
        );
        apply_secondary_window_placement(settings_hwnd, &state.settings, placement);
    }
    crate::SETTINGS_WIN.with(|handle| *handle.borrow_mut() = Some(settings_window));
}

pub(super) fn apply_settings_window_to_state(
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
        crate::app::settings_ui::apply_settings_window_changes(settings_window, &mut next_settings);
        apply_runtime_settings_window_changes(settings_window, &mut next_settings);
        next_settings = next_settings.normalized();

        if next_settings == previous_settings {
            return;
        }

        state_guard.settings = next_settings;
        state_guard.window_collection.current_layout = state_guard.settings.effective_layout();
        let _ = state_guard
            .settings
            .save(state_guard.workspace_name.as_deref());
        let hwnd = state_guard.shell.hwnd;
        let always_on_top = state_guard.settings.always_on_top;
        let new_dock_edge = state_guard.settings.dock_edge;
        let new_language = state_guard.settings.language;
        let locale_changed = prev_language != new_language;
        let settings_clone = state_guard.settings.clone();
        let workspace_name = state_guard.workspace_name.clone();

        if prev_dock_edge != new_dock_edge {
            if state_guard.shell.is_appbar {
                unregister_appbar(hwnd);
                state_guard.shell.is_appbar = false;
            }
            if new_dock_edge.is_some() {
                apply_dock_mode(&mut state_guard);
            } else {
                restore_floating_style(hwnd);
            }
        } else if state_guard.shell.is_appbar {
            reposition_appbar(&mut state_guard);
        }

        drop(state_guard);
        startup::sync_run_at_startup(settings_clone.run_at_startup, workspace_name.as_deref());
        global_hotkey::sync_activate_hotkey(hwnd, &settings_clone);
        let _ = refresh_windows(state);
        if locale_changed {
            let _ = panopticon::i18n::set_locale(new_language);
            if let Some(main_window) = main_weak.upgrade() {
                populate_tr_global(&main_window);
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
            recompute_and_update_ui(state, &main_window);
        }
        refresh_secondary_window_stacking(state);
    });
}

pub(super) fn refresh_open_settings_window(state: &Rc<RefCell<AppState>>) {
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
        if let Some(dialog_hwnd) = get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.shell.hwnd, &state.settings);
        }
    });
}

pub(super) fn open_settings_window_page(
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
