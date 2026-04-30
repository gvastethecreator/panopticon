//! Discrete phases of the UI tick loop.
//!
//! Each phase is a standalone function with a small, explicit interface.
//! [`run_ui_tick`] orchestrates them in order and accumulates their effects
//! into a [`TickEffects`] struct.  This makes dependencies between phases
//! explicit and allows individual phases to be tested in isolation.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, Timer, TimerMode};
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use crate::{AppState, MainWindow, PendingAction, UpdateStatus, PENDING_ACTIONS};
use panopticon::settings::RefreshPerformanceMode;
use super::model_sync::{advance_animation, recompute_and_update_ui};
use super::window_sync::refresh_windows;

// ───────────────────────── Constants ─────────────────────────

const DEFAULT_REFRESH_TIMER_INTERVAL_MS: u32 = 2_000;
const MIN_REFRESH_TIMER_INTERVAL_MS: u32 = 50;
const DWM_IDLE_SYNC_EVERY_TICKS: u8 = 4;
const DWM_IDLE_SYNC_EVERY_TICKS_REALTIME: u8 = 1;
const DWM_IDLE_SYNC_EVERY_TICKS_BATTERY_SAVER: u8 = 8;
const DWM_IDLE_SYNC_EVERY_TICKS_MANUAL_SLOW: u8 = 12;

// ───────────────────────── TickEffects ─────────────────────────

/// Accumulates what happened during a single UI tick so downstream
/// phases can make conditional decisions without re-reading state.
#[derive(Debug, Default, Clone, Copy)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "effect accumulator — each bool represents an independent cross-cutting concern"
)]
pub(crate) struct TickEffects {
    pub had_actions: bool,
    pub recomputed_from_resize: bool,
    pub recomputed_from_refresh: bool,
    pub viewport_changed: bool,
    pub window_animation_active: bool,
    pub theme_animation_active: bool,
    pub is_animating_or_dirty: bool,
    pub should_sync_dwm: bool,
}

impl TickEffects {
    /// True when any work was done that makes a DWM sync worthwhile.
    pub fn needs_immediate_dwm_sync(self) -> bool {
        self.had_actions
            || self.recomputed_from_resize
            || self.recomputed_from_refresh
            || self.viewport_changed
            || self.is_animating_or_dirty
    }
}

// ───────────────────────── Phases ─────────────────────────

/// Phase 1 — retry native runtime init if HWND is not yet available.
///
/// Returns `true` when a retry was scheduled (caller should skip the rest of
/// the tick).
pub(crate) fn try_native_init(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    win: &MainWindow,
    native_init_retry_timer: &Rc<Timer>,
) -> bool {
    if !state.borrow().shell.hwnd.0.is_null() {
        return false;
    }
    schedule_native_runtime_retry(state, weak, native_init_retry_timer, win);
    true
}

/// Phase 2 — poll update check if one is in flight.
pub(crate) fn poll_update_check(state: &Rc<RefCell<AppState>>) {
    let should_poll = state
        .try_borrow()
        .is_ok_and(|state_ref| matches!(state_ref.update_status, UpdateStatus::Checking));
    if should_poll {
        if let Some(outcome) = super::updates::poll_latest_release_check() {
            super::runtime_support::apply_update_check_outcome(state, outcome);
        }
    }
}

/// Phase 3 — drain the pending action queue.
pub(crate) fn drain_actions(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
) -> bool {
    PENDING_ACTIONS.with(|queue_cell| {
        let mut queue = queue_cell.borrow_mut();
        if queue.is_empty() {
            return false;
        }

        let mut batch = std::mem::take(&mut *queue);
        drop(queue);

        for action in batch.drain(..) {
            handle_pending_action(state, win, action);
        }

        let mut queue = queue_cell.borrow_mut();
        if queue.is_empty() {
            *queue = batch;
        }
        true
    })
}

