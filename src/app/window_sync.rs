//! Window enumeration refresh and synchronization with managed state.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{apply_pinned_positions, sort_windows_for_grouping};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use super::dwm::hydrate_reconciled_thumbnails;
use super::icon::invalidate_cached_app_icon;
use super::managed_window_reconcile::reconcile_managed_windows;
use crate::AppState;

pub(crate) fn refresh_windows(state: &Rc<RefCell<AppState>>) -> bool {
    let mut state = state.borrow_mut();
    let host_hwnd = state.shell.hwnd;
    if host_hwnd.0.is_null() {
        return false;
    }
    let host_visible = unsafe {
        // SAFETY: read-only visibility query for the application's own top-level window.
        IsWindowVisible(host_hwnd).as_bool()
    };

    let mut discovered = collect_discovered_windows(&mut state, host_hwnd);

    sort_windows_for_grouping(&mut discovered, &state.settings);
    apply_pinned_positions(&mut discovered, &state.settings);

    let outcome = reconcile_managed_windows(&mut state.window_collection.windows, discovered);
    for app_id in &outcome.icon_invalidations {
        invalidate_cached_app_icon(app_id);
    }

    let dwm_changed = hydrate_reconciled_thumbnails(
        host_hwnd,
        host_visible,
        &mut state.window_collection.windows,
    );

    outcome.changed || dwm_changed
}

fn collect_discovered_windows(state: &mut AppState, host_hwnd: HWND) -> Vec<WindowInfo> {
    let discovered_all: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != host_hwnd)
        .collect();

    for window in &discovered_all {
        state
            .settings
            .refresh_app_label(&window.app_id, window.app_label());
    }

    let monitor_filter = state.settings.active_monitor_filter.clone();
    let tag_filter = state.settings.active_tag_filter.clone();
    let app_filter = state.settings.active_app_filter.clone();

    discovered_all
        .into_iter()
        .filter(|window| {
            monitor_filter
                .as_deref()
                .is_none_or(|monitor| window.monitor_name == monitor)
        })
        .filter(|window| {
            tag_filter
                .as_deref()
                .is_none_or(|tag| state.settings.app_has_tag(&window.app_id, tag))
        })
        .filter(|window| {
            app_filter
                .as_deref()
                .is_none_or(|app_id| window.app_id == app_id)
        })
        .filter(|window| !state.settings.is_hidden(&window.app_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::managed_window_reconcile::{new_managed_window, reconcile_managed_windows};
    use super::*;
    use std::ffi::c_void;

    fn window_info(
        hwnd_value: usize,
        title: &str,
        app_id: &str,
        process_name: &str,
        process_path: Option<&str>,
        class_name: &str,
        monitor_name: &str,
    ) -> WindowInfo {
        WindowInfo {
            hwnd: HWND(hwnd_value as *mut c_void),
            title: title.to_owned(),
            app_id: app_id.to_owned(),
            process_name: process_name.to_owned(),
            process_path: process_path.map(str::to_owned),
            class_name: class_name.to_owned(),
            monitor_name: monitor_name.to_owned(),
        }
    }

    #[test]
    fn window_metadata_changed_detects_any_tracked_difference() {
        let current = window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );
        let fresh = window_info(
            1,
            "Alpha renamed",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );

        let mut windows = vec![new_managed_window(current.clone())];
        let changed = reconcile_managed_windows(&mut windows, vec![fresh]).changed;
        assert!(changed);

        let mut windows = vec![new_managed_window(current.clone())];
        let unchanged = reconcile_managed_windows(&mut windows, vec![current]).changed;
        assert!(!unchanged);
    }

    #[test]
    fn should_reset_cached_icon_only_for_app_or_path_changes() {
        let current = window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );
        let title_only = window_info(
            1,
            "Alpha renamed",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );
        let path_changed = window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("D:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );
        let app_changed = window_info(
            1,
            "Alpha",
            "app:beta",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );

        for (fresh, expected) in [
            (title_only, Vec::<String>::new()),
            (path_changed, vec!["app:alpha".to_owned()]),
            (
                app_changed,
                vec!["app:alpha".to_owned(), "app:beta".to_owned()],
            ),
        ] {
            let mut windows = vec![new_managed_window(current.clone())];
            let outcome = reconcile_managed_windows(&mut windows, vec![fresh]);
            assert_eq!(outcome.icon_invalidations, expected);
        }
    }
}
