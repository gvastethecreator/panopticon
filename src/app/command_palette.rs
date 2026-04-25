//! Command palette window and command execution registry.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;
use panopticon::window_enum::enumerate_windows;
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, SharedString, VecModel};

use super::dock::apply_topmost_mode;
use super::layout_actions;
use super::secondary_windows;
use super::tray_actions;
use crate::{
    refresh_ui, refresh_windows, update_settings, AppState, CommandPaletteWindow, MainWindow,
};

thread_local! {
    static COMMAND_PALETTE_RECENT_KEYS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

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
    HideApp(String, String),
    ClearAllFilters,
    SetMonitorFilter(String),
    SetTagFilter(String),
    SetAppFilter(String),
    ClearMonitorFilter,
    ClearTagFilter,
    ClearAppFilter,
    RestoreHiddenApp(String),
    LoadWorkspace(Option<String>),
    OpenWorkspaceInNewInstance(Option<String>),
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    ToggleAlwaysOnTop,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandCategory {
    Layout,
    Theme,
    System,
    Settings,
    Filters,
    Windows,
    Workspace,
}

impl CommandCategory {
    const fn label(self) -> &'static str {
        match self {
            Self::Layout => "Layout",
            Self::Theme => "Theme",
            Self::System => "System",
            Self::Settings => "Settings",
            Self::Filters => "Filters",
            Self::Windows => "Windows",
            Self::Workspace => "Workspace",
        }
    }
}

#[derive(Debug, Clone)]
struct CommandEntry {
    id: CommandId,
    category: CommandCategory,
    title: String,
    keywords: String,
}

fn command_entries() -> Vec<CommandEntry> {
    let mut entries = vec![
        CommandEntry {
            id: CommandId::CycleLayout,
            category: CommandCategory::Layout,
            title: "Layout: Cycle".to_owned(),
            keywords: "layout cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Grid),
            category: CommandCategory::Layout,
            title: "Layout: Grid".to_owned(),
            keywords: "layout grid".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Mosaic),
            category: CommandCategory::Layout,
            title: "Layout: Mosaic".to_owned(),
            keywords: "layout mosaic".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Bento),
            category: CommandCategory::Layout,
            title: "Layout: Bento".to_owned(),
            keywords: "layout bento".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Fibonacci),
            category: CommandCategory::Layout,
            title: "Layout: Fibonacci".to_owned(),
            keywords: "layout fibonacci".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Columns),
            category: CommandCategory::Layout,
            title: "Layout: Columns".to_owned(),
            keywords: "layout columns".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Row),
            category: CommandCategory::Layout,
            title: "Layout: Row".to_owned(),
            keywords: "layout row".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Column),
            category: CommandCategory::Layout,
            title: "Layout: Column".to_owned(),
            keywords: "layout column".to_owned(),
        },
        CommandEntry {
            id: CommandId::ResetLayoutRatios,
            category: CommandCategory::Layout,
            title: "Layout: Reset ratios".to_owned(),
            keywords: "layout reset ratios separators".to_owned(),
        },
        CommandEntry {
            id: CommandId::CycleTheme,
            category: CommandCategory::Theme,
            title: "Theme: Cycle".to_owned(),
            keywords: "theme cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::RefreshNow,
            category: CommandCategory::System,
            title: "Refresh: Run now".to_owned(),
            keywords: "refresh update windows now".to_owned(),
        },
        CommandEntry {
            id: CommandId::RestoreAllHiddenApps,
            category: CommandCategory::Windows,
            title: "Windows: Restore all hidden apps".to_owned(),
            keywords: "windows hidden apps restore all".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettings,
            category: CommandCategory::Settings,
            title: "Open Settings".to_owned(),
            keywords: "settings preferences config".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsBehaviorPage,
            category: CommandCategory::Settings,
            title: "Settings: Behavior & Display".to_owned(),
            keywords: "settings behavior display".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsFiltersPage,
            category: CommandCategory::Settings,
            title: "Settings: Filters".to_owned(),
            keywords: "settings filters monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsWorkspacesPage,
            category: CommandCategory::Settings,
            title: "Settings: Workspaces".to_owned(),
            keywords: "settings workspaces profiles".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsShortcutsPage,
            category: CommandCategory::Settings,
            title: "Settings: Shortcuts".to_owned(),
            keywords: "settings keyboard shortcuts".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsAdvancedPage,
            category: CommandCategory::Settings,
            title: "Settings: Advanced".to_owned(),
            keywords: "settings advanced refresh dock".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenAbout,
            category: CommandCategory::Settings,
            title: "Open About".to_owned(),
            keywords: "about version update".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenMenu,
            category: CommandCategory::System,
            title: "Open App Menu".to_owned(),
            keywords: "menu context tray".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAllFilters,
            category: CommandCategory::Filters,
            title: "Filters: Clear all".to_owned(),
            keywords: "filters clear all monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearMonitorFilter,
            category: CommandCategory::Filters,
            title: "Filters: Clear monitor".to_owned(),
            keywords: "filters monitor clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearTagFilter,
            category: CommandCategory::Filters,
            title: "Filters: Clear tag".to_owned(),
            keywords: "filters tag clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAppFilter,
            category: CommandCategory::Filters,
            title: "Filters: Clear app".to_owned(),
            keywords: "filters app clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAnimations,
            category: CommandCategory::Settings,
            title: "Toggle Animations".to_owned(),
            keywords: "animations toggle transitions".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleToolbar,
            category: CommandCategory::Settings,
            title: "Toggle Status Bar".to_owned(),
            keywords: "toolbar status bar toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleWindowInfo,
            category: CommandCategory::Settings,
            title: "Toggle Window Info".to_owned(),
            keywords: "window info labels overlay toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAlwaysOnTop,
            category: CommandCategory::Settings,
            title: "Toggle Always On Top".to_owned(),
            keywords: "topmost always on top pin".to_owned(),
        },
        CommandEntry {
            id: CommandId::LoadWorkspace(None),
            category: CommandCategory::Workspace,
            title: "Workspace: Load default".to_owned(),
            keywords: "workspace load default".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(None),
            category: CommandCategory::Workspace,
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
            category: CommandCategory::Workspace,
            title: format!("Workspace: Load {workspace}"),
            keywords: format!("workspace load switch {workspace}"),
        });
        entries.push(CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(Some(workspace.clone())),
            category: CommandCategory::Workspace,
            title: format!("Workspace: Open {workspace} in new instance"),
            keywords: format!("workspace open launch new instance {workspace}"),
        });
    }

    entries.push(CommandEntry {
        id: CommandId::Exit,
        category: CommandCategory::System,
        title: "Exit Panopticon".to_owned(),
        keywords: "quit exit close app".to_owned(),
    });

    entries
}

