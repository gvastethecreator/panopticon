//! Pure helpers for ordering, summarising, and presenting enumerated windows.
//!
//! These utilities are shared by the binary runtime and integration tests so
//! that window ordering and filter summaries can be validated without going
//! through the native UI event loop.

use std::collections::{BTreeSet, HashMap};

use crate::constants::{MAX_TITLE_CHARS, TITLE_TRUNCATE_AT};
use crate::settings::{AppSelectionEntry, AppSettings, WindowGrouping};
use crate::window_enum::WindowInfo;

/// Sort windows according to the currently selected grouping strategy.
pub fn sort_windows_for_grouping(windows: &mut [WindowInfo], settings: &AppSettings) {
    if settings.group_windows_by == WindowGrouping::None {
        return;
    }

    windows.sort_by_cached_key(|window| {
        (
            grouping_sort_key(window, settings.group_windows_by),
            normalize_sort_value(&window.app_label()),
            normalize_sort_value(&window.title),
            normalize_sort_value(&window.monitor_name),
            window.hwnd.0 as isize,
        )
    });
}

/// Reorder windows to honour any per-app pinned positions.
pub fn apply_pinned_positions(windows: &mut Vec<WindowInfo>, settings: &AppSettings) {
    if windows.len() < 2 {
        return;
    }

    let total = windows.len();
    let mut pinned = Vec::new();
    let mut remaining = Vec::new();

    for window in windows.drain(..) {
        if let Some(position) = settings.pinned_position_for(&window.app_id) {
            pinned.push((position, window));
        } else {
            remaining.push(window);
        }
    }

    if pinned.is_empty() {
        *windows = remaining;
        return;
    }

    pinned.sort_by_key(|(position, window)| {
        (
            *position,
            normalize_sort_value(&window.app_label()),
            normalize_sort_value(&window.title),
            window.hwnd.0 as isize,
        )
    });

    let mut slots: Vec<Option<WindowInfo>> = std::iter::repeat_with(|| None).take(total).collect();

    for (desired_position, window) in pinned {
        let mut target = desired_position.min(total.saturating_sub(1));
        while target < total && slots[target].is_some() {
            target += 1;
        }

        if target >= total {
            target = total.saturating_sub(1);
            while slots[target].is_some() && target > 0 {
                target -= 1;
            }
        }

        if slots[target].is_none() {
            slots[target] = Some(window);
        } else {
            remaining.push(window);
        }
    }

    let mut reordered = Vec::with_capacity(total);
    let mut remaining_iter = remaining.into_iter();
    for slot in slots {
        if let Some(window) = slot {
            reordered.push(window);
        } else if let Some(window) = remaining_iter.next() {
            reordered.push(window);
        }
    }
    reordered.extend(remaining_iter);
    *windows = reordered;
}

/// Build a compact user-facing summary of active filters and grouping.
#[must_use]
pub fn active_filter_summary(settings: &AppSettings) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(monitor) = &settings.active_monitor_filter {
        parts.push(format!("monitor:{monitor}"));
    }
    if let Some(group_filter) = settings.active_group_filter_label() {
        parts.push(group_filter);
    }
    if let Some(grouping) = settings.grouping_label() {
        parts.push(grouping);
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// Collect unique monitor names in sorted order.
#[must_use]
pub fn collect_available_monitors(windows: &[WindowInfo]) -> Vec<String> {
    let set: BTreeSet<String> = windows
        .iter()
        .map(|window| window.monitor_name.clone())
        .collect();
    set.into_iter().collect()
}

/// Collect unique applications in sorted, user-friendly order.
#[must_use]
pub fn collect_available_apps(windows: &[WindowInfo]) -> Vec<AppSelectionEntry> {
    let mut map: HashMap<String, String> = HashMap::new();
    for window in windows {
        map.entry(window.app_id.clone())
            .or_insert_with(|| window.app_label());
    }
    let mut apps: Vec<AppSelectionEntry> = map
        .into_iter()
        .map(|(app_id, label)| AppSelectionEntry { app_id, label })
        .collect();
    apps.sort_by(|a, b| a.label.cmp(&b.label).then(a.app_id.cmp(&b.app_id)));
    apps
}

/// Truncate a long window title while preserving the workspace style.
#[must_use]
pub fn truncate_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() > MAX_TITLE_CHARS {
        let mut short: String = chars[..TITLE_TRUNCATE_AT].iter().collect();
        short.push_str("...");
        short
    } else {
        title.to_owned()
    }
}

fn grouping_sort_key(window: &WindowInfo, grouping: WindowGrouping) -> String {
    match grouping {
        WindowGrouping::None => String::new(),
        WindowGrouping::Application => normalize_sort_value(&window.app_label()),
        WindowGrouping::Monitor => normalize_sort_value(&window.monitor_name),
        WindowGrouping::WindowTitle => normalize_sort_value(&window.title),
        WindowGrouping::ClassName => normalize_sort_value(&window.class_name),
    }
}

fn normalize_sort_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
