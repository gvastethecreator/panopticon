//! Runtime loop wiring for the main window lifecycle and recurring timers.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use crate::{AppState, MainWindow, PendingAction, UpdateStatus, PENDING_ACTIONS};

const DWM_IDLE_SYNC_EVERY_TICKS: u8 = 4;
const PERF_REPORT_EVERY_TICKS: u16 = 60;

#[derive(Clone)]
struct UiPerfCounters {
    ticks: Rc<Cell<u16>>,
    action_batches: Rc<Cell<u16>>,
    recompute_resize: Rc<Cell<u16>>,
    recompute_refresh: Rc<Cell<u16>>,
    dwm_syncs: Rc<Cell<u16>>,
}

impl UiPerfCounters {
    fn new() -> Self {
        Self {
            ticks: Rc::new(Cell::new(0u16)),
            action_batches: Rc::new(Cell::new(0u16)),
            recompute_resize: Rc::new(Cell::new(0u16)),
            recompute_refresh: Rc::new(Cell::new(0u16)),
            dwm_syncs: Rc::new(Cell::new(0u16)),
        }
    }
}

#[derive(Clone)]
struct UiTickState {
    native_init_retry_timer: Rc<Timer>,
    floating_size_sync_timer: Rc<Timer>,
    refresh_recompute_pending: Rc<Cell<bool>>,
    dwm_idle_ticks: Rc<Cell<u8>>,
    last_viewport: Rc<Cell<Option<(f32, f32)>>>,
    perf_counters: UiPerfCounters,
}

impl UiTickState {
    fn new(refresh_recompute_pending: &Rc<Cell<bool>>) -> Self {
        Self {
            native_init_retry_timer: Rc::new(Timer::default()),
            floating_size_sync_timer: Rc::new(Timer::default()),
            refresh_recompute_pending: refresh_recompute_pending.clone(),
            dwm_idle_ticks: Rc::new(Cell::new(0u8)),
            last_viewport: Rc::new(Cell::new(None::<(f32, f32)>)),
            perf_counters: UiPerfCounters::new(),
        }
    }
}

#[derive(Copy, Clone, Default)]
struct UiTickSignals(u8);

impl UiTickSignals {
    const HAD_ACTIONS: u8 = 1 << 0;
    const RECOMPUTED_FROM_RESIZE: u8 = 1 << 1;
    const RECOMPUTED_FROM_REFRESH: u8 = 1 << 2;
    const SYNCED_DWM: u8 = 1 << 3;

    fn insert(&mut self, flag: u8) {
        self.0 |= flag;
    }

    fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

/// Keeps the recurring Slint timers alive for the lifetime of `run_app`.
pub(crate) struct RuntimeLoop {
    _init: Timer,
    _ui: Timer,
    _refresh: Timer,
    _scrollbar: Timer,
}

pub(crate) fn install_close_behavior(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) {
    main_window.window().on_close_requested({
        let state = state.clone();
        move || {
            let state_ref = state.borrow();
            if state_ref.settings.close_to_tray {
                drop(state_ref);
                crate::app::dwm::release_all_thumbnails(&state);
                CloseRequestResponse::HideWindow
            } else {
                drop(state_ref);
                crate::queue_exit_request();
                CloseRequestResponse::KeepWindowShown
            }
        }
    });
}

pub(crate) fn start(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) -> RuntimeLoop {
    let refresh_recompute_pending = Rc::new(Cell::new(false));
    let init_timer = start_initialization_timer(main_window, state);
    let ui_timer = start_ui_timer(main_window, state, &refresh_recompute_pending);
    let refresh_timer = start_refresh_timer(state, &refresh_recompute_pending);
    let scrollbar_timer = start_scrollbar_timer(main_window);

    RuntimeLoop {
        _init: init_timer,
        _ui: ui_timer,
        _refresh: refresh_timer,
        _scrollbar: scrollbar_timer,
    }
}

fn start_initialization_timer(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) -> Timer {
    let init_timer = Timer::default();
    init_timer.start(TimerMode::SingleShot, Duration::from_millis(0), {
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            let Some(win) = weak.upgrade() else {
                return;
            };
            let _ = crate::app::native_runtime::try_initialize_native_runtime(&state, &win);
        }
    });
    init_timer
}

fn start_ui_timer(
    main_window: &MainWindow,
    state: &Rc<RefCell<AppState>>,
    refresh_recompute_pending: &Rc<Cell<bool>>,
) -> Timer {
    let tick_state = UiTickState::new(refresh_recompute_pending);

    let ui_timer = Timer::default();
    ui_timer.start(TimerMode::Repeated, Duration::from_millis(16), {
        let state = state.clone();
        let weak = main_window.as_weak();
        let tick_state = tick_state.clone();
        move || {
            run_ui_tick(&state, &weak, &tick_state);
        }
    });
    ui_timer
}

