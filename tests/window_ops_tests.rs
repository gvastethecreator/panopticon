//! Integration tests for pure window ordering and summary helpers.

use std::ffi::c_void;

use panopticon::settings::{AppSettings, WindowGrouping};
use panopticon::window_enum::WindowInfo;
use panopticon::window_ops::{
    active_filter_summary, apply_pinned_positions, collect_available_apps,
    collect_available_monitors, sort_windows_for_grouping, truncate_title,
};
use windows::Win32::Foundation::HWND;

fn window(
    id: usize,
    title: &str,
    app_id: &str,
    process_name: &str,
    class_name: &str,
    monitor_name: &str,
) -> WindowInfo {
    WindowInfo {
        hwnd: HWND(id as *mut c_void),
        title: title.to_owned(),
        app_id: app_id.to_owned(),
        process_name: process_name.to_owned(),
        process_path: None,
        class_name: class_name.to_owned(),
        monitor_name: monitor_name.to_owned(),
    }
}

#[test]
fn collect_available_monitors_returns_unique_sorted_names() {
    let windows = vec![
        window(1, "Alpha", "app:a", "Arc", "Chrome_WidgetWin_1", "DISPLAY2"),
        window(2, "Beta", "app:b", "Code", "Chrome_WidgetWin_1", "DISPLAY1"),
        window(
            3,
            "Gamma",
            "app:c",
            "Terminal",
            "CASCADIA_HOSTING_WINDOW_CLASS",
            "DISPLAY2",
        ),
    ];

    let monitors = collect_available_monitors(&windows);

    assert_eq!(monitors, vec!["DISPLAY1".to_owned(), "DISPLAY2".to_owned()]);
}

#[test]
fn collect_available_apps_returns_unique_entries_sorted_by_label_then_id() {
    let windows = vec![
        window(
            1,
            "Alpha",
            "app:zeta",
            "Arc",
            "Chrome_WidgetWin_1",
            "DISPLAY1",
        ),
        window(
            2,
            "Beta",
            "app:code",
            "Code",
            "Chrome_WidgetWin_1",
            "DISPLAY1",
        ),
        window(
            3,
            "Gamma",
            "app:arc-secondary",
            "Arc",
            "Chrome_WidgetWin_1",
            "DISPLAY2",
        ),
    ];

    let apps = collect_available_apps(&windows);
    let labels: Vec<_> = apps
        .iter()
        .map(|entry| format!("{}:{}", entry.label, entry.app_id))
        .collect();

    assert_eq!(
        labels,
        vec![
            "Arc:app:arc-secondary".to_owned(),
            "Arc:app:zeta".to_owned(),
            "Code:app:code".to_owned(),
        ]
    );
}

#[test]
fn sort_windows_for_grouping_orders_by_application_then_title() {
    let mut settings = AppSettings::default();
    settings.group_windows_by = WindowGrouping::Application;

    let mut windows = vec![
        window(
            1,
            "zeta.txt",
            "app:code",
            "Code",
            "Chrome_WidgetWin_1",
            "DISPLAY1",
        ),
        window(
            2,
            "alpha.com",
            "app:arc",
            "Arc",
            "Chrome_WidgetWin_1",
            "DISPLAY1",
        ),
        window(
            3,
            "beta.txt",
            "app:code",
            "Code",
            "Chrome_WidgetWin_1",
            "DISPLAY2",
        ),
    ];

    sort_windows_for_grouping(&mut windows, &settings);

    let ordered: Vec<_> = windows.iter().map(|window| window.title.as_str()).collect();
    assert_eq!(ordered, vec!["alpha.com", "beta.txt", "zeta.txt"]);
}

#[test]
fn apply_pinned_positions_reserves_requested_slot_and_preserves_remaining_order() {
    let mut settings = AppSettings::default();
    let _ = settings.toggle_app_pinned_position("app:code", "Code", 0);
    let _ = settings.toggle_app_pinned_position("app:terminal", "Terminal", 2);

    let mut windows = vec![
        window(1, "Arc", "app:arc", "Arc", "Chrome_WidgetWin_1", "DISPLAY1"),
        window(
            2,
            "Code",
            "app:code",
            "Code",
            "Chrome_WidgetWin_1",
            "DISPLAY1",
        ),
        window(
            3,
            "Terminal",
            "app:terminal",
            "WindowsTerminal",
            "CASCADIA_HOSTING_WINDOW_CLASS",
            "DISPLAY1",
        ),
        window(
            4,
            "Notes",
            "app:notes",
            "Notes",
            "ApplicationFrameWindow",
            "DISPLAY1",
        ),
    ];

    apply_pinned_positions(&mut windows, &settings);

    let ordered: Vec<_> = windows
        .iter()
        .map(|window| window.app_id.as_str())
        .collect();
    assert_eq!(
        ordered,
        vec!["app:code", "app:arc", "app:terminal", "app:notes"]
    );
}

#[test]
fn truncate_title_adds_ellipsis_only_when_needed() {
    let original = "12345678901234567890123456789012345678901234567890EXTRA";

    let truncated = truncate_title(original);

    assert!(truncated.ends_with("..."));
    assert!(truncated.len() < original.len());
    assert_eq!(truncate_title("short title"), "short title");
}

#[test]
fn active_filter_summary_combines_monitor_tag_and_grouping_labels() {
    let mut settings = AppSettings::default();
    settings.set_monitor_filter(Some("DISPLAY2"));
    let _ = settings.toggle_app_tag("app:arc", "Arc", "work");
    settings.set_tag_filter(Some("work"));
    settings.group_windows_by = WindowGrouping::Application;

    let summary = active_filter_summary(&settings);

    assert_eq!(
        summary,
        Some("monitor:DISPLAY2 · tag:work · grouped by:Application".to_owned())
    );
}
