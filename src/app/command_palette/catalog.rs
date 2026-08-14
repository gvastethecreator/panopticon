//! Command palette catalog: static and dynamic command entries.

use panopticon::i18n;
use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;
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
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Layout => i18n::t("command.category.layout"),
            Self::Theme => i18n::t("command.category.theme"),
            Self::System => i18n::t("command.category.system"),
            Self::Settings => i18n::t("command.category.settings"),
            Self::Filters => i18n::t("command.category.filters"),
            Self::Windows => i18n::t("command.category.windows"),
            Self::Workspace => i18n::t("command.category.workspace"),
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
            title: i18n::t("command.layout_cycle").to_owned(),
            keywords: "layout cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Grid),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_grid").to_owned(),
            keywords: "layout grid".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Mosaic),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_mosaic").to_owned(),
            keywords: "layout mosaic".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Bento),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_bento").to_owned(),
            keywords: "layout bento".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Fibonacci),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_fibonacci").to_owned(),
            keywords: "layout fibonacci".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Columns),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_columns").to_owned(),
            keywords: "layout columns".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Row),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_row").to_owned(),
            keywords: "layout row".to_owned(),
        },
        CommandEntry {
            id: CommandId::SetLayout(LayoutType::Column),
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_column").to_owned(),
            keywords: "layout column".to_owned(),
        },
        CommandEntry {
            id: CommandId::ResetLayoutRatios,
            category: CommandCategory::Layout,
            title: i18n::t("command.layout_reset_ratios").to_owned(),
            keywords: "layout reset ratios separators".to_owned(),
        },
        CommandEntry {
            id: CommandId::CycleTheme,
            category: CommandCategory::Theme,
            title: i18n::t("command.theme_cycle").to_owned(),
            keywords: "theme cycle next".to_owned(),
        },
        CommandEntry {
            id: CommandId::RefreshNow,
            category: CommandCategory::System,
            title: i18n::t("command.refresh_now").to_owned(),
            keywords: "refresh update windows now".to_owned(),
        },
        CommandEntry {
            id: CommandId::RestoreAllHiddenApps,
            category: CommandCategory::Windows,
            title: i18n::t("command.restore_all_hidden").to_owned(),
            keywords: "windows hidden apps restore all".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettings,
            category: CommandCategory::Settings,
            title: i18n::t("command.open_settings").to_owned(),
            keywords: "settings preferences config".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsBehaviorPage,
            category: CommandCategory::Settings,
            title: i18n::t("command.settings_behavior").to_owned(),
            keywords: "settings behavior display".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsFiltersPage,
            category: CommandCategory::Settings,
            title: i18n::t("command.settings_filters").to_owned(),
            keywords: "settings filters monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsWorkspacesPage,
            category: CommandCategory::Settings,
            title: i18n::t("command.settings_workspaces").to_owned(),
            keywords: "settings workspaces profiles".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsShortcutsPage,
            category: CommandCategory::Settings,
            title: i18n::t("command.settings_shortcuts").to_owned(),
            keywords: "settings keyboard shortcuts".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenSettingsAdvancedPage,
            category: CommandCategory::Settings,
            title: i18n::t("command.settings_advanced").to_owned(),
            keywords: "settings advanced refresh dock".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenAbout,
            category: CommandCategory::Settings,
            title: i18n::t("command.open_about").to_owned(),
            keywords: "about version update".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenMenu,
            category: CommandCategory::System,
            title: i18n::t("command.open_menu").to_owned(),
            keywords: "menu context tray".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAllFilters,
            category: CommandCategory::Filters,
            title: i18n::t("command.clear_all_filters").to_owned(),
            keywords: "filters clear all monitor tag app".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearMonitorFilter,
            category: CommandCategory::Filters,
            title: i18n::t("command.clear_monitor_filter").to_owned(),
            keywords: "filters monitor clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearTagFilter,
            category: CommandCategory::Filters,
            title: i18n::t("command.clear_tag_filter").to_owned(),
            keywords: "filters tag clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ClearAppFilter,
            category: CommandCategory::Filters,
            title: i18n::t("command.clear_app_filter").to_owned(),
            keywords: "filters app clear".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAnimations,
            category: CommandCategory::Settings,
            title: i18n::t("command.toggle_animations").to_owned(),
            keywords: "animations toggle transitions".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleToolbar,
            category: CommandCategory::Settings,
            title: i18n::t("command.toggle_toolbar").to_owned(),
            keywords: "toolbar status bar toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleWindowInfo,
            category: CommandCategory::Settings,
            title: i18n::t("command.toggle_window_info").to_owned(),
            keywords: "window info labels overlay toggle".to_owned(),
        },
        CommandEntry {
            id: CommandId::ToggleAlwaysOnTop,
            category: CommandCategory::Settings,
            title: i18n::t("command.toggle_always_on_top").to_owned(),
            keywords: "topmost always on top pin".to_owned(),
        },
        CommandEntry {
            id: CommandId::LoadWorkspace(None),
            category: CommandCategory::Workspace,
            title: i18n::t("command.workspace_load_default").to_owned(),
            keywords: "workspace load default".to_owned(),
        },
        CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(None),
            category: CommandCategory::Workspace,
            title: i18n::t("command.workspace_open_default").to_owned(),
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
            title: i18n::t_fmt("command.workspace_load", &workspace),
            keywords: format!("workspace load switch {workspace}"),
        });
        entries.push(CommandEntry {
            id: CommandId::OpenWorkspaceInNewInstance(Some(workspace.clone())),
            category: CommandCategory::Workspace,
            title: i18n::t_fmt("command.workspace_open", &workspace),
            keywords: format!("workspace open launch new instance {workspace}"),
        });
    }

    entries.push(CommandEntry {
        id: CommandId::Exit,
        category: CommandCategory::System,
        title: i18n::t("command.exit").to_owned(),
        keywords: "quit exit close app".to_owned(),
    });

    entries
}

