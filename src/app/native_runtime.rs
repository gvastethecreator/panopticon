//! Native HWND extraction, runtime bootstrap, and shutdown helpers.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::ComponentHandle;
use windows::Win32::Foundation::HWND;

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, sync_dock_system_menu,
    unregister_appbar,
};
use super::dwm::release_thumbnail;
use super::tray::{apply_window_icons, TrayIcon};
use crate::{AppState, MainWindow, SETTINGS_WIN};

pub(crate) fn get_hwnd(window: &slint::Window) -> Option<HWND> {
    let slint_handle = window.window_handle();
    let raw = slint_handle.window_handle().ok()?;
    match raw.as_raw() {
        RawWindowHandle::Win32(handle) => Some(HWND(handle.hwnd.get() as *mut c_void)),
        _ => None,
    }
}

pub(crate) fn try_initialize_native_runtime(
    state: &Rc<RefCell<AppState>>,
    win: &MainWindow,
) -> bool {
    if !state.borrow().hwnd.0.is_null() {
        return true;
    }

    let Some(hwnd) = get_hwnd(win.window()) else {
        tracing::debug!("HWND not ready yet; deferring native initialization");
        return false;
    };

    {
        let mut state = state.borrow_mut();
        if !state.hwnd.0.is_null() {
            return true;
        }
        state.hwnd = hwnd;
    }

    let settings_snapshot = state.borrow().settings.clone();
    tracing::info!(hwnd = ?hwnd, "native HWND acquired");

    {
        let state = state.borrow();
        apply_window_icons(hwnd, &state.icons);
    }

    apply_window_appearance(hwnd, &settings_snapshot);
    apply_topmost_mode(hwnd, settings_snapshot.always_on_top);
    sync_dock_system_menu(hwnd, settings_snapshot.dock_edge.is_some());

    {
        let mut state = state.borrow_mut();
        match TrayIcon::add(hwnd, state.icons.small) {
            Ok(tray) => {
                tracing::info!("tray icon registered");
                state.tray_icon = Some(tray);
            }
            Err(error) => tracing::error!(%error, "tray icon registration failed"),
        }
    }

    super::window_subclass::setup_subclass(hwnd, state, win);

    let refreshed = crate::refresh_windows(state);
    let tracked = state.borrow().windows.len();
    tracing::info!(
        refreshed,
        tracked_windows = tracked,
        "initial window refresh completed"
    );
    crate::recompute_and_update_ui(state, win);

    if settings_snapshot.dock_edge.is_some() {
        let mut state = state.borrow_mut();
        apply_dock_mode(&mut state);
    }

    if settings_snapshot.start_in_tray {
        tracing::info!("start_in_tray is active — hiding main window");
        for managed_window in &mut state.borrow_mut().windows {
            release_thumbnail(managed_window);
        }
        win.hide().ok();
    }

    true
}

pub(crate) fn request_exit(state: &Rc<RefCell<AppState>>) {
    tracing::info!("exiting Panopticon");
    {
        let mut state = state.borrow_mut();
        if state.is_appbar {
            unregister_appbar(state.hwnd);
            state.is_appbar = false;
        }
        for managed_window in &mut state.windows {
            release_thumbnail(managed_window);
        }
        state.windows.clear();
        if let Some(tray) = state.tray_icon.as_mut() {
            tray.remove();
        }
    }
    SETTINGS_WIN.with(|handle| {
        handle.borrow_mut().take();
    });
    slint::quit_event_loop().ok();
}