fn run_ui_tick(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    tick_state: &UiTickState,
) {
    let Some(win) = weak.upgrade() else {
        return;
    };

    if state.borrow().hwnd.0.is_null() {
        schedule_native_runtime_retry(state, weak, &tick_state.native_init_retry_timer, &win);
        return;
    }

    let should_poll_updates = state
        .try_borrow()
        .is_ok_and(|state_ref| matches!(state_ref.update_status, UpdateStatus::Checking));
    if should_poll_updates {
        if let Some(outcome) = crate::app::updates::poll_latest_release_check() {
            crate::app::runtime_support::apply_update_check_outcome(state, outcome);
        }
    }

    let had_actions = drain_pending_actions(state, &win);

    if !host_window_is_visible(state) {
        return;
    }

    let recomputed_from_resize =
        process_window_resize(state, &win, &tick_state.floating_size_sync_timer);
    let recomputed_from_refresh = tick_state.refresh_recompute_pending.replace(false);
    if recomputed_from_resize || recomputed_from_refresh {
        crate::recompute_and_update_ui(state, &win);
    }

    let viewport_changed = update_and_detect_viewport_change(&win, &tick_state.last_viewport);
    let (window_animation_active, theme_animation_active, is_animating_or_dirty) =
        runtime_activity_flags(state);

    let should_sync_dwm = if had_actions
        || recomputed_from_resize
        || recomputed_from_refresh
        || viewport_changed
        || is_animating_or_dirty
    {
        tick_state.dwm_idle_ticks.set(0);
        true
    } else {
        schedule_idle_dwm_sync(&tick_state.dwm_idle_ticks)
    };

    if window_animation_active {
        crate::advance_animation(state, &win);
    }
    if theme_animation_active {
        crate::app::theme_ui::advance_theme_animation(state, &win);
    }
    if should_sync_dwm {
        crate::app::dwm::update_dwm_thumbnails(state, &win);
    }

    if tracing::enabled!(tracing::Level::TRACE) {
        let mut tick_signals = UiTickSignals::default();
        if had_actions {
            tick_signals.insert(UiTickSignals::HAD_ACTIONS);
        }
        if recomputed_from_resize {
            tick_signals.insert(UiTickSignals::RECOMPUTED_FROM_RESIZE);
        }
        if recomputed_from_refresh {
            tick_signals.insert(UiTickSignals::RECOMPUTED_FROM_REFRESH);
        }
        if should_sync_dwm {
            tick_signals.insert(UiTickSignals::SYNCED_DWM);
        }

        record_perf_tick(&tick_state.perf_counters, tick_signals);
    }
}

fn runtime_activity_flags(state: &Rc<RefCell<AppState>>) -> (bool, bool, bool) {
    state
        .try_borrow()
        .map_or((false, false, false), |state_ref| {
            let window_animation_active = state_ref.animation_started_at.is_some();
            let theme_animation_active = state_ref.theme_animation.is_some();
            let is_animating_or_dirty = window_animation_active
                || theme_animation_active
                || state_ref.drag_separator.is_some()
                || state_ref
                    .windows
                    .iter()
                    .any(|managed_window| managed_window.thumbnail.is_none());
            (
                window_animation_active,
                theme_animation_active,
                is_animating_or_dirty,
            )
        })
}

fn start_refresh_timer(
    state: &Rc<RefCell<AppState>>,
    refresh_recompute_pending: &Rc<Cell<bool>>,
) -> Timer {
    let refresh_timer = Timer::default();
    refresh_timer.start(
        TimerMode::Repeated,
        Duration::from_millis((state.borrow().settings.refresh_interval_ms as u64).max(50)),
        {
            let state = state.clone();
            let refresh_recompute_pending = refresh_recompute_pending.clone();
            move || {
                if !host_window_is_visible(&state) {
                    return;
                }
                if crate::refresh_windows(&state) {
                    refresh_recompute_pending.set(true);
                }
            }
        },
    );
    refresh_timer
}

fn start_scrollbar_timer(main_window: &MainWindow) -> Timer {
    let scrollbar_timer = Timer::default();
    scrollbar_timer.start(TimerMode::Repeated, Duration::from_millis(500), {
        let weak = main_window.as_weak();
        move || {
            crate::app::window_subclass::hide_scrollbar_if_idle(&weak);
        }
    });
    scrollbar_timer
}

