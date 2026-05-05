//! Runtime effects emitted by app actions.
//!
//! Actions decide what should happen; this module owns the adapter work needed
//! to apply those effects to Slint, DWM/Enumeration, and process lifecycle.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{queue_exit_request, AppState, MainWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeEffect {
    RefreshUi,
    RefreshWindows,
    Exit,
}

pub(crate) fn apply_runtime_effects(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    effects: impl IntoIterator<Item = RuntimeEffect>,
) {
    for effect in effects {
        apply_runtime_effect(state, weak, effect);
    }
}

fn apply_runtime_effect(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    effect: RuntimeEffect,
) {
    match effect {
        RuntimeEffect::RefreshUi => {
            super::runtime_support::refresh_ui(state, weak);
        }
        RuntimeEffect::RefreshWindows => {
            let _ = super::window_sync::refresh_windows(state);
        }
        RuntimeEffect::Exit => {
            queue_exit_request();
        }
    }
}
