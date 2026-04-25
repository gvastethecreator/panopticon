//! Command palette window and command execution registry.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, SharedString, VecModel};

use super::dock::apply_topmost_mode;
use super::layout_actions;
use super::secondary_windows;
use super::tray_actions;
use crate::{
    refresh_ui, refresh_windows, update_settings, AppState, CommandPaletteWindow, MainWindow,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandId {
    CycleLayout,
    SetLayout(LayoutType),
    ResetLayoutRatios,
    CycleTheme,
    RefreshNow,
    RestoreAllHiddenApps,
    OpenSettings,
    OpenSettingsBehaviorPage,
    OpenSettingsFiltersPage,
    OpenSettingsWorkspacesPage,
    OpenSettingsShortcutsPage,
    OpenSettingsAdvancedPage,
    OpenAbout,
    OpenMenu,
    ClearAllFilters,
    ClearMonitorFilter,
    ClearTagFilter,
    ClearAppFilter,
    LoadWorkspace(Option<String>),
    OpenWorkspaceInNewInstance(Option<String>),
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    ToggleAlwaysOnTop,
    Exit,
}

#[derive(Debug, Clone)]
struct CommandEntry {
    id: CommandId,
    title: String,
    keywords: String,
}

fn command_entries() -> Vec<CommandEntry> {
    let mut entries = vec![
        CommandEntry {
            id: CommandId::CycleLayout,
            title: "Layout: Cycle".to_owned(),
            keywords: "layout cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Grid),
            title: "Layout: Grid".to_owned(),
            keywords: "layout grid".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Mosaic),
            title: "Layout: Mosaic".to_owned(),
            keywords: "layout mosaic".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Bento),
            title: "Layout: Bento".to_owned(),
            keywords: "layout bento".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Fibonacci),
            title: "Layout: Fibonacci".to_owned(),
            keywords: "layout fibonacci".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Columns),
            title: "Layout: Columns".to_owned(),
            keywords: "layout columns".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Row),
            title: "Layout: Row".to_owned(),
            keywords: "layout row".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Column),
            title: "Layout: Column".to_owned(),
            keywords: "layout column".to_owned(),
        },
        CommandEntry {
            id: CommandId::ResetLayoutRatios,
            title: "Layout: Reset ratios".to_owned(),
            keywords: "layout reset ratios separators".to_owned(),
        },
        CommandEntry {
            id: CommandId::CycleTheme,
            title: "Theme: Cycle".to_owned(),
            keywords: "theme cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::RefreshNow,
            title: "Refresh: Run now".to_owned(),
            keywords: "refresh update windows now".to_owned(),
        },
        CommandEntry {
            id: CommandId::RestoreAllHiddenApps,
            title: "Windows: Restore all hidden apps".to_owned(),
            keywords: "windows hidden apps restore all".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettings,
            title: "Open Settings".to_owned(),
            keywords: "settings preferences config".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsBehaviorPage,
            title: "Settings: Behavior & Display".to_owned(),
            keywords: "settings behavior display".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsFiltersPage,
            title: "Settings: Filters".to_owned(),
            keywords: "settings filters monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsWorkspacesPage,
            title: "Settings: Workspaces".to_owned(),
            keywords: "settings workspaces profiles".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsShortcutsPage,
            title: "Settings: Shortcuts".to_owned(),
            keywords: "settings keyboard shortcuts".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsAdvancedPage,
            title: "Settings: Advanced".to_owned(),
            keywords: "settings advanced refresh dock".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenAbout,
            title: "Open About".to_owned(),
            keywords: "about version update".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenMenu,
            title: "Open App Menu".to_owned(),
            keywords: "menu context tray".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAllFilters,
            title: "Filters: Clear all".to_owned(),
            keywords: "filters clear all monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearMonitorFilter,
            title: "Filters: Clear monitor".to_owned(),
            keywords: "filters monitor clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearTagFilter,
            title: "Filters: Clear tag".to_owned(),
            keywords: "filters tag clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAppFilter,
            title: "Filters: Clear app".to_owned(),
            keywords: "filters app clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAnimations,
            title: "Toggle Animations".to_owned(),
            keywords: "animations toggle transitions".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleToolbar,
            title: "Toggle Status Bar".to_owned(),
            keywords: "toolbar status bar toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleWindowInfo,
            title: "Toggle Window Info".to_owned(),
            keywords: "window info labels overlay toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAlwaysOnTop,
            title: "Toggle Always On Top".to_owned(),
            keywords: "topmost always on top pin".to_owned(),
        },
        CommandEntry {
            id: CommandId::LoadWorkspace(None),
            title: "Workspace: Load default".to_owned(),
            keywords: "workspace load default".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(None),
            title: "Workspace: Open default in new instance".to_owned(),
            keywords: "workspace open default new instance".to_owned(),
        },
    ];

    let workspaces = AppSettings::list_workspaces_with_default().unwrap_or_else(|error| {
        tracing::warn!(%error, "failed to enumerate workspaces for command palette");
        vec!["default".to_owned()]
    });

    for workspace in workspaces {
        if workspace.eq_ignore_ascii_case("default") {
            continue;
        }
        entries.push(CommandEntry {
            id: CommandId::LoadWorkspace(Some(workspace.clone())),
            title: format!("Workspace: Load {workspace}"),
            keywords: format!("workspace load switch {workspace}"),
        });
        entries.push(CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(Some(workspace.clone())),
            title: format!("Workspace: Open {workspace} in new instance"),
            keywords: format!("workspace open launch new instance {workspace}"),
        });
    }

    entries.push(CommandEntry {
        id: CommandId::Exit,
        title: "Exit Panopticon".to_owned(),
        keywords: "quit exit close app".to_owned(),
    });

    entries
}