/// Phase 4 — detect window resize and sync floating size to settings.
pub(crate) fn detect_resize(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
    floating_size_sync_timer: &Rc<Timer>,
) -> bool {
    let phys_size = win.window().size();
    let scale = win.window().scale_factor();
    let logical_w = (phys_size.width as f32 / scale).round() as i32;
    let logical_h = (phys_size.height as f32 / scale).round() as i32;
    let needs_relayout = {
        let state_ref = state.borrow();
        logical_w != state_ref.shell.last_size.0 || logical_h != state_ref.shell.last_size.1
    };

    if !needs_relayout {
        return false;
    }

    {
        let mut state_ref = state.borrow_mut();
        state_ref.shell.last_size = (logical_w, logical_h);
    }

    super::runtime_support::sync_floating_window_size_with_resize(
        state,
        logical_w,
        logical_h,
        floating_size_sync_timer,
    );
    true
}

/// Phase 5 — reconcile refresh-triggered recompute.
pub(crate) fn reconcile_refresh(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
    refresh_recompute_pending: &Cell<bool>,
) -> bool {
    let did_refresh = refresh_recompute_pending.replace(false);
    if did_refresh {
        recompute_and_update_ui(state, win);
    }
    did_refresh
}

/// Phase 6 — detect viewport scroll change.
pub(crate) fn detect_viewport_change(
    win: &MainWindow,
    last_viewport: &Cell<Option<(f32, f32)>>,
) -> bool {
    let current = (win.get_viewport_x(), win.get_viewport_y());
    let previous = last_viewport.get();
    last_viewport.set(Some(current));
    previous.is_none_or(|(x, y)| {
        (current.0 - x).abs() > f32::EPSILON || (current.1 - y).abs() > f32::EPSILON
    })
}

/// Phase 7 — compute runtime activity flags (animations, drag).
pub(crate) fn compute_activity_flags(
    state: &Rc<RefCell<AppState>>,
) -> (bool, bool, bool) {
    state
        .try_borrow()
        .map_or((false, false, false), |state_ref| {
            let window_animation_active = state_ref.theme.animation_started_at.is_some();
            let theme_animation_active = state_ref.theme.theme_animation.is_some();
            let is_animating_or_dirty = window_animation_active
                || theme_animation_active
                || state_ref.window_collection.drag_separator.is_some();
            (
                window_animation_active,
                theme_animation_active,
                is_animating_or_dirty,
            )
        })
}

/// Phase 8 — decide whether to sync DWM this tick.
pub(crate) fn decide_dwm_sync(
    effects: TickEffects,
    dwm_idle_ticks: &Cell<u8>,
    state: &Rc<RefCell<AppState>>,
) -> bool {
    if effects.needs_immediate_dwm_sync() {
        dwm_idle_ticks.set(0);
        true
    } else {
        let cadence = current_dwm_idle_sync_every_ticks(state);
        schedule_idle_dwm_sync(dwm_idle_ticks, cadence)
    }
}

/// Phase 9 — advance window layout animation.
pub(crate) fn advance_window_animation(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
    active: bool,
) {
    if active {
        advance_animation(state, win);
    }
}

/// Phase 10 — advance theme animation.
pub(crate) fn advance_theme_animation(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
    active: bool,
) {
    if active {
        super::theme_ui::advance_theme_animation(state, win);
    }
}

/// Phase 11 — synchronise DWM thumbnail positions.
pub(crate) fn sync_dwm(state: &Rc<RefCell<AppState>>, win: &MainWindow, should_sync: bool) {
    if should_sync {
        super::dwm::update_dwm_thumbnails(state, win);
    }
}

// ───────────────────────── Helpers ─────────────────────────

fn schedule_native_runtime_retry(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    native_init_retry_timer: &Rc<Timer>,
    win: &MainWindow,
) {
    if native_init_retry_timer.running()
        || super::native_runtime::try_initialize_native_runtime(state, win)
    {
        return;
    }

    let state_retry = state.clone();
    let weak_retry = weak.clone();
    native_init_retry_timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(350),
        move || {
            if let Some(win_retry) = weak_retry.upgrade() {
                let _ = super::native_runtime::try_initialize_native_runtime(
                    &state_retry,
                    &win_retry,
                );
            }
        },
    );
}

