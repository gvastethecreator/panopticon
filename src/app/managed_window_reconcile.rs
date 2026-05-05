//! Pure reconciliation between discovered `WindowInfo` values and `ManagedWindow` state.

use std::collections::{HashMap, HashSet};

use panopticon::window_enum::WindowInfo;
use windows::Win32::Foundation::{RECT, SIZE};

use crate::ManagedWindow;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ReconcileOutcome {
    pub(crate) changed: bool,
    pub(crate) removed: usize,
    pub(crate) added: usize,
    pub(crate) metadata_updated: usize,
    pub(crate) icon_invalidations: Vec<String>,
}

pub(crate) fn new_managed_window(info: WindowInfo) -> ManagedWindow {
    ManagedWindow {
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
    }
}

pub(crate) fn reconcile_managed_windows(
    windows: &mut Vec<ManagedWindow>,
    discovered: Vec<WindowInfo>,
) -> ReconcileOutcome {
    let discovered_map: HashMap<isize, WindowInfo> = discovered
        .iter()
        .map(|window| (window.hwnd.0 as isize, window.clone()))
        .collect();
    let discovered_hwnds: HashSet<isize> = discovered_map.keys().copied().collect();
    let discovered_order: HashMap<isize, usize> = discovered
        .iter()
        .enumerate()
        .map(|(index, window)| (window.hwnd.0 as isize, index))
        .collect();

    let previous_len = windows.len();
    windows
        .retain(|managed_window| discovered_hwnds.contains(&(managed_window.info.hwnd.0 as isize)));

    let mut outcome = ReconcileOutcome {
        removed: previous_len.saturating_sub(windows.len()),
        ..ReconcileOutcome::default()
    };

    for managed_window in windows.iter_mut() {
        let Some(fresh) = discovered_map.get(&(managed_window.info.hwnd.0 as isize)) else {
            continue;
        };
        if !window_metadata_changed(&managed_window.info, fresh) {
            continue;
        }

        if should_reset_cached_icon(&managed_window.info, fresh) {
            let previous_app_id = managed_window.info.app_id.clone();
            push_unique(&mut outcome.icon_invalidations, previous_app_id.clone());
            if fresh.app_id != previous_app_id {
                push_unique(&mut outcome.icon_invalidations, fresh.app_id.clone());
            }
            managed_window.cached_icon = None;
        }

        managed_window.info = fresh.clone();
        outcome.metadata_updated += 1;
    }

    let existing: HashSet<isize> = windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();

    for info in discovered {
        if existing.contains(&(info.hwnd.0 as isize)) {
            continue;
        }
        windows.push(new_managed_window(info));
        outcome.added += 1;
    }

    let order_before: Vec<isize> = windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();
    windows.sort_by_key(|managed_window| {
        discovered_order
            .get(&(managed_window.info.hwnd.0 as isize))
            .copied()
            .unwrap_or(usize::MAX)
    });
    let order_after: Vec<isize> = windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();

    outcome.changed = outcome.removed > 0
        || outcome.added > 0
        || outcome.metadata_updated > 0
        || order_before != order_after;
    outcome
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn window_metadata_changed(current: &WindowInfo, fresh: &WindowInfo) -> bool {
    fresh.app_id != current.app_id
        || fresh.class_name != current.class_name
        || fresh.monitor_name != current.monitor_name
        || fresh.process_name != current.process_name
        || fresh.process_path != current.process_path
        || fresh.title != current.title
}

fn should_reset_cached_icon(current: &WindowInfo, fresh: &WindowInfo) -> bool {
    fresh.app_id != current.app_id || fresh.process_path != current.process_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;

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
    fn new_managed_window_uses_clean_runtime_defaults() {
        let managed = new_managed_window(window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        ));

        assert!(managed.thumbnail.is_none());
        assert_eq!(managed.source_size.cx, 800);
        assert_eq!(managed.source_size.cy, 600);
        assert_eq!(managed.target_rect, RECT::default());
        assert_eq!(managed.display_rect, RECT::default());
        assert!(managed.last_thumb_update.is_none());
        assert!(managed.last_thumb_dest.is_none());
        assert!(!managed.last_thumb_visible);
        assert!(managed.cached_icon.is_none());
    }

    #[test]
    fn reconcile_adds_removes_updates_and_preserves_order() {
        let mut windows = vec![
            new_managed_window(window_info(
                1,
                "Alpha",
                "app:alpha",
                "Alpha",
                Some("C:/Alpha.exe"),
                "AlphaClass",
                "DISPLAY1",
            )),
            new_managed_window(window_info(
                2,
                "Beta",
                "app:beta",
                "Beta",
                Some("C:/Beta.exe"),
                "BetaClass",
                "DISPLAY1",
            )),
        ];

        let outcome = reconcile_managed_windows(
            &mut windows,
            vec![
                window_info(
                    2,
                    "Beta renamed",
                    "app:beta",
                    "Beta",
                    Some("C:/Beta.exe"),
                    "BetaClass",
                    "DISPLAY2",
                ),
                window_info(
                    3,
                    "Gamma",
                    "app:gamma",
                    "Gamma",
                    Some("C:/Gamma.exe"),
                    "GammaClass",
                    "DISPLAY1",
                ),
            ],
        );

        assert!(outcome.changed);
        assert_eq!(outcome.removed, 1);
        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.metadata_updated, 1);
        assert!(outcome.icon_invalidations.is_empty());
        assert_eq!(windows[0].info.hwnd.0 as isize, 2);
        assert_eq!(windows[0].info.title, "Beta renamed");
        assert_eq!(windows[1].info.hwnd.0 as isize, 3);
    }

    #[test]
    fn reconcile_invalidates_icons_only_for_app_or_path_changes() {
        let mut windows = vec![new_managed_window(window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        ))];

        let outcome = reconcile_managed_windows(
            &mut windows,
            vec![window_info(
                1,
                "Alpha",
                "app:alpha-renamed",
                "Alpha",
                Some("D:/Alpha.exe"),
                "AlphaClass",
                "DISPLAY1",
            )],
        );

        assert_eq!(
            outcome.icon_invalidations,
            vec!["app:alpha".to_owned(), "app:alpha-renamed".to_owned()]
        );
        assert!(windows[0].cached_icon.is_none());
    }

    #[test]
    fn reconcile_reports_unchanged_for_equivalent_discovery() {
        let info = window_info(
            1,
            "Alpha",
            "app:alpha",
            "Alpha",
            Some("C:/Alpha.exe"),
            "AlphaClass",
            "DISPLAY1",
        );
        let mut windows = vec![new_managed_window(info.clone())];

        let outcome = reconcile_managed_windows(&mut windows, vec![info]);

        assert!(!outcome.changed);
        assert_eq!(outcome, ReconcileOutcome::default());
    }
}