fn command_entries_for_state(state: &AppState) -> Vec<CommandEntry> {
    let mut entries = command_entries();

    let windows = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect::<Vec<_>>();

    for monitor in collect_available_monitors(&windows) {
        entries.push(CommandEntry {
            id: CommandId::SetMonitorFilter(monitor.clone()),
            category: CommandCategory::Filters,
            title: format!("Filters: Monitor {monitor}"),
            keywords: format!("filters monitor set {monitor}"),
        });
    }

    for tag in state.settings.known_tags() {
        entries.push(CommandEntry {
            id: CommandId::SetTagFilter(tag.clone()),
            category: CommandCategory::Filters,
            title: format!("Filters: Tag {tag}"),
            keywords: format!("filters tag set {tag}"),
        });
    }

    for app in collect_available_apps(&windows) {
        entries.push(CommandEntry {
            id: CommandId::HideApp(app.app_id.clone(), app.label.clone()),
            category: CommandCategory::Windows,
            title: format!("Windows: Hide {}", app.label),
            keywords: format!("windows hide {} {}", app.label, app.app_id),
        });
        entries.push(CommandEntry {
            id: CommandId::SetAppFilter(app.app_id.clone()),
            category: CommandCategory::Filters,
            title: format!("Filters: App {}", app.label),
            keywords: format!("filters app set {} {}", app.label, app.app_id),
        });
    }

    for hidden in state.settings.hidden_app_entries() {
        entries.push(CommandEntry {
            id: CommandId::RestoreHiddenApp(hidden.app_id.clone()),
            category: CommandCategory::Windows,
            title: format!("Windows: Restore hidden {}", hidden.label),
            keywords: format!("windows hidden restore {} {}", hidden.label, hidden.app_id),
        });
    }

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
    let all_entries = Rc::new({
        let state_ref = state.borrow();
        command_entries_for_state(&state_ref)
    });
    rebuild_filtered_commands(&window, "", &filtered, all_entries.as_ref().as_slice());

    window.on_apply_filter({
        let filtered = filtered.clone();
        let all_entries = all_entries.clone();
        move || {
            crate::COMMAND_PALETTE_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(window) = guard.as_ref() else {
                    return;
                };
                let query = window.get_command_search().to_string();
                rebuild_filtered_commands(
                    window,
                    &query,
                    &filtered,
                    all_entries.as_ref().as_slice(),
                );
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
    all_entries: &[CommandEntry],
) {
    let normalized_query = query.trim().to_ascii_lowercase();

    let matches: Vec<CommandEntry> = if normalized_query.is_empty() {
        all_entries.to_vec()
    } else {
        let mut scored_matches: Vec<(i32, CommandEntry)> = all_entries
            .iter()
            .cloned()
            .filter_map(|entry| {
                let title = entry.title.to_ascii_lowercase();
                let keywords = entry.keywords.to_ascii_lowercase();
                let category = entry.category.label().to_ascii_lowercase();

                if !title.contains(&normalized_query)
                    && !keywords.contains(&normalized_query)
                    && !category.contains(&normalized_query)
                {
                    return None;
                }

                let mut score = 0;
                if title.contains(&normalized_query) {
                    score += 50;
                }
                if keywords.contains(&normalized_query) {
                    score += 25;
                }
                if category.contains(&normalized_query) {
                    score += 10;
                }
                if title.starts_with(&normalized_query) {
                    score += 30;
                }
                if title
                    .split(|character: char| !character.is_alphanumeric())
                    .any(|part| part.starts_with(&normalized_query))
                {
                    score += 20;
                }

                if let Some(recent_rank) = recent_rank_for_command(&entry.id) {
                    score += (40_i32 - i32::try_from(recent_rank).unwrap_or(40)).max(0);
                }

                Some((score, entry))
            })
            .collect();

        scored_matches.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.title.cmp(&right.1.title))
        });

        scored_matches.into_iter().map(|(_, entry)| entry).collect()
    };

    *filtered.borrow_mut() = matches.iter().map(|entry| entry.id.clone()).collect();

    let labels = if matches.is_empty() {
        vec![SharedString::from("No commands found")]
    } else {
        matches
            .iter()
            .map(|entry| {
                SharedString::from(format!("[{}] {}", entry.category.label(), entry.title))
            })
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
    remember_recent_command(&command_id);

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
        CommandId::HideApp(app_id, app_label) => {
            update_settings(state, |settings| {
                let _ = settings.toggle_hidden(&app_id, &app_label);
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
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
        CommandId::SetMonitorFilter(monitor) => {
            update_settings(state, |settings| {
                settings.set_monitor_filter(Some(&monitor));
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::SetTagFilter(tag) => {
            update_settings(state, |settings| {
                settings.set_tag_filter(Some(&tag));
            });
            if refresh_windows(state) {
                refresh_ui(state, main_weak);
            }
        }
        CommandId::SetAppFilter(app_id) => {
            update_settings(state, |settings| {
                settings.set_app_filter(Some(&app_id));
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
        CommandId::RestoreHiddenApp(app_id) => {
            update_settings(state, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
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

fn remember_recent_command(command_id: &CommandId) {
    let key = format!("{command_id:?}");
    COMMAND_PALETTE_RECENT_KEYS.with(|recent_keys| {
        let mut recent = recent_keys.borrow_mut();
        recent.retain(|candidate| candidate != &key);
        recent.insert(0, key);
        recent.truncate(12);
    });
}

fn recent_rank_for_command(command_id: &CommandId) -> Option<usize> {
    let key = format!("{command_id:?}");
    COMMAND_PALETTE_RECENT_KEYS.with(|recent_keys| {
        recent_keys
            .borrow()
            .iter()
            .position(|candidate| candidate == &key)
    })
}

fn close_command_palette_window() {
    let taken = crate::COMMAND_PALETTE_WIN.with(|handle| handle.borrow_mut().take());
    if let Some(window) = taken {
        let _ = window.hide();
    }
}
