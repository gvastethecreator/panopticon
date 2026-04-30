//! Shared runtime helpers extracted from the binary entry point.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use panopticon::settings::{AppSettings, MIN_FIXED_WINDOW_HEIGHT, MIN_FIXED_WINDOW_WIDTH};
use slint::{Timer, TimerMode};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

use crate::{AppState, MainWindow, UpdateStatus};
use super::model_sync::recompute_and_update_ui;
use super::window_sync::refresh_windows;

const FLOATING_SIZE_SYNC_DEBOUNCE_MS: u64 = 220;

pub(crate) fn request_update_check(state: &Rc<RefCell<AppState>>, user_initiated: bool) -> bool {
    {
        let mut guard = state.borrow_mut();
        if matches!(guard.update_status, UpdateStatus::Checking) {
            return false;
        }
        guard.update_status = UpdateStatus::Checking;
    }
    crate::app::secondary_windows::refresh_open_settings_window(state);
    crate::app::secondary_windows::refresh_open_about_window(state);

    if !crate::app::updates::request_latest_release_check(env!("CARGO_PKG_VERSION")) {
        let mut guard = state.borrow_mut();
        guard.update_status = UpdateStatus::Failed;
        drop(guard);
        crate::app::secondary_windows::refresh_open_settings_window(state);
        crate::app::secondary_windows::refresh_open_about_window(state);
        if user_initiated {
            tracing::warn!("manual update-check request could not be started");
        }
        return false;
    }

    true
}

pub(crate) fn apply_update_check_outcome(
    state: &Rc<RefCell<AppState>>,
    outcome: crate::app::updates::UpdateCheckOutcome,
) {
    let next_status = match outcome {
        crate::app::updates::UpdateCheckOutcome::UpToDate { latest_version } => {
            UpdateStatus::UpToDate { latest_version }
        }
        crate::app::updates::UpdateCheckOutcome::Available {
            latest_version,
            release_url,
        } => {
            tracing::info!(%latest_version, %release_url, "new release available");
            UpdateStatus::Available {
                latest_version,
                release_url,
            }
        }
        crate::app::updates::UpdateCheckOutcome::Failed { reason } => {
            tracing::warn!(%reason, "update check failed");
            UpdateStatus::Failed
        }
    };

    state.borrow_mut().update_status = next_status;
    crate::app::secondary_windows::refresh_open_settings_window(state);
    crate::app::secondary_windows::refresh_open_about_window(state);
}

pub(crate) fn sync_floating_window_size_with_resize(
    state: &Rc<RefCell<AppState>>,
    logical_w: i32,
    logical_h: i32,
    size_sync_timer: &Rc<Timer>,
) {
    let Ok(width) = u32::try_from(logical_w) else {
        return;
    };
    let Ok(height) = u32::try_from(logical_h) else {
        return;
    };
    if width == 0 || height == 0 {
        return;
    }

    let clamped_width = width.max(MIN_FIXED_WINDOW_WIDTH);
    let clamped_height = height.max(MIN_FIXED_WINDOW_HEIGHT);

    {
        let mut guard = state.borrow_mut();
        if guard.settings.dock_edge.is_some() {
            return;
        }
        guard.settings.fixed_width = Some(clamped_width);
        guard.settings.fixed_height = Some(clamped_height);
    }

    let state_for_save = state.clone();
    size_sync_timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(FLOATING_SIZE_SYNC_DEBOUNCE_MS),
        move || {
            let mut guard = state_for_save.borrow_mut();
            if guard.settings.dock_edge.is_some() {
                return;
            }
            guard.settings = guard.settings.normalized();
            if let Err(error) = guard.settings.save(guard.workspace_name.as_deref()) {
                tracing::warn!(%error, "failed to persist floating window size after resize");
            }
            drop(guard);
            crate::app::secondary_windows::refresh_open_settings_window(&state_for_save);
        },
    );
}

pub(crate) fn update_settings(
    state: &Rc<RefCell<AppState>>,
    mutate: impl FnOnce(&mut AppSettings),
) {
    let (hwnd, settings_snapshot, workspace_name) = {
        let mut state = state.borrow_mut();
        mutate(&mut state.settings);
        state.settings = state.settings.normalized();
        let _ = state.settings.save(state.workspace_name.as_deref());
        (
            state.shell.hwnd,
            state.settings.clone(),
            state.workspace_name.clone(),
        )
    };
    crate::app::startup::sync_run_at_startup(
        settings_snapshot.run_at_startup,
        workspace_name.as_deref(),
    );
    crate::app::global_hotkey::sync_activate_hotkey(hwnd, &settings_snapshot);
}

pub(crate) fn refresh_ui(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
        crate::app::theme_ui::advance_theme_animation(state, &win);
    }
    crate::app::secondary_windows::refresh_open_settings_window(state);
}

/// Schedule an immediate refresh + a deferred one (300 ms) so that
/// closed/killed windows disappear promptly even if the process takes
/// a moment to terminate.
pub(crate) fn schedule_deferred_refresh(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
) {
    let _ = refresh_windows(state);
    refresh_ui(state, weak);

    let state2 = state.clone();
    let weak2 = weak.clone();
    let timer = Timer::default();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(300),
        move || {
            if refresh_windows(&state2) {
                if let Some(win) = weak2.upgrade() {
                    recompute_and_update_ui(&state2, &win);
                }
            }
        },
    );
    // Intentional: the Slint event loop owns the timer until it fires;
    // dropping it here would cancel the callback. `forget` transfers
    // ownership to the event loop (no real leak for SingleShot timers).
    std::mem::forget(timer);
}

pub(crate) fn logical_to_screen_point(hwnd: HWND, logical_x: f32, logical_y: f32) -> POINT {
    let mut window_rect = RECT::default();
    // SAFETY: hwnd is our live window; window_rect is stack-allocated and valid.
    unsafe {
        let _ = GetWindowRect(hwnd, &raw mut window_rect);
    }

    POINT {
        x: window_rect.left + logical_x.round() as i32,
        y: window_rect.top + logical_y.round() as i32,
    }
}