pub(crate) fn open_command_palette_window(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    let already_open = crate::COMMAND_PALETTE_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            let _ = existing.show();
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let window = match CommandPaletteWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create command palette window");
            return;
        }
    };

    crate::populate_tr_global(&window);

    let filtered = Rc::new(RefCell::new(Vec::<CommandId>::new()));
    rebuild_filtered_commands(&window, "", &filtered);

    window.on_apply_filter({
        let filtered = filtered.clone();
        move || {
            crate::COMMAND_PALETTE_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(window) = guard.as_ref() else {
                    return;
                };
                let query = window.get_command_search().to_string();
                rebuild_filtered_commands(window, &query, &filtered);
            });
        }
    });

    window.on_run_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        let filtered = filtered.clone();
        move || {
            crate::COMMAND_PALETTE_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(window) = guard.as_ref() else {
                    return;
                };

                let index = usize::try_from(window.get_command_index()).ok();
                let command_id = index.and_then(|index| filtered.borrow().get(index).cloned());
                let Some(command_id) = command_id else {
                    return;
                };

                execute_command(command_id, &state, &main_weak);
                close_command_palette_window();
            });
        }
    });

    window.on_key_pressed({
        let state = state.clone();
        let main_weak = main_weak.clone();
        let filtered = filtered.clone();
        move |key_text, _shift_pressed| {
            if key_text == "\u{001B}" {
                close_command_palette_window();
                return true;
            }

            if key_text == "\n" || key_text == "\r" {
                crate::COMMAND_PALETTE_WIN.with(|handle| {
                    let guard = handle.borrow();
                    let Some(window) = guard.as_ref() else {
                        return;
                    };

                    let index = usize::try_from(window.get_command_index()).ok();
                    let command_id = index.and_then(|index| filtered.borrow().get(index).cloned());
                    if let Some(command_id) = command_id {
                        execute_command(command_id, &state, &main_weak);
                        close_command_palette_window();
                    }
                });
                return true;
            }

            false
        }
    });

    window.on_close_requested(close_command_palette_window);
    window.window().on_close_requested(|| {
        close_command_palette_window();
        CloseRequestResponse::HideWindow
    });

    if let Err(error) = window.show() {
        tracing::error!(%error, "failed to show command palette window");
        return;
    }

    crate::COMMAND_PALETTE_WIN.with(|handle| *handle.borrow_mut() = Some(window));
}

fn rebuild_filtered_commands(
    window: &CommandPaletteWindow,
    query: &str,
    filtered: &RefCell<Vec<CommandId>>,
) {
    let normalized_query = query.trim().to_ascii_lowercase();
    let entries = command_entries();

    let matches: Vec<CommandEntry> = if normalized_query.is_empty() {
        entries
    } else {
        entries
            .into_iter()
            .filter(|entry| {
                entry.title.to_ascii_lowercase().contains(&normalized_query)
                    || entry
                        .keywords
                        .to_ascii_lowercase()
                        .contains(&normalized_query)
            })
            .collect()
    };

    *filtered.borrow_mut() = matches.iter().map(|entry| entry.id.clone()).collect();

    let labels = if matches.is_empty() {
        vec![SharedString::from("No commands found")]
    } else {
        matches
            .iter()
            .map(|entry| SharedString::from(entry.title.clone()))
            .collect()
    };

    window.set_command_options(ModelRc::new(VecModel::from(labels)));
    window.set_command_index(0);
}

