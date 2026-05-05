//! Command palette catalog: static and dynamic command entries.

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;
use panopticon::window_enum::enumerate_windows;
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};

use crate::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandId {
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
pub(crate) enum CommandCategory {
    Layout,
    Theme,
    System,
    Settings,
    Filters,
    Windows,
    Workspace,
}

impl CommandCategory {
    pub(crate) const fn label(self) -> &'static str {
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
pub(crate) struct CommandEntry {
    pub(crate) id: CommandId,
    pub(crate) category: CommandCategory,
    pub(crate) title: String,
    pub(crate) keywords: String,
}

#[expect(
    clippy::too_many_lines,
    reason = "base and dynamic command catalog are intentionally assembled in one contiguous list"
)]
pub(crate) fn command_entries() -> Vec<CommandEntry> {
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

pub(crate) fn command_entries_for_state(state: &AppState) -> Vec<CommandEntry> {
    let mut entries = command_entries();

    let windows = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.shell.hwnd)
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
