//! Window enumeration refresh and synchronization with managed state.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{apply_pinned_positions, sort_windows_for_grouping};
use windows::Win32::Foundation::{HWND, RECT, SIZE};
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use super::dwm::{ensure_thumbnail, query_source_size};
use crate::{AppState, ManagedWindow};

pub(crate) fn refresh_windows(state: &Rc<RefCell<AppState>>) -> bool {
    let mut state = state.borrow_mut();
    let host_hwnd = state.hwnd;
    if host_hwnd.0.is_null() {
        return false;
    }
    let host_visible = unsafe { IsWindowVisible(host_hwnd).as_bool() };

    let mut discovered = collect_discovered_windows(&mut state, host_hwnd);

    sort_windows_for_grouping(&mut discovered, &state.settings);
    apply_pinned_positions(&mut discovered, &state.settings);

    let discovered_map: HashMap<isize, &WindowInfo> = discovered
        .iter()
        .map(|window| (window.hwnd.0 as isize, window))
        .collect();
    let discovered_hwnds: HashSet<isize> = discovered_map.keys().copied().collect();
    let discovered_order: HashMap<isize, usize> = discovered
        .iter()
        .enumerate()
        .map(|(index, window)| (window.hwnd.0 as isize, index))
        .collect();

    let previous_len = state.windows.len();
    state
        .windows
        .retain(|managed_window| discovered_hwnds.contains(&(managed_window.info.hwnd.0 as isize)));
    let mut changed = state.windows.len() != previous_len;

    changed |=
        update_existing_windows(&mut state.windows, &discovered_map, host_hwnd, host_visible);

    let existing: HashSet<isize> = state
        .windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();

    changed |= append_new_windows(
        &mut state.windows,
        discovered,
        &existing,
        host_hwnd,
        host_visible,
    );

    let order_before: Vec<isize> = state
        .windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();
    state.windows.sort_by_key(|managed_window| {
        discovered_order
            .get(&(managed_window.info.hwnd.0 as isize))
            .copied()
            .unwrap_or(usize::MAX)
    });
    let order_after: Vec<isize> = state
        .windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();
    if order_before != order_after {
        changed = true;
    }

    changed
}

fn collect_discovered_windows(state: &mut AppState, host_hwnd: HWND) -> Vec<WindowInfo> {
    let discovered_all: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != host_hwnd)
        .collect();

    for window in &discovered_all {
        state
            .settings
            .refresh_app_label(&window.app_id, &window.app_label());
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

fn append_new_windows(
    windows: &mut Vec<ManagedWindow>,
    discovered: Vec<WindowInfo>,
    existing: &HashSet<isize>,
    host_hwnd: HWND,
    host_visible: bool,
) -> bool {
    let mut changed = false;
    for info in discovered {
        if existing.contains(&(info.hwnd.0 as isize)) {
            continue;
        }
        let mut managed_window = ManagedWindow {
            info,
            thumbnail: None,
            target_rect: RECT::default(),
            display_rect: RECT::default(),
            animation_from_rect: RECT::default(),
            source_size: SIZE { cx: 800, cy: 600 },
            last_thumb_update: None,
            last_thumb_dest: None,
            last_thumb_visible: false,
            cached_icon: None,
        };
        if host_visible {
            let _ = ensure_thumbnail(host_hwnd, &mut managed_window);
        }
        windows.push(managed_window);
        changed = true;
    }
    changed
}

fn update_existing_windows(
    windows: &mut [ManagedWindow],
    discovered_map: &HashMap<isize, &WindowInfo>,
    host_hwnd: HWND,
    host_visible: bool,
) -> bool {
    let mut changed = false;
    for managed_window in windows.iter_mut() {
        if let Some(fresh) = discovered_map.get(&(managed_window.info.hwnd.0 as isize)) {
            let metadata_changed = window_metadata_changed(&managed_window.info, fresh);
            if metadata_changed {
                let icon_changed = should_reset_cached_icon(&managed_window.info, fresh);
                managed_window.info = (*fresh).clone();
                if icon_changed {
                    managed_window.cached_icon = None;
                }
                changed = true;
            }
            if host_visible {
                if ensure_thumbnail(host_hwnd, managed_window) {
                    changed = true;
                }
                if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
                    let fresh_size = query_source_size(thumbnail.handle());
                    if fresh_size.cx != managed_window.source_size.cx
                        || fresh_size.cy != managed_window.source_size.cy
                    {
                        managed_window.source_size = fresh_size;
                        changed = true;
                    }
                }
            }
        }
    }
    changed
}

fn window_metadata_changed(current: &WindowInfo, fresh: &WindowInfo) -> bool {
    fresh.title != current.title
        || fresh.app_id != current.app_id
        || fresh.process_name != current.process_name
        || fresh.process_path != current.process_path
        || fresh.class_name != current.class_name
        || fresh.monitor_name != current.monitor_name
}

fn should_reset_cached_icon(current: &WindowInfo, fresh: &WindowInfo) -> bool {
    fresh.app_id != current.app_id || fresh.process_path != current.process_path
}

#[cfg(test)]
mod tests {
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

        assert!(window_metadata_changed(&current, &fresh));
        assert!(!window_metadata_changed(&current, &current));
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

        assert!(!should_reset_cached_icon(&current, &title_only));
        assert!(should_reset_cached_icon(&current, &path_changed));
        assert!(should_reset_cached_icon(&current, &app_changed));
    }
}