pub(crate) fn command_entries_for_state(state: &AppState) -> Vec<CommandEntry> {
    let mut entries = command_entries();

    let windows = state.window_collection.catalog.windows();

    for monitor in collect_available_monitors(windows) {
        entries.push(CommandEntry {
            id: CommandId::SetMonitorFilter(monitor.clone()),
            category: CommandCategory::Filters,
            title: i18n::t_fmt("command.filter_monitor", &monitor),
            keywords: format!("filters monitor set {monitor}"),
        });
    }

    for tag in state.settings.known_tags() {
        entries.push(CommandEntry {
            id: CommandId::SetTagFilter(tag.clone()),
            category: CommandCategory::Filters,
            title: i18n::t_fmt("command.filter_tag", &tag),
            keywords: format!("filters tag set {tag}"),
        });
    }

    for app in collect_available_apps(windows) {
        entries.push(CommandEntry {
            id: CommandId::HideApp(app.app_id.clone(), app.label.clone()),
            category: CommandCategory::Windows,
            title: i18n::t_fmt("command.hide_app", &app.label),
            keywords: format!("windows hide {} {}", app.label, app.app_id),
        });
        entries.push(CommandEntry {
            id: CommandId::SetAppFilter(app.app_id.clone()),
            category: CommandCategory::Filters,
            title: i18n::t_fmt("command.filter_app", &app.label),
            keywords: format!("filters app set {} {}", app.label, app.app_id),
        });
    }

    for hidden in state.settings.hidden_app_entries() {
        entries.push(CommandEntry {
            id: CommandId::RestoreHiddenApp(hidden.app_id.clone()),
            category: CommandCategory::Windows,
            title: i18n::t_fmt("command.restore_hidden_app", &hidden.label),
            keywords: format!("windows hidden restore {} {}", hidden.label, hidden.app_id),
        });
    }

    entries
}
