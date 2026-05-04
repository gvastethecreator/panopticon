use panopticon::i18n;
use panopticon::settings::{DockEdge, ToolbarPosition, WindowGrouping};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, PostMessageW, SetForegroundWindow,
    TrackPopupMenu, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_TOPALIGN, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP,
};

use crate::app::menu_utils::{checked_flag, disabled_flag, encode_wide};

use super::*;

/// Convert a tray callback into a higher-level action.
#[must_use]
pub fn handle_tray_message(
    hwnd: HWND,
    lparam: LPARAM,
    state: &TrayMenuState,
) -> Option<TrayAction> {
    match lparam.0 as u32 {
        WM_LBUTTONUP => Some(TrayAction::Toggle),
        WM_RBUTTONUP => show_application_context_menu(hwnd, state),
        _ => None,
    }
}

#[must_use]
pub fn show_application_context_menu(hwnd: HWND, state: &TrayMenuState) -> Option<TrayAction> {
    show_application_context_menu_at(hwnd, state, None, false)
}

#[allow(clippy::too_many_lines)]
pub fn show_application_context_menu_at(
    hwnd: HWND,
    state: &TrayMenuState,
    anchor: Option<POINT>,
    prefer_below_anchor: bool,
) -> Option<TrayAction> {
    // SAFETY: menu is created, populated, and destroyed on the same thread.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let toggle_label = if state.window_visible {
            i18n::t("tray.hide")
        } else {
            i18n::t("tray.show")
        };

        let visibility_title = encode_wide(i18n::t("tray.visibility"));
        let layout_title = encode_wide(i18n::t("tray.layout"));
        let display_title = encode_wide(i18n::t("tray.display"));
        let behaviour_title = encode_wide(i18n::t("tray.behaviour"));
        let filters_title = encode_wide(i18n::t("tray.filters"));
        let toggle = encode_wide(toggle_label);
        let refresh = encode_wide(i18n::t("tray.refresh"));
        let open_settings = encode_wide(i18n::t("tray.open_settings"));
        let open_about = encode_wide(i18n::t("tray.open_about"));
        let next_layout = encode_wide(i18n::t("tray.next_layout"));
        let lock_layout = encode_wide(i18n::t("tray.lock_layout"));
        let lock_cell_resize = encode_wide(i18n::t("tray.lock_resize"));
        let dock_title = encode_wide(i18n::t("tray.dock_position"));
        let grouping_title = encode_wide(i18n::t("tray.group_by"));
        let minimize_to_tray = encode_wide(i18n::t("tray.minimize_to_tray"));
        let close_to_tray = encode_wide(i18n::t("tray.close_to_tray"));
        let refresh_interval = encode_wide(&i18n::t_fmt(
            "tray.cycle_refresh",
            &format_refresh_interval_label(state.refresh_interval_ms),
        ));
        let animations = encode_wide(i18n::t("tray.animate"));
        let default_aspect_ratio = encode_wide(i18n::t("tray.default_aspect"));
        let default_hide_on_select = encode_wide(i18n::t("tray.default_hide"));
        let always_on_top = encode_wide(i18n::t("tray.always_on_top"));
        let show_toolbar = encode_wide(i18n::t("tray.show_toolbar"));
        let toolbar_position_title = encode_wide(i18n::t("settings.option.toolbar_position.title"));
        let toolbar_top = encode_wide(i18n::t("settings.toolbar_position.top"));
        let toolbar_bottom = encode_wide(i18n::t("settings.toolbar_position.bottom"));
        let show_window_info = encode_wide(i18n::t("tray.show_info"));
        let show_app_icons = encode_wide(i18n::t("tray.show_icons"));
        let start_in_tray = encode_wide(i18n::t("tray.start_tray"));
        let dock_none = encode_wide(i18n::t("dock.none"));
        let dock_left = encode_wide(i18n::t("dock.left"));
        let dock_right = encode_wide(i18n::t("dock.right"));
        let dock_top = encode_wide(i18n::t("dock.top"));
        let dock_bottom = encode_wide(i18n::t("dock.bottom"));
        let group_none = encode_wide(i18n::t("group.none"));
        let group_application = encode_wide(i18n::t("group.application"));
        let group_monitor = encode_wide(i18n::t("group.monitor"));
        let group_window_title = encode_wide(i18n::t("group.title"));
        let group_class_name = encode_wide(i18n::t("group.class"));
        let restore_hidden_title = encode_wide(i18n::t("tray.restore_hidden"));
        let restore_all_hidden = encode_wide(i18n::t("tray.restore_all"));
        let workspaces_title = encode_wide(i18n::t("tray.workspaces"));
        let default_workspace = encode_wide(i18n::t("tray.workspace_default"));
        let monitor_filter_title = encode_wide(i18n::t("tray.filter_monitor"));
        let monitor_all = encode_wide(i18n::t("tray.all_monitors"));
        let tag_filter_title = encode_wide(i18n::t("tray.filter_tag"));
        let tag_filter_all = encode_wide(i18n::t("tray.all_tags"));
        let app_filter_title = encode_wide(i18n::t("tray.filter_app"));
        let app_filter_all = encode_wide(i18n::t("tray.all_apps"));
        let exit = encode_wide(i18n::t("tray.exit"));

        let mut hidden_labels: Vec<Vec<u16>> = Vec::with_capacity(state.hidden_apps.len());
        let mut monitor_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_monitors.len());
        let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_tags.len());
        let mut app_labels: Vec<Vec<u16>> = Vec::with_capacity(state.available_apps.len());
        let mut workspace_labels: Vec<Vec<u16>> =
            Vec::with_capacity(state.available_workspaces.len());
        let mut restore_actions: Vec<(u16, String)> = Vec::with_capacity(state.hidden_apps.len());
        let mut monitor_actions: Vec<(u16, String)> =
            Vec::with_capacity(state.available_monitors.len());
        let mut tag_actions: Vec<(u16, String)> = Vec::with_capacity(state.available_tags.len());
        let mut app_actions: Vec<(u16, String)> = Vec::with_capacity(state.available_apps.len());
        let mut workspace_actions: Vec<(u16, String)> =
            Vec::with_capacity(state.available_workspaces.len());

        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(visibility_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_TOGGLE as usize,
            PCWSTR(toggle.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_REFRESH as usize,
            PCWSTR(refresh.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_OPEN_SETTINGS as usize,
            PCWSTR(open_settings.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_OPEN_ABOUT as usize,
            PCWSTR(open_about.as_ptr()),
        );

        if !state.available_workspaces.is_empty() {
            let workspaces_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                workspaces_menu,
                MF_STRING | checked_flag(state.current_workspace.is_none()),
                CMD_TRAY_LOAD_DEFAULT_WORKSPACE as usize,
                PCWSTR(default_workspace.as_ptr()),
            );

            for (index, workspace) in state.available_workspaces.iter().enumerate() {
                if workspace.eq_ignore_ascii_case("default") {
                    continue;
                }
                let Some(command_id) = CMD_TRAY_LOAD_WORKSPACE_BASE.checked_add(index as u16)
                else {
                    break;
                };
                workspace_labels.push(encode_wide(workspace));
                if let Some(label) = workspace_labels.last() {
                    let _ = AppendMenuW(
                        workspaces_menu,
                        MF_STRING
                            | checked_flag(
                                state.current_workspace.as_deref() == Some(workspace.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                workspace_actions.push((command_id, workspace.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                workspaces_menu.0 as usize,
                PCWSTR(workspaces_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(layout_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_NEXT_LAYOUT as usize,
            PCWSTR(next_layout.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.locked_layout),
            CMD_TRAY_TOGGLE_LOCKED_LAYOUT as usize,
            PCWSTR(lock_layout.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.lock_cell_resize),
            CMD_TRAY_TOGGLE_LOCK_CELL_RESIZE as usize,
            PCWSTR(lock_cell_resize.as_ptr()),
        );

        {
            let dock_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge.is_none()),
                CMD_TRAY_DOCK_NONE as usize,
                PCWSTR(dock_none.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Left)),
                CMD_TRAY_DOCK_LEFT as usize,
                PCWSTR(dock_left.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Right)),
                CMD_TRAY_DOCK_RIGHT as usize,
                PCWSTR(dock_right.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Top)),
                CMD_TRAY_DOCK_TOP as usize,
                PCWSTR(dock_top.as_ptr()),
            );
            let _ = AppendMenuW(
                dock_menu,
                MF_STRING | checked_flag(state.dock_edge == Some(DockEdge::Bottom)),
                CMD_TRAY_DOCK_BOTTOM as usize,
                PCWSTR(dock_bottom.as_ptr()),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                dock_menu.0 as usize,
                PCWSTR(dock_title.as_ptr()),
            );
        }

        {
            let grouping_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::None),
                CMD_TRAY_GROUP_NONE as usize,
                PCWSTR(group_none.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::Application),
                CMD_TRAY_GROUP_APPLICATION as usize,
                PCWSTR(group_application.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::Monitor),
                CMD_TRAY_GROUP_MONITOR as usize,
                PCWSTR(group_monitor.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::WindowTitle),
                CMD_TRAY_GROUP_WINDOW_TITLE as usize,
                PCWSTR(group_window_title.as_ptr()),
            );
            let _ = AppendMenuW(
                grouping_menu,
                MF_STRING | checked_flag(state.group_windows_by == WindowGrouping::ClassName),
                CMD_TRAY_GROUP_CLASS_NAME as usize,
                PCWSTR(group_class_name.as_ptr()),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                grouping_menu.0 as usize,
                PCWSTR(grouping_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(display_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_toolbar),
            CMD_TRAY_TOGGLE_TOOLBAR as usize,
            PCWSTR(show_toolbar.as_ptr()),
        );

        {
            let toolbar_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                toolbar_menu,
                MF_STRING | checked_flag(state.toolbar_position == ToolbarPosition::Top),
                CMD_TRAY_TOOLBAR_TOP as usize,
                PCWSTR(toolbar_top.as_ptr()),
            );
            let _ = AppendMenuW(
                toolbar_menu,
                MF_STRING | checked_flag(state.toolbar_position == ToolbarPosition::Bottom),
                CMD_TRAY_TOOLBAR_BOTTOM as usize,
                PCWSTR(toolbar_bottom.as_ptr()),
            );
            let _ = AppendMenuW(
                menu,
                MF_POPUP | disabled_flag(!state.show_toolbar),
                toolbar_menu.0 as usize,
                PCWSTR(toolbar_position_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_window_info),
            CMD_TRAY_TOGGLE_WINDOW_INFO as usize,
            PCWSTR(show_window_info.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.show_app_icons),
            CMD_TRAY_TOGGLE_APP_ICONS as usize,
            PCWSTR(show_app_icons.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.always_on_top),
            CMD_TRAY_TOGGLE_ALWAYS_ON_TOP as usize,
            PCWSTR(always_on_top.as_ptr()),
        );

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_GRAYED,
            0,
            PCWSTR(behaviour_title.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.minimize_to_tray),
            CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY as usize,
            PCWSTR(minimize_to_tray.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.close_to_tray),
            CMD_TRAY_TOGGLE_CLOSE_TO_TRAY as usize,
            PCWSTR(close_to_tray.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_CYCLE_REFRESH as usize,
            PCWSTR(refresh_interval.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.animate_transitions),
            CMD_TRAY_TOGGLE_ANIMATIONS as usize,
            PCWSTR(animations.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.preserve_aspect_ratio),
            CMD_TRAY_TOGGLE_DEFAULT_ASPECT_RATIO as usize,
            PCWSTR(default_aspect_ratio.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.hide_on_select) | disabled_flag(state.is_docked),
            CMD_TRAY_TOGGLE_DEFAULT_HIDE_ON_SELECT as usize,
            PCWSTR(default_hide_on_select.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_flag(state.start_in_tray),
            CMD_TRAY_TOGGLE_START_IN_TRAY as usize,
            PCWSTR(start_in_tray.as_ptr()),
        );

        let has_filter_section = !state.available_monitors.is_empty()
            || !state.available_tags.is_empty()
            || !state.available_apps.is_empty();
        if has_filter_section {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                PCWSTR(filters_title.as_ptr()),
            );
        }

        if !state.available_monitors.is_empty() {
            let monitor_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                monitor_menu,
                MF_STRING | checked_flag(state.active_monitor_filter.is_none()),
                CMD_TRAY_MONITOR_ALL as usize,
                PCWSTR(monitor_all.as_ptr()),
            );

            for (index, monitor) in state.available_monitors.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_MONITOR_BASE.checked_add(index as u16) else {
                    break;
                };

                monitor_labels.push(encode_wide(monitor));
                if let Some(label) = monitor_labels.last() {
                    let _ = AppendMenuW(
                        monitor_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_monitor_filter.as_deref() == Some(monitor.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                monitor_actions.push((command_id, monitor.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                monitor_menu.0 as usize,
                PCWSTR(monitor_filter_title.as_ptr()),
            );
        }

        if !state.available_tags.is_empty() {
            let tag_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                tag_menu,
                MF_STRING | checked_flag(state.active_tag_filter.is_none()),
                CMD_TRAY_TAG_FILTER_ALL as usize,
                PCWSTR(tag_filter_all.as_ptr()),
            );

            for (index, tag) in state.available_tags.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_TAG_FILTER_BASE.checked_add(index as u16) else {
                    break;
                };

                tag_labels.push(encode_wide(tag));
                if let Some(label) = tag_labels.last() {
                    let _ = AppendMenuW(
                        tag_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_tag_filter.as_deref() == Some(tag.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                tag_actions.push((command_id, tag.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                tag_menu.0 as usize,
                PCWSTR(tag_filter_title.as_ptr()),
            );
        }

        if !state.available_apps.is_empty() {
            let app_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                app_menu,
                MF_STRING | checked_flag(state.active_app_filter.is_none()),
                CMD_TRAY_APP_FILTER_ALL as usize,
                PCWSTR(app_filter_all.as_ptr()),
            );

            for (index, app) in state.available_apps.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_APP_FILTER_BASE.checked_add(index as u16) else {
                    break;
                };

                app_labels.push(encode_wide(&app.label));
                if let Some(label) = app_labels.last() {
                    let _ = AppendMenuW(
                        app_menu,
                        MF_STRING
                            | checked_flag(
                                state.active_app_filter.as_deref() == Some(app.app_id.as_str()),
                            ),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                app_actions.push((command_id, app.app_id.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                app_menu.0 as usize,
                PCWSTR(app_filter_title.as_ptr()),
            );
        }

        if !state.hidden_apps.is_empty() {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let hidden_menu = CreatePopupMenu().ok()?;
            let _ = AppendMenuW(
                hidden_menu,
                MF_STRING,
                CMD_TRAY_RESTORE_ALL_HIDDEN as usize,
                PCWSTR(restore_all_hidden.as_ptr()),
            );
            let _ = AppendMenuW(hidden_menu, MF_SEPARATOR, 0, PCWSTR::null());

            for (index, hidden_app) in state.hidden_apps.iter().enumerate() {
                let Some(command_id) = CMD_TRAY_RESTORE_HIDDEN_BASE.checked_add(index as u16)
                else {
                    break;
                };
                hidden_labels.push(encode_wide(&hidden_app.label));
                if let Some(label) = hidden_labels.last() {
                    let _ = AppendMenuW(
                        hidden_menu,
                        MF_STRING,
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                restore_actions.push((command_id, hidden_app.app_id.clone()));
            }

            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                hidden_menu.0 as usize,
                PCWSTR(restore_hidden_title.as_ptr()),
            );
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_TRAY_EXIT as usize,
            PCWSTR(exit.as_ptr()),
        );

        let mut cursor = anchor.unwrap_or_default();
        if anchor.is_none() {
            let _ = GetCursorPos(&raw mut cursor);
        }
        let _ = SetForegroundWindow(hwnd);

        let vertical_alignment = if anchor.is_some() && prefer_below_anchor {
            TPM_TOPALIGN
        } else {
            TPM_BOTTOMALIGN
        };

        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | vertical_alignment,
            cursor.x,
            cursor.y,
            Some(0),
            hwnd,
            None,
        );

        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));

        let _ = DestroyMenu(menu);

        match command.0 as u16 {
            CMD_TRAY_TOGGLE => Some(TrayAction::Toggle),
            CMD_TRAY_REFRESH => Some(TrayAction::Refresh),
            CMD_TRAY_NEXT_LAYOUT => Some(TrayAction::NextLayout),
            CMD_TRAY_TOGGLE_MINIMIZE_TO_TRAY => Some(TrayAction::ToggleMinimizeToTray),
            CMD_TRAY_TOGGLE_CLOSE_TO_TRAY => Some(TrayAction::ToggleCloseToTray),
            CMD_TRAY_CYCLE_REFRESH => Some(TrayAction::CycleRefreshInterval),
            CMD_TRAY_TOGGLE_ANIMATIONS => Some(TrayAction::ToggleAnimateTransitions),
            CMD_TRAY_TOGGLE_DEFAULT_ASPECT_RATIO => Some(TrayAction::ToggleDefaultAspectRatio),
            CMD_TRAY_TOGGLE_DEFAULT_HIDE_ON_SELECT => Some(TrayAction::ToggleDefaultHideOnSelect),
            CMD_TRAY_TOGGLE_ALWAYS_ON_TOP => Some(TrayAction::ToggleAlwaysOnTop),
            CMD_TRAY_TOGGLE_TOOLBAR => Some(TrayAction::ToggleToolbar),
            CMD_TRAY_TOOLBAR_TOP => Some(TrayAction::SetToolbarPosition(ToolbarPosition::Top)),
            CMD_TRAY_TOOLBAR_BOTTOM => {
                Some(TrayAction::SetToolbarPosition(ToolbarPosition::Bottom))
            }
            CMD_TRAY_TOGGLE_WINDOW_INFO => Some(TrayAction::ToggleWindowInfo),
            CMD_TRAY_TOGGLE_APP_ICONS => Some(TrayAction::ToggleAppIcons),
            CMD_TRAY_TOGGLE_START_IN_TRAY => Some(TrayAction::ToggleStartInTray),
            CMD_TRAY_TOGGLE_LOCKED_LAYOUT => Some(TrayAction::ToggleLockedLayout),
            CMD_TRAY_TOGGLE_LOCK_CELL_RESIZE => Some(TrayAction::ToggleLockCellResize),
            CMD_TRAY_OPEN_SETTINGS => Some(TrayAction::OpenSettingsWindow),
            CMD_TRAY_OPEN_ABOUT => Some(TrayAction::OpenAboutWindow),
            CMD_TRAY_LOAD_DEFAULT_WORKSPACE => Some(TrayAction::LoadWorkspace(None)),
            CMD_TRAY_DOCK_NONE => Some(TrayAction::SetDockEdge(None)),
            CMD_TRAY_DOCK_LEFT => Some(TrayAction::SetDockEdge(Some(DockEdge::Left))),
            CMD_TRAY_DOCK_RIGHT => Some(TrayAction::SetDockEdge(Some(DockEdge::Right))),
            CMD_TRAY_DOCK_TOP => Some(TrayAction::SetDockEdge(Some(DockEdge::Top))),
            CMD_TRAY_DOCK_BOTTOM => Some(TrayAction::SetDockEdge(Some(DockEdge::Bottom))),
            CMD_TRAY_GROUP_NONE => Some(TrayAction::SetWindowGrouping(WindowGrouping::None)),
            CMD_TRAY_GROUP_APPLICATION => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::Application))
            }
            CMD_TRAY_GROUP_MONITOR => Some(TrayAction::SetWindowGrouping(WindowGrouping::Monitor)),
            CMD_TRAY_GROUP_WINDOW_TITLE => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::WindowTitle))
            }
            CMD_TRAY_GROUP_CLASS_NAME => {
                Some(TrayAction::SetWindowGrouping(WindowGrouping::ClassName))
            }
            CMD_TRAY_MONITOR_ALL => Some(TrayAction::SetMonitorFilter(None)),
            CMD_TRAY_TAG_FILTER_ALL => Some(TrayAction::SetTagFilter(None)),
            CMD_TRAY_APP_FILTER_ALL => Some(TrayAction::SetAppFilter(None)),
            CMD_TRAY_RESTORE_ALL_HIDDEN => Some(TrayAction::RestoreAllHidden),
            CMD_TRAY_EXIT => Some(TrayAction::Exit),
            dynamic => monitor_actions
                .into_iter()
                .find_map(|(command_id, monitor)| {
                    (dynamic == command_id).then_some(TrayAction::SetMonitorFilter(Some(monitor)))
                })
                .or_else(|| {
                    tag_actions.into_iter().find_map(|(command_id, tag)| {
                        (dynamic == command_id).then_some(TrayAction::SetTagFilter(Some(tag)))
                    })
                })
                .or_else(|| {
                    app_actions.into_iter().find_map(|(command_id, app_id)| {
                        (dynamic == command_id).then_some(TrayAction::SetAppFilter(Some(app_id)))
                    })
                })
                .or_else(|| {
                    workspace_actions
                        .into_iter()
                        .find_map(|(command_id, workspace)| {
                            (dynamic == command_id)
                                .then_some(TrayAction::LoadWorkspace(Some(workspace)))
                        })
                })
                .or_else(|| {
                    restore_actions
                        .into_iter()
                        .find_map(|(command_id, app_id)| {
                            (dynamic == command_id).then_some(TrayAction::RestoreHidden(app_id))
                        })
                }),
        }
    }
}

fn format_refresh_interval_label(interval_ms: u32) -> String {
    if interval_ms.is_multiple_of(1_000) {
        format!("{}s", interval_ms / 1_000)
    } else {
        format!("{:.1}s", f64::from(interval_ms) / 1_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::format_refresh_interval_label;

    #[test]
    fn refresh_interval_labels_keep_whole_seconds_compact() {
        assert_eq!(format_refresh_interval_label(1_000), "1s");
        assert_eq!(format_refresh_interval_label(10_000), "10s");
    }

    #[test]
    fn refresh_interval_labels_keep_fractional_seconds_readable() {
        assert_eq!(format_refresh_interval_label(500), "0.5s");
        assert_eq!(format_refresh_interval_label(2_500), "2.5s");
    }
}
