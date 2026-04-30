//! Runtime loop wiring for the main window lifecycle and recurring timers.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};

use crate::{AppState, MainWindow};

const MIN_REFRESH_TIMER_INTERVAL_MS: u32 = 50;
const PERF_REPORT_EVERY_TICKS: u16 = 60;

#[derive(Clone)]
pub(crate) struct UiPerfCounters {
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
pub(crate) struct UiTickState {
    pub(crate) native_init_retry_timer: Rc<Timer>,
    pub(crate) refresh_timer: Rc<Timer>,
    pub(crate) floating_size_sync_timer: Rc<Timer>,
    pub(crate) last_refresh_interval_ms: Rc<Cell<u32>>,
    pub(crate) refresh_recompute_pending: Rc<Cell<bool>>,
    pub(crate) dwm_idle_ticks: Rc<Cell<u8>>,
    pub(crate) last_viewport: Rc<Cell<Option<(f32, f32)>>>,
    perf_counters: UiPerfCounters,
}

impl UiTickState {
    fn new(
        refresh_timer: &Rc<Timer>,
        refresh_recompute_pending: &Rc<Cell<bool>>,
        initial_refresh_interval_ms: u32,
    ) -> Self {
        Self {
            native_init_retry_timer: Rc::new(Timer::default()),
            refresh_timer: refresh_timer.clone(),
            floating_size_sync_timer: Rc::new(Timer::default()),
            last_refresh_interval_ms: Rc::new(Cell::new(initial_refresh_interval_ms)),
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
    _refresh: Rc<Timer>,
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
    let refresh_timer = Rc::new(Timer::default());
    let initial_refresh_interval_ms =
        start_refresh_timer(&refresh_timer, state, &refresh_recompute_pending);
    let init_timer = start_initialization_timer(main_window, state);
    let ui_timer = start_ui_timer(
        main_window,
        state,
        &refresh_timer,
        &refresh_recompute_pending,
        initial_refresh_interval_ms,
    );
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
    refresh_timer: &Rc<Timer>,
    refresh_recompute_pending: &Rc<Cell<bool>>,
    initial_refresh_interval_ms: u32,
) -> Timer {
    let tick_state = UiTickState::new(
        refresh_timer,
        refresh_recompute_pending,
        initial_refresh_interval_ms,
    );

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
    use super::tick_phases::{
        advance_theme_animation, advance_window_animation, compute_activity_flags,
        decide_dwm_sync, detect_resize, detect_viewport_change, drain_actions,
        poll_update_check, reconcile_refresh, sync_dwm, try_native_init, TickEffects,
    };

    let Some(win) = weak.upgrade() else {
        return;
    };

    if try_native_init(state, weak, &win, &tick_state.native_init_retry_timer) {
        return;
    }

    maybe_reconfigure_refresh_timer(state, tick_state);
    poll_update_check(state);

    let had_actions = drain_actions(state, &win);

    if !host_window_is_visible(state) {
        return;
    }

    let recomputed_from_resize = detect_resize(state, &win, &tick_state.floating_size_sync_timer);
    let recomputed_from_refresh =
        reconcile_refresh(state, &win, &tick_state.refresh_recompute_pending);
    if recomputed_from_resize || recomputed_from_refresh {
        super::model_sync::recompute_and_update_ui(state, &win);
    }

    let viewport_changed = detect_viewport_change(&win, &tick_state.last_viewport);
    let (window_animation_active, theme_animation_active, is_animating_or_dirty) =
        compute_activity_flags(state);

    let effects = TickEffects {
        had_actions,
        recomputed_from_resize,
        recomputed_from_refresh,
        viewport_changed,
        window_animation_active,
        theme_animation_active,
        is_animating_or_dirty,
        should_sync_dwm: false,
    };

    let should_sync_dwm = decide_dwm_sync(effects, &tick_state.dwm_idle_ticks, state);

    advance_window_animation(state, &win, window_animation_active);
    advance_theme_animation(state, &win, theme_animation_active);
    sync_dwm(state, &win, should_sync_dwm);

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

fn start_refresh_timer(
    refresh_timer: &Rc<Timer>,
    state: &Rc<RefCell<AppState>>,
    refresh_recompute_pending: &Rc<Cell<bool>>,
) -> u32 {
    let interval_ms = super::tick_phases::effective_refresh_interval_ms(state);
    restart_refresh_timer(refresh_timer, state, refresh_recompute_pending, interval_ms);
    interval_ms
}

fn restart_refresh_timer(
    refresh_timer: &Rc<Timer>,
    state: &Rc<RefCell<AppState>>,
    refresh_recompute_pending: &Rc<Cell<bool>>,
    interval_ms: u32,
) {
    use super::window_sync::refresh_windows;

    refresh_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(u64::from(interval_ms.max(MIN_REFRESH_TIMER_INTERVAL_MS))),
        {
            let state = state.clone();
            let refresh_recompute_pending = refresh_recompute_pending.clone();
            move || {
                if refresh_recompute_pending.get() {
                    return;
                }
                if !host_window_is_visible(&state) {
                    return;
                }
                if refresh_windows(&state) {
                    refresh_recompute_pending.set(true);
                }
            }
        },
    );
}

fn maybe_reconfigure_refresh_timer(state: &Rc<RefCell<AppState>>, tick_state: &UiTickState) {
    let next_interval_ms = super::tick_phases::effective_refresh_interval_ms(state);
    if next_interval_ms == tick_state.last_refresh_interval_ms.get() {
        return;
    }

    tick_state.last_refresh_interval_ms.set(next_interval_ms);
    restart_refresh_timer(
        &tick_state.refresh_timer,
        state,
        &tick_state.refresh_recompute_pending,
        next_interval_ms,
    );
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
    use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
    state.try_borrow().is_ok_and(|state_ref| {
        !state_ref.shell.hwnd.0.is_null() && unsafe { IsWindowVisible(state_ref.shell.hwnd).as_bool() }
    })
}