fn handle_pending_action(state: &Rc<RefCell<AppState>>, win: &MainWindow, action: PendingAction) {
    let weak = win.as_weak();
    match action {
        PendingAction::Tray(action, anchor) => {
            super::tray_actions::handle_tray_action(state, &weak, action, anchor);
        }
        PendingAction::ActivateMainWindow => {
            super::tray_actions::activate_main_window(state, &weak);
        }
        PendingAction::Reposition => {
            if let Ok(mut state_ref) = state.try_borrow_mut() {
                if state_ref.shell.is_appbar {
                    super::dock::reposition_appbar(&mut state_ref);
                }
            }
        }
        PendingAction::HideToTray => {
            super::dwm::release_all_thumbnails(state);
            win.hide().ok();
        }
        PendingAction::Refresh => {
            if refresh_windows(state) {
                recompute_and_update_ui(state, win);
            }
        }
        PendingAction::Exit => {
            super::native_runtime::request_exit(state);
        }
    }
}

pub(crate) fn schedule_idle_dwm_sync(dwm_idle_ticks: &Cell<u8>, sync_every_ticks: u8) -> bool {
    let sync_every_ticks = sync_every_ticks.max(1);
    let next = dwm_idle_ticks.get().saturating_add(1);
    if next >= sync_every_ticks {
        dwm_idle_ticks.set(0);
        true
    } else {
        dwm_idle_ticks.set(next);
        false
    }
}

fn current_dwm_idle_sync_every_ticks(state: &Rc<RefCell<AppState>>) -> u8 {
    state
        .try_borrow()
        .map_or(DWM_IDLE_SYNC_EVERY_TICKS, |state_ref| {
            match state_ref.settings.refresh_performance_mode {
                RefreshPerformanceMode::Realtime => DWM_IDLE_SYNC_EVERY_TICKS_REALTIME,
                RefreshPerformanceMode::Balanced => DWM_IDLE_SYNC_EVERY_TICKS,
                RefreshPerformanceMode::BatterySaver => DWM_IDLE_SYNC_EVERY_TICKS_BATTERY_SAVER,
                RefreshPerformanceMode::Manual => {
                    manual_dwm_idle_sync_every_ticks(state_ref.settings.refresh_interval_ms)
                }
            }
        })
}

pub(crate) const fn manual_dwm_idle_sync_every_ticks(refresh_interval_ms: u32) -> u8 {
    if refresh_interval_ms <= 1_000 {
        2
    } else if refresh_interval_ms <= 2_000 {
        DWM_IDLE_SYNC_EVERY_TICKS
    } else if refresh_interval_ms <= 5_000 {
        DWM_IDLE_SYNC_EVERY_TICKS_BATTERY_SAVER
    } else {
        DWM_IDLE_SYNC_EVERY_TICKS_MANUAL_SLOW
    }
}

fn host_window_is_visible(state: &Rc<RefCell<AppState>>) -> bool {
    state.try_borrow().is_ok_and(|state_ref| {
        !state_ref.shell.hwnd.0.is_null() && unsafe { IsWindowVisible(state_ref.shell.hwnd).as_bool() }
    })
}

pub(crate) fn effective_refresh_interval_ms(state: &Rc<RefCell<AppState>>) -> u32 {
    state
        .try_borrow()
        .map_or(DEFAULT_REFRESH_TIMER_INTERVAL_MS, |state_ref| {
            state_ref
                .settings
                .refresh_interval_ms
                .max(MIN_REFRESH_TIMER_INTERVAL_MS)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn manual_dwm_idle_cadence_tracks_refresh_interval_buckets() {
        assert_eq!(manual_dwm_idle_sync_every_ticks(1_000), 2);
        assert_eq!(manual_dwm_idle_sync_every_ticks(2_000), 4);
        assert_eq!(manual_dwm_idle_sync_every_ticks(5_000), 8);
        assert_eq!(manual_dwm_idle_sync_every_ticks(10_000), 12);
    }

    #[test]
    fn idle_scheduler_triggers_on_configured_tick_cadence() {
        let ticks = Rc::new(Cell::new(0u8));

        assert!(!schedule_idle_dwm_sync(&ticks, 4));
        assert!(!schedule_idle_dwm_sync(&ticks, 4));
        assert!(!schedule_idle_dwm_sync(&ticks, 4));
        assert!(schedule_idle_dwm_sync(&ticks, 4));
        assert_eq!(ticks.get(), 0);
    }
}
