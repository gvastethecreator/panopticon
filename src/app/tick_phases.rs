//! Discrete phases of the UI tick loop.
//!
//! Each phase is a standalone function with a small, explicit interface.
//! [`run_ui_tick`] orchestrates them in order and accumulates their effects
//! into a [`TickEffects`] struct.  This makes dependencies between phases
//! explicit and allows individual phases to be tested in isolation.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{ComponentHandle, Timer, TimerMode};

use super::model_sync::{advance_animation, recompute_and_update_ui};
use super::window_sync::refresh_windows;
use crate::{AppState, MainWindow, PendingAction, UpdateStatus, PENDING_ACTIONS};
use panopticon::settings::RefreshPerformanceMode;

// ───────────────────────── Constants ─────────────────────────

const DEFAULT_REFRESH_TIMER_INTERVAL_MS: u32 = 2_000;
const MIN_REFRESH_TIMER_INTERVAL_MS: u32 = 50;
const DWM_IDLE_SYNC_INTERVAL_MS: u64 = 250;
const DWM_IDLE_SYNC_INTERVAL_MS_REALTIME: u64 = 64;
const DWM_IDLE_SYNC_INTERVAL_MS_BATTERY_SAVER: u64 = 500;
const DWM_IDLE_SYNC_INTERVAL_MS_MANUAL_MAX: u64 = 2_000;

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
    pub is_animating_or_dirty: bool,
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
pub(crate) fn drain_actions(state: &Rc<RefCell<AppState>>, win: &MainWindow) -> bool {
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
    _state: &Rc<RefCell<AppState>>,
    _win: &MainWindow,
    refresh_recompute_pending: &Cell<bool>,
) -> bool {
    refresh_recompute_pending.replace(false)
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
pub(crate) fn compute_activity_flags(state: &Rc<RefCell<AppState>>) -> (bool, bool, bool) {
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
    last_dwm_sync: &Cell<Option<Instant>>,
    state: &Rc<RefCell<AppState>>,
) -> bool {
    let now = Instant::now();
    if effects.needs_immediate_dwm_sync() {
        last_dwm_sync.set(Some(now));
        true
    } else {
        let interval = current_dwm_idle_sync_interval(state);
        schedule_idle_dwm_sync(last_dwm_sync, now, interval)
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
        super::presentation::advance_theme_animation(state, win);
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
                let _ =
                    super::native_runtime::try_initialize_native_runtime(&state_retry, &win_retry);
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

pub(crate) fn schedule_idle_dwm_sync(
    last_sync: &Cell<Option<Instant>>,
    now: Instant,
    interval: Duration,
) -> bool {
    if last_sync
        .get()
        .is_none_or(|previous| now.duration_since(previous) >= interval)
    {
        last_sync.set(Some(now));
        true
    } else {
        false
    }
}

fn current_dwm_idle_sync_interval(state: &Rc<RefCell<AppState>>) -> Duration {
    state.try_borrow().map_or(
        Duration::from_millis(DWM_IDLE_SYNC_INTERVAL_MS),
        |state_ref| match state_ref.settings.refresh_performance_mode {
            RefreshPerformanceMode::Realtime => {
                Duration::from_millis(DWM_IDLE_SYNC_INTERVAL_MS_REALTIME)
            }
            RefreshPerformanceMode::Balanced => Duration::from_millis(DWM_IDLE_SYNC_INTERVAL_MS),
            RefreshPerformanceMode::BatterySaver => {
                Duration::from_millis(DWM_IDLE_SYNC_INTERVAL_MS_BATTERY_SAVER)
            }
            RefreshPerformanceMode::Manual => {
                Duration::from_millis(u64::from(state_ref.settings.refresh_interval_ms).clamp(
                    DWM_IDLE_SYNC_INTERVAL_MS,
                    DWM_IDLE_SYNC_INTERVAL_MS_MANUAL_MAX,
                ))
            }
        },
    )
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
    fn idle_scheduler_uses_elapsed_time_instead_of_tick_count() {
        let last_sync = Rc::new(Cell::new(None));
        let started = Instant::now();

        assert!(schedule_idle_dwm_sync(
            &last_sync,
            started,
            Duration::from_millis(250)
        ));
        assert!(!schedule_idle_dwm_sync(
            &last_sync,
            started + Duration::from_millis(249),
            Duration::from_millis(250)
        ));
        assert!(schedule_idle_dwm_sync(
            &last_sync,
            started + Duration::from_millis(250),
            Duration::from_millis(250)
        ));
    }
}
