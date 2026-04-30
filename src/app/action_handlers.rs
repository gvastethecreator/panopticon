//! Action handler trait and implementations for complex cross-domain actions.
//!
//! Simple actions (one-liner toggles) remain inline in [`dispatch_action`].
//! Complex actions that touch multiple subsystems are extracted here so
//! each handler lives near the domain it orchestrates.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::settings::DockEdge;

use crate::{AppState, MainWindow};

/// Context passed to every action handler.
pub(crate) struct ActionContext<'a> {
    pub state: &'a Rc<RefCell<AppState>>,
    pub weak: &'a slint::Weak<MainWindow>,
}

/// Trait for actions complex enough to warrant their own module.
pub(crate) trait ActionHandler {
    fn handle(&self, ctx: &mut ActionContext);
}

// ───────────────────────── SetDockEdge handler ─────────────────────────

pub(crate) struct SetDockEdgeHandler(pub Option<DockEdge>);

impl ActionHandler for SetDockEdgeHandler {
    fn handle(&self, ctx: &mut ActionContext) {
        use super::dock::{
            apply_dock_mode, apply_topmost_mode, restore_floating_style, unregister_appbar,
        };
        use super::native_runtime::apply_configured_main_window_size;
        use super::runtime_support::refresh_ui;
        use super::window_sync::refresh_windows;

        let edge = self.0;
        let mut floating_settings = None;
        {
            let mut state = ctx.state.borrow_mut();
            if state.shell.is_appbar {
                unregister_appbar(state.shell.hwnd);
                state.shell.is_appbar = false;
            }
            state.settings.dock_edge = edge;
            state.settings = state.settings.normalized();
            state.window_collection.current_layout = state.settings.effective_layout();
            let _ = state.settings.save(state.workspace_name.as_deref());
            if edge.is_some() {
                apply_dock_mode(&mut state);
            } else {
                restore_floating_style(state.shell.hwnd);
                apply_topmost_mode(state.shell.hwnd, state.settings.always_on_top);
                floating_settings = Some(state.settings.clone());
            }
        }
        if let Some(settings) = floating_settings {
            if let Some(main_window) = ctx.weak.upgrade() {
                let _ = apply_configured_main_window_size(&main_window, &settings);
            }
        }
        let _ = refresh_windows(ctx.state);
        refresh_ui(ctx.state, ctx.weak);
    }
}

// ───────────────────────── CycleTheme handler ─────────────────────────

pub(crate) struct CycleThemeHandler {
    pub direction: i32,
}

impl ActionHandler for CycleThemeHandler {
    fn handle(&self, ctx: &mut ActionContext) {
        use super::dock::apply_window_appearance;
        use super::runtime_support::{refresh_ui, update_settings};
        use super::secondary_windows;

        let current_idx = {
            let state = ctx.state.borrow();
            panopticon::theme::theme_index(state.settings.theme_id.as_deref())
        };
        let total = panopticon::theme::theme_labels().len() as i32;
        let next_idx = (current_idx + self.direction).rem_euclid(total);
        let new_id = panopticon::theme::theme_id_by_index(next_idx);
        let next_background_hex =
            panopticon::theme::theme_base_background_hex(new_id.as_deref(), "181513");

        update_settings(ctx.state, |settings| {
            settings.theme_id = new_id;
            if settings.theme_id.is_some() {
                settings
                    .background_color_hex
                    .clone_from(&next_background_hex);
            }
        });

        let state_ref = ctx.state.borrow();
        apply_window_appearance(state_ref.shell.hwnd, &state_ref.settings);
        drop(state_ref);

        secondary_windows::refresh_secondary_window_stacking(ctx.state);
        refresh_ui(ctx.state, ctx.weak);
    }
}

// ───────────────────────── ToggleAlwaysOnTop handler ─────────────────────────

pub(crate) struct ToggleAlwaysOnTopHandler;

impl ActionHandler for ToggleAlwaysOnTopHandler {
    fn handle(&self, ctx: &mut ActionContext) {
        use super::dock::apply_topmost_mode;
        use super::runtime_support::{refresh_ui, update_settings};
        use super::secondary_windows;

        update_settings(ctx.state, |settings| {
            settings.always_on_top = !settings.always_on_top;
        });
        let state_ref = ctx.state.borrow();
        apply_topmost_mode(state_ref.shell.hwnd, state_ref.settings.always_on_top);
        drop(state_ref);
        secondary_windows::refresh_secondary_window_stacking(ctx.state);
        refresh_ui(ctx.state, ctx.weak);
    }
}
