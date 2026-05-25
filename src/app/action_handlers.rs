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
        let edge = self.0;
        let _ = super::action_execution::execute_settings_action(ctx.state, ctx.weak, |settings| {
            settings.dock_edge = edge;
        });
    }
}

// ───────────────────────── CycleTheme handler ─────────────────────────

pub(crate) struct CycleThemeHandler {
    pub direction: i32,
}

impl ActionHandler for CycleThemeHandler {
    fn handle(&self, ctx: &mut ActionContext) {
        let current_idx = {
            let state = ctx.state.borrow();
            panopticon::theme::theme_index(state.settings.theme_id.as_deref())
        };
        let total = panopticon::theme::theme_labels().len() as i32;
        let next_idx = (current_idx + self.direction).rem_euclid(total);
        let new_id = panopticon::theme::theme_id_by_index(next_idx);
        let next_background_hex =
            panopticon::theme::theme_base_background_hex(new_id.as_deref(), "181513");

        let _ = super::action_execution::execute_settings_action(ctx.state, ctx.weak, |settings| {
            settings.theme_id = new_id;
            if settings.theme_id.is_some() {
                settings
                    .background_color_hex
                    .clone_from(&next_background_hex);
            }
        });
    }
}

// ───────────────────────── ToggleAlwaysOnTop handler ─────────────────────────

pub(crate) struct ToggleAlwaysOnTopHandler;

impl ActionHandler for ToggleAlwaysOnTopHandler {
    fn handle(&self, ctx: &mut ActionContext) {
        let _ = super::action_execution::execute_settings_action(ctx.state, ctx.weak, |settings| {
            settings.always_on_top = !settings.always_on_top;
        });
    }
}
