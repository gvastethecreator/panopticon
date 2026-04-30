//! Native HWND extraction, runtime bootstrap, and shutdown helpers.

use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

use panopticon::settings::AppSettings;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{ComponentHandle, LogicalSize};
use windows::Win32::Foundation::HWND;

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, center_window_on_owner_monitor,
    sync_dock_system_menu, unregister_appbar,
};
use super::dwm::release_thumbnail;
use super::global_hotkey;
use super::tray::{apply_window_icons, TrayIcon};
use crate::{AppState, MainWindow, ABOUT_WIN, SETTINGS_WIN, TAG_DIALOG_WIN};
use super::model_sync::recompute_and_update_ui;
use super::window_sync::refresh_windows;

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
    if !state.borrow().shell.hwnd.0.is_null() {
        return true;
    }

    let Some(hwnd) = get_hwnd(win.window()) else {
        tracing::debug!("HWND not ready yet; deferring native initialization");
        return false;
    };

    {
        let mut state = state.borrow_mut();
        if !state.shell.hwnd.0.is_null() {
            return true;
        }
        state.shell.hwnd = hwnd;
    }

    let settings_snapshot = state.borrow().settings.clone();
    tracing::info!(hwnd = ?hwnd, "native HWND acquired");

    {
        let state = state.borrow();
        apply_window_icons(hwnd, &state.shell.icons);
    }

    apply_window_appearance(hwnd, &settings_snapshot);
    apply_topmost_mode(hwnd, settings_snapshot.always_on_top);
    let _ = apply_configured_main_window_size(win, &settings_snapshot);
    if settings_snapshot.dock_edge.is_none() {
        center_window_on_owner_monitor(hwnd, HWND::default());
    }
    sync_dock_system_menu(hwnd, settings_snapshot.dock_edge.is_some());

    {
        let mut state = state.borrow_mut();
        match TrayIcon::add(hwnd, state.shell.icons.small) {
            Ok(tray) => {
                tracing::info!("tray icon registered");
                state.shell.tray_icon = Some(tray);
            }
            Err(error) => tracing::error!(%error, "tray icon registration failed"),
        }
    }

    super::window_subclass::setup_subclass(hwnd, state, win);
    global_hotkey::sync_activate_hotkey(hwnd, &settings_snapshot);

    let refreshed = refresh_windows(state);
    let tracked = state.borrow().window_collection.windows.len();
    tracing::info!(
        refreshed,
        tracked_windows = tracked,
        "initial window refresh completed"
    );
    recompute_and_update_ui(state, win);

    if settings_snapshot.dock_edge.is_some() {
        let mut state = state.borrow_mut();
        apply_dock_mode(&mut state);
    }

    if settings_snapshot.start_in_tray {
        tracing::info!("start_in_tray is active — hiding main window");
        for managed_window in &mut state.borrow_mut().window_collection.windows {
            release_thumbnail(managed_window);
        }
        win.hide().ok();
    }

    true
}

pub(crate) fn apply_configured_main_window_size(win: &MainWindow, settings: &AppSettings) -> bool {
    let current_size = LogicalSize::from_physical(win.window().size(), win.window().scale_factor());
    let Some(target_size) = configured_floating_window_size(current_size, settings) else {
        return false;
    };

    if (target_size.width - current_size.width).abs() < 0.5
        && (target_size.height - current_size.height).abs() < 0.5
    {
        return false;
    }

    win.window().set_size(target_size);
    true
}

fn configured_floating_window_size(
    current_size: LogicalSize,
    settings: &AppSettings,
) -> Option<LogicalSize> {
    if settings.dock_edge.is_some()
        || (settings.fixed_width.is_none() && settings.fixed_height.is_none())
    {
        return None;
    }

    Some(LogicalSize::new(
        settings
            .fixed_width
            .map_or(current_size.width, |width| width as f32),
        settings
            .fixed_height
            .map_or(current_size.height, |height| height as f32),
    ))
}

pub(crate) fn request_exit(state: &Rc<RefCell<AppState>>) {
    tracing::info!("exiting Panopticon");
    {
        let mut state = state.borrow_mut();
        global_hotkey::unregister_activate_hotkey(state.shell.hwnd);
        if state.shell.is_appbar {
            unregister_appbar(state.shell.hwnd);
            state.shell.is_appbar = false;
        }
        for managed_window in &mut state.window_collection.windows {
            release_thumbnail(managed_window);
        }
        state.window_collection.windows.clear();
        if let Some(tray) = state.shell.tray_icon.as_mut() {
            tray.remove();
        }
    }
    SETTINGS_WIN.with(|handle| {
        handle.borrow_mut().take();
    });
    TAG_DIALOG_WIN.with(|handle| {
        handle.borrow_mut().take();
    });
    ABOUT_WIN.with(|handle| {
        handle.borrow_mut().take();
    });
    slint::quit_event_loop().ok();
}

#[cfg(test)]
mod tests {
    use super::configured_floating_window_size;
    use panopticon::settings::{AppSettings, DockEdge};
    use slint::LogicalSize;

    #[test]
    fn floating_window_size_uses_fixed_dimensions_when_undocked() {
        let settings = AppSettings {
            fixed_width: Some(900),
            fixed_height: Some(700),
            ..AppSettings::default()
        };

        let size = configured_floating_window_size(LogicalSize::new(1320.0, 840.0), &settings);

        assert_eq!(size, Some(LogicalSize::new(900.0, 700.0)));
    }

    #[test]
    fn floating_window_size_preserves_unspecified_dimension() {
        let settings = AppSettings {
            fixed_width: Some(960),
            ..AppSettings::default()
        };

        let size = configured_floating_window_size(LogicalSize::new(1320.0, 840.0), &settings);

        assert_eq!(size, Some(LogicalSize::new(960.0, 840.0)));
    }

    #[test]
    fn floating_window_size_is_disabled_while_docked_or_without_overrides() {
        let current = LogicalSize::new(1320.0, 840.0);
        let settings = AppSettings::default();

        assert_eq!(configured_floating_window_size(current, &settings), None);

        let docked_settings = AppSettings {
            fixed_width: Some(500),
            dock_edge: Some(DockEdge::Left),
            ..AppSettings::default()
        };
        assert_eq!(
            configured_floating_window_size(current, &docked_settings),
            None
        );
    }
}
