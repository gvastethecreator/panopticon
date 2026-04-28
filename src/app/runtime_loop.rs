//! Runtime loop wiring for the main window lifecycle and recurring timers.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use slint::{CloseRequestResponse, ComponentHandle, Timer, TimerMode};
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use crate::{AppState, MainWindow, PendingAction, PENDING_ACTIONS, UI_STATE};

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
    let native_init_retry_timer = Rc::new(Timer::default());
    let floating_size_sync_timer = Rc::new(Timer::default());

    let ui_timer = Timer::default();
    ui_timer.start(TimerMode::Repeated, Duration::from_millis(16), {
        let state = state.clone();
        let weak = main_window.as_weak();
        let floating_size_sync_timer = floating_size_sync_timer.clone();
        let native_init_retry_timer = native_init_retry_timer.clone();
        let refresh_recompute_pending = refresh_recompute_pending.clone();
        move || {
            let Some(win) = weak.upgrade() else {
                return;
            };

            if state.borrow().hwnd.0.is_null() {
                schedule_native_runtime_retry(&state, &weak, &native_init_retry_timer, &win);
                return;
            }

            if let Some(outcome) = crate::app::updates::poll_latest_release_check() {
                crate::app::runtime_support::apply_update_check_outcome(&state, outcome);
            }

            drain_pending_actions(&state, &win);
            process_window_resize(&state, &win, &floating_size_sync_timer);

            if refresh_recompute_pending.replace(false) {
                crate::recompute_and_update_ui(&state, &win);
            }

            crate::advance_animation(&state, &win);
            crate::app::theme_ui::advance_theme_animation(&state, &win);
            crate::app::dwm::update_dwm_thumbnails(&state, &win);
        }
    });
    ui_timer
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
                if !main_window_is_visible() {
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

fn drain_pending_actions(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    PENDING_ACTIONS.with(|queue_cell| {
        let mut queue = queue_cell.borrow_mut();
        if queue.is_empty() {
            return;
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
    });
}

fn process_window_resize(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
    floating_size_sync_timer: &Rc<Timer>,
) {
    let phys_size = win.window().size();
    let scale = win.window().scale_factor();
    let logical_w = (phys_size.width as f32 / scale).round() as i32;
    let logical_h = (phys_size.height as f32 / scale).round() as i32;
    let needs_relayout = {
        let state_ref = state.borrow();
        logical_w != state_ref.last_size.0 || logical_h != state_ref.last_size.1
    };

    if !needs_relayout {
        return;
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
    crate::recompute_and_update_ui(state, win);
}

fn main_window_is_visible() -> bool {
    UI_STATE.with(|state| {
        state.borrow().as_ref().is_some_and(|state| {
            state
                .try_borrow()
                .is_ok_and(|state_ref| unsafe { IsWindowVisible(state_ref.hwnd).as_bool() })
        })
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