fn schedule_native_runtime_retry(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    native_init_retry_timer: &Rc<Timer>,
    win: &MainWindow,
) {
    if native_init_retry_timer.running()
        || crate::app::native_runtime::try_initialize_native_runtime(state, win)
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
                let _ = crate::app::native_runtime::try_initialize_native_runtime(
                    &state_retry,
                    &win_retry,
                );
            }
        },
    );
}

fn drain_pending_actions(state: &Rc<RefCell<AppState>>, win: &MainWindow) -> bool {
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

fn process_window_resize(
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
        logical_w != state_ref.last_size.0 || logical_h != state_ref.last_size.1
    };

    if !needs_relayout {
        return false;
    }

    {
        let mut state_ref = state.borrow_mut();
        state_ref.last_size = (logical_w, logical_h);
    }

    crate::app::runtime_support::sync_floating_window_size_with_resize(
        state,
        logical_w,
        logical_h,
        floating_size_sync_timer,
    );
    true
}

fn update_and_detect_viewport_change(
    win: &MainWindow,
    last_viewport: &Rc<Cell<Option<(f32, f32)>>>,
) -> bool {
    let current = (win.get_viewport_x(), win.get_viewport_y());
    let previous = last_viewport.get();
    last_viewport.set(Some(current));
    previous.is_none_or(|(x, y)| {
        (current.0 - x).abs() > f32::EPSILON || (current.1 - y).abs() > f32::EPSILON
    })
}

fn schedule_idle_dwm_sync(dwm_idle_ticks: &Rc<Cell<u8>>) -> bool {
    let next = dwm_idle_ticks.get().saturating_add(1);
    if next >= DWM_IDLE_SYNC_EVERY_TICKS {
        dwm_idle_ticks.set(0);
        true
    } else {
        dwm_idle_ticks.set(next);
        false
    }
}

fn record_perf_tick(perf: &UiPerfCounters, signals: UiTickSignals) {
    let ticks = perf.ticks.get().saturating_add(1);
    perf.ticks.set(ticks);

    if signals.contains(UiTickSignals::HAD_ACTIONS) {
        perf.action_batches
            .set(perf.action_batches.get().saturating_add(1));
    }
    if signals.contains(UiTickSignals::RECOMPUTED_FROM_RESIZE) {
        perf.recompute_resize
            .set(perf.recompute_resize.get().saturating_add(1));
    }
    if signals.contains(UiTickSignals::RECOMPUTED_FROM_REFRESH) {
        perf.recompute_refresh
            .set(perf.recompute_refresh.get().saturating_add(1));
    }
    if signals.contains(UiTickSignals::SYNCED_DWM) {
        perf.dwm_syncs.set(perf.dwm_syncs.get().saturating_add(1));
    }

    if ticks < PERF_REPORT_EVERY_TICKS {
        return;
    }

    tracing::trace!(
        ticks,
        action_batches = perf.action_batches.get(),
        recompute_resize = perf.recompute_resize.get(),
        recompute_refresh = perf.recompute_refresh.get(),
        dwm_syncs = perf.dwm_syncs.get(),
        "runtime loop perf counters"
    );

    perf.ticks.set(0);
    perf.action_batches.set(0);
    perf.recompute_resize.set(0);
    perf.recompute_refresh.set(0);
    perf.dwm_syncs.set(0);
}

fn host_window_is_visible(state: &Rc<RefCell<AppState>>) -> bool {
    state.try_borrow().is_ok_and(|state_ref| {
        !state_ref.hwnd.0.is_null() && unsafe { IsWindowVisible(state_ref.hwnd).as_bool() }
    })
}

fn handle_pending_action(state: &Rc<RefCell<AppState>>, win: &MainWindow, action: PendingAction) {
    let weak = win.as_weak();
    match action {
        PendingAction::Tray(action, anchor) => {
            crate::app::tray_actions::handle_tray_action(state, &weak, action, anchor);
        }
        PendingAction::ActivateMainWindow => {
            crate::app::tray_actions::activate_main_window(state, &weak);
        }
        PendingAction::Reposition => {
            if let Ok(mut state_ref) = state.try_borrow_mut() {
                if state_ref.is_appbar {
                    crate::app::dock::reposition_appbar(&mut state_ref);
                }
            }
        }
        PendingAction::HideToTray => {
            crate::app::dwm::release_all_thumbnails(state);
            win.hide().ok();
        }
        PendingAction::Refresh => {
            if crate::refresh_windows(state) {
                crate::recompute_and_update_ui(state, win);
            }
        }
        PendingAction::Exit => {
            crate::app::native_runtime::request_exit(state);
        }
    }
}