fn execute_command(
    command_id: CommandId,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    match command_id {
        CommandId::CycleLayout => {
            layout_actions::cycle_layout(state);
            refresh_ui(state, main_weak);
        }
        CommandId::SetLayout(layout) => {
            layout_actions::set_layout(state, main_weak, layout);
        }
        CommandId::ResetLayoutRatios => {
            layout_actions::reset_layout_custom(state);
            refresh_ui(state, main_weak);
        }
        CommandId::CycleTheme => {
            let current_idx = {
                let state = state.borrow();
                panopticon::theme::theme_index(state.settings.theme_id.as_deref())
            };
            let total = panopticon::theme::theme_labels().len() as i32;
            let next_idx = (current_idx + 1).rem_euclid(total);
            let new_id = panopticon::theme::theme_id_by_index(next_idx);
            let next_background_hex =
                panopticon::theme::theme_base_background_hex(new_id.as_deref(), "181513");

            update_settings(state, |settings| {
                settings.theme_id = new_id;
                if settings.theme_id.is_some() {
                    settings
                        .background_color_hex
                        .clone_from(&next_background_hex);
                }
            });

            let state_ref = state.borrow();
            super::dock::apply_window_appearance(state_ref.hwnd, &state_ref.settings);
            drop(state_ref);

            refresh_ui(state, main_weak);
        }
        CommandId::RefreshNow => {
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::RestoreAllHiddenApps => {
            update_settings(state, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::OpenSettings => {
            secondary_windows::open_settings_window(state, main_weak);
        }
        CommandId::OpenSettingsBehaviorPage => {
            secondary_windows::open_settings_window_page(state, main_weak, 0);
        }
        CommandId::OpenSettingsFiltersPage => {
            secondary_windows::open_settings_window_page(state, main_weak, 1);
        }
        CommandId::OpenSettingsWorkspacesPage => {
            secondary_windows::open_settings_window_page(state, main_weak, 3);
        }
        CommandId::OpenSettingsShortcutsPage => {
            secondary_windows::open_settings_window_page(state, main_weak, 4);
        }
        CommandId::OpenSettingsAdvancedPage => {
            secondary_windows::open_settings_window_page(state, main_weak, 5);
        }
        CommandId::OpenAbout => {
            secondary_windows::open_about_window(state);
        }
        CommandId::OpenMenu => {
            tray_actions::open_application_context_menu(state, main_weak, None);
        }
        CommandId::ClearAllFilters => {
            update_settings(state, |settings| {
                settings.set_monitor_filter(None);
                settings.set_tag_filter(None);
                settings.set_app_filter(None);
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::ClearMonitorFilter => {
            update_settings(state, |settings| {
                settings.set_monitor_filter(None);
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::ClearTagFilter => {
            update_settings(state, |settings| {
                settings.set_tag_filter(None);
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::ClearAppFilter => {
            update_settings(state, |settings| {
                settings.set_app_filter(None);
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::LoadWorkspace(workspace_name) => {
            let _ = secondary_windows::load_workspace_into_current_instance(
                state,
                main_weak,
                workspace_name,
            );
        }
        CommandId::OpenWorkspaceInNewInstance(workspace_name) => {
            let _ = secondary_windows::open_workspace_in_new_instance(state, workspace_name);
        }
        CommandId::ToggleAnimations => {
            update_settings(state, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
            refresh_ui(state, main_weak);
        }
        CommandId::ToggleToolbar => {
            update_settings(state, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            refresh_ui(state, main_weak);
        }
        CommandId::ToggleWindowInfo => {
            update_settings(state, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
            refresh_ui(state, main_weak);
        }
        CommandId::ToggleAlwaysOnTop => {
            update_settings(state, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            let state_ref = state.borrow();
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            refresh_ui(state, main_weak);
        }
        CommandId::Exit => crate::queue_exit_request(),
    }
}

fn close_command_palette_window() {
    let taken = crate::COMMAND_PALETTE_WIN.with(|handle| handle.borrow_mut().take());
    if let Some(window) = taken {
        let _ = window.hide();
    }
}
