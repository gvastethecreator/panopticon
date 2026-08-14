//! Internationalization support for Panopticon.
//!
//! The application ships with English (default) and Spanish translations.
//! A persisted language preference can be overridden at runtime with the
//! `PANOPTICON_LANG` environment variable.
//!
//! # Locale resolution order
//!
//! 1. `PANOPTICON_LANG` environment variable (`en`, `es`).
//! 2. Persisted application setting.
//! 3. English.

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

// ── Locale type ──────────────────────────────────────────────

/// Supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    #[default]
    English,
    Spanish,
}

static LOCALE: RwLock<Locale> = RwLock::new(Locale::English);

/// Resolve and store the active UI locale.
pub fn init(preferred: Locale) {
    let locale = set_locale(preferred);
    tracing::info!(?locale, "i18n locale resolved");
}

/// Update the active locale and return the effective value.
#[must_use]
pub fn set_locale(preferred: Locale) -> Locale {
    let locale = resolve_locale(preferred);
    *LOCALE
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = locale;
    locale
}

/// Return the active locale.
#[must_use]
pub fn current() -> Locale {
    *LOCALE
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Translate a key to the active locale, falling back to English.
#[must_use]
pub fn t(key: &str) -> &'static str {
    match current() {
        Locale::English => en(key),
        Locale::Spanish => es(key).unwrap_or_else(|| en(key)),
    }
}

/// Format a translated string with a single argument.
#[must_use]
pub fn t_fmt(key: &str, arg: &str) -> String {
    let template = t(key);
    template.replacen("{}", arg, 1)
}

// ── Locale detection ─────────────────────────────────────────

fn resolve_locale(preferred: Locale) -> Locale {
    if let Ok(lang) = std::env::var("PANOPTICON_LANG") {
        return parse_locale_tag(&lang);
    }

    preferred
}

fn parse_locale_tag(tag: &str) -> Locale {
    let lower = tag.to_ascii_lowercase();
    if lower.starts_with("es") {
        Locale::Spanish
    } else {
        Locale::English
    }
}

// ── English translations (default) ──────────────────────────

#[allow(
    clippy::too_many_lines,
    clippy::match_same_arms,
    reason = "translation catalogs intentionally reuse the same copy for multiple keys"
)]
fn en(key: &str) -> &'static str {
    match key {
        // ── App identity ──
        "app.name" => "Panopticon",
        "window.main_title" => "Panopticon",
        "window.settings_title" => "Panopticon — Settings",
        "window.tag_title" => "Panopticon — Create tag",
        "window.about_title" => "Panopticon — About",
        "window.command_palette_title" => "Panopticon — Command Palette",

        // ── Locales ──
        "locale.english" => "English",
        "locale.spanish" => "Spanish",

        // ── Layout labels ──
        "layout.grid" => "Grid",
        "layout.mosaic" => "Mosaic",
        "layout.bento" => "Bento",
        "layout.fibonacci" => "Fibonacci",
        "layout.columns" => "Columns",
        "layout.row" => "Row",
        "layout.column" => "Column",

        // ── Window context menu ──
        "menu.hide_from_layout" => "Hide from layout",
        "menu.pin_position" => "Pin app at this position",
        "menu.preserve_aspect" => "Preserve aspect ratio",
        "menu.hide_on_select" => "Hide Panopticon when activating this app",
        "menu.create_tag" => "Create custom tag…",
        "menu.thumbnail_refresh" => "Thumbnail refresh mode",
        "menu.thumbnail_refresh_realtime" => "Realtime",
        "menu.thumbnail_refresh_frozen" => "Frozen",
        "menu.thumbnail_refresh_interval" => "Interval",
        "menu.cell_color" => "Cell colour",
        "menu.use_theme_color" => "Use theme colour",
        "menu.close_window" => "Close window",
        "menu.kill_process" => "Kill process",

        // ── Colour presets ──
        "color.amber" => "Use amber",
        "color.sky" => "Use sky",
        "color.mint" => "Use mint",
        "color.rose" => "Use rose",
        "color.violet" => "Use violet",
        "color.sun" => "Use sun",
        "tag.color.amber" => "Amber",
        "tag.color.sky" => "Sky",
        "tag.color.mint" => "Mint",
        "tag.color.rose" => "Rose",
        "tag.color.violet" => "Violet",
        "tag.color.sun" => "Sun",

        // ── Tray tooltip ──
        "tray.tooltip" => "Panopticon — Live window overview",

        // ── Tray menu ──
        "tray.visibility" => "Visibility",
        "tray.show" => "Show Panopticon",
        "tray.hide" => "Hide to tray",
        "tray.refresh" => "Refresh windows",
        "tray.open_settings" => "Open settings window",
        "tray.open_about" => "About Panopticon",
        "tray.profiles" => "Workspaces",
        "tray.profile_default" => "default",
        "tray.workspaces" => "Workspaces",
        "tray.workspace_default" => "default",
        "tray.layout" => "Layout",
        "tray.next_layout" => "Next layout",
        "tray.lock_layout" => "Lock layout switching",
        "tray.lock_resize" => "Lock cell / column resizing",
        "tray.dock_position" => "Dock position",
        "tray.group_by" => "Group windows by",
        "tray.display" => "Display",
        "tray.show_toolbar" => "Show status bar",
        "tray.show_info" => "Show window info",
        "tray.show_icons" => "Show app icons in cells",
        "tray.always_on_top" => "Keep Panopticon on top",
        "tray.behaviour" => "Behaviour",
        "tray.minimize_to_tray" => "Hide on minimize",
        "tray.close_to_tray" => "Hide on close",
        "tray.cycle_refresh" => "Cycle refresh interval ({})",
        "tray.animate" => "Animate transitions",
        "tray.default_aspect" => "Default: preserve aspect ratio",
        "tray.default_hide" => "Default: hide after activation",
        "tray.start_tray" => "Start hidden in tray",
        "tray.filters" => "Filters",
        "tray.filter_monitor" => "Filter by monitor",
        "tray.all_monitors" => "All monitors",
        "tray.filter_tag" => "Filter by tag",
        "tray.all_tags" => "All tags",
        "tray.filter_app" => "Filter by application",
        "tray.all_apps" => "All applications",
        "tray.restore_hidden" => "Restore hidden apps",
        "tray.restore_all" => "Restore all hidden apps",
        "tray.exit" => "Exit",

        // ── Dock submenu ──
        "dock.none" => "Floating (no dock)",
        "dock.left" => "Left",
        "dock.right" => "Right",
        "dock.top" => "Top",
        "dock.bottom" => "Bottom",

        // ── Grouping submenu ──
        "group.none" => "No grouping",
        "group.application" => "Application",
        "group.monitor" => "Monitor",
        "group.title" => "Window title",
        "group.class" => "Window class",
        "filter.grouped_by" => "grouped by:",

        // ── UI labels (Slint) ──
        "ui.minimized" => "minimized",
        "ui.last_seen" => "LAST SEEN",
        "ui.visible" => "visible",
        "ui.hidden" => "hidden",
        "ui.always_on_top" => "always on top",
        "ui.normal_window" => "normal window",
        "ui.toolbar_hint" => "right-click status bar / M: menu  ·  Esc exit",
        "ui.anim_on" => "anim on",
        "ui.anim_off" => "anim off",

        // ── Empty state ──
        "ui.empty_message" => "No windows available to preview",
        "ui.empty_helper" => {
            "Open or restore any desktop window.\nPanopticon will keep watching from the tray."
        }

        // ── Settings ──
        "settings.hidden_app_fallback" => "Hidden app",
        "settings.dock_hint" => "In dock mode this option is automatically disabled.",
        "settings.filters_hint" => {
            "Filters and grouping are also reflected in the status bar and reorder visible cells."
        }
        "settings.no_saved_profiles" => "No saved workspaces",
        "settings.default_profile" => "default",
        "settings.saved_profiles" => "Saved workspaces: default",
        "settings.saved_profiles_fmt" => "Saved workspaces: {}",
        "settings.current_profile" => "Current workspace: ",
        "settings.profile_label" => "Workspace:",
        "settings.save_profile" => "Save workspace",
        "settings.open_instance" => "Open another instance",
        "settings.no_hidden_hint" => "No hidden apps to restore right now.",
        "settings.no_hidden" => "No hidden apps",
        "settings.hidden_one" => "1 hidden app ready to restore",
        "settings.hidden_many" => "{} hidden apps ready to restore",
        "settings.title" => "Settings",
        "settings.subtitle" => {
            "Customize the dashboard, backgrounds, shortcuts, and overall behavior."
        }
        "settings.profile_badge" => "Profile",
        "settings.nav.behaviour_display.title" => "Behaviour & Display",
        "settings.nav.behaviour_display.subtitle" => {
            "Window behaviour, tray and visible chrome"
        }
        "settings.nav.filters.title" => "Filters",
        "settings.nav.filters.subtitle" => "Monitor, tag, app and hidden-state tools",
        "settings.nav.theme_background.title" => "Theme & Background",
        "settings.nav.theme_background.subtitle" => {
            "Theme presets, solid canvas colour and image"
        }
        "settings.nav.profiles.title" => "Profiles (Workspaces)",
        "settings.nav.profiles.subtitle" => "Save and launch named setups",
        "settings.nav.shortcuts.title" => "Keyboard Shortcuts",
        "settings.nav.shortcuts.subtitle" => "Customize the dashboard key map",
        "settings.nav.advanced.title" => "Advanced Options",
        "settings.nav.advanced.subtitle" => "Layout, refresh cadence and dock behaviour",
        "settings.page.behaviour_display.title" => "Behaviour & Display",
        "settings.page.behaviour_display.subtitle" => {
            "Adjust how the main window behaves, what information is shown, and how it responds to the tray."
        }
        "settings.section.behaviour.title" => "Behaviour",
        "settings.section.behaviour.helper" => {
            "Each option includes a small summary so you do not have to guess what it does."
        }
        "settings.option.language.title" => "Language",
        "settings.option.language.description" => {
            "Choose the application language. English is the default; Spanish is also available."
        }
        "settings.option.always_on_top.title" => "Always on top",
        "settings.option.always_on_top.description" => {
            "Keep Panopticon above other windows even while switching applications."
        }
        "settings.option.animate_transitions.title" => "Animate transitions",
        "settings.option.animate_transitions.description" => {
            "Smooth layout changes, filters, and visual reordering between thumbnails."
        }
        "settings.option.minimize_to_tray.title" => "Minimize to tray",
        "settings.option.minimize_to_tray.description" => {
            "When minimized, hide the app from the desktop while keeping it alive in the system tray."
        }
        "settings.option.close_to_tray.title" => "Close to tray",
        "settings.option.close_to_tray.description" => {
            "Interpret the window close button as hiding to the tray instead of exiting."
        }
        "settings.option.preserve_aspect_ratio.title" => "Preserve aspect ratio by default",
        "settings.option.preserve_aspect_ratio.description" => {
            "New applications will better respect the original proportion of their thumbnails."
        }
        "settings.option.hide_on_select.title" => "Hide after selecting an app",
        "settings.option.hide_on_select.description" => {
            "Hide Panopticon when you activate a window from the dashboard."
        }
        "settings.option.start_in_tray.title" => "Start hidden in tray",
        "settings.option.start_in_tray.description" => {
            "Launch directly in the background for a quieter startup."
        }
        "settings.option.run_at_startup.title" => "Run at startup",
        "settings.option.run_at_startup.description" => {
            "Register Panopticon in the current Windows session so it launches automatically when you sign in."
        }
        "settings.option.lock_layout.title" => "Lock layout changes",
        "settings.option.lock_layout.description" => {
            "Prevent layout changes from the keyboard or the application menu."
        }
        "settings.option.lock_cell_resize.title" => "Lock cell resizing",
        "settings.option.lock_cell_resize.description" => {
            "Disable separator dragging to protect the current composition."
        }
        "settings.section.display.title" => "Display",
        "settings.section.display.helper" => {
            "Controls for layout-at-startup, dock placement, sizing, and dashboard readability."
        }
        "settings.option.show_toolbar.title" => "Show status bar",
        "settings.option.show_toolbar.description" => {
            "Show the dashboard status bar with status summary and quick access to the menu."
        }
        "settings.toolbar_position.top" => "Top",
        "settings.toolbar_position.bottom" => "Bottom",
        "settings.option.toolbar_position.title" => "Status bar position",
        "settings.option.toolbar_position.description" => {
            "Choose whether the status bar stays at the top or bottom of the dashboard."
        }
        "settings.option.show_info.title" => "Show window info above thumbnails",
        "settings.option.show_info.description" => {
            "Add the window and application name above each preview for quick context."
        }
        "settings.option.show_app_icons.title" => "Show app icons in cells",
        "settings.option.show_app_icons.description" => {
            "Render the process icon inside each cell to identify apps faster."
        }
        "settings.page.filters.title" => "Filters",
        "settings.page.filters.subtitle" => {
            "Limit the dashboard by monitor, tags, applications, or groups, and restore hidden apps without leaving this view."
        }
        "settings.option.monitor_filter.title" => "Monitor filter",
        "settings.option.monitor_filter.description" => {
            "Limit the dashboard to a specific monitor when working with multiple displays."
        }
        "settings.option.tag_filter.title" => "Tag filter",
        "settings.option.tag_filter.description" => {
            "Show only applications associated with a specific manual tag."
        }
        "settings.option.app_filter.title" => "Application filter",
        "settings.option.app_filter.description" => {
            "Isolate a specific app when you want to review only its window group."
        }
        "settings.option.group_windows.title" => "Group windows by",
        "settings.option.group_windows.description" => {
            "Visually reorder the list without filtering content, which is ideal for spotting patterns."
        }
        "settings.section.hidden_apps.title" => "Hidden applications",
        "settings.section.hidden_apps.helper" => {
            "Restore hidden apps one by one or all at once from persisted state."
        }
        "settings.section.app_rules.title" => "App Rules Manager",
        "settings.section.app_rules.helper" => {
            "Edit per-app rules for visibility, aspect ratio, hide-on-select, refresh mode, tags, and color."
        }
        "settings.option.app_rules.app.title" => "Application",
        "settings.option.app_rules.app.description" => {
            "Includes running apps and apps with saved rules."
        }
        "settings.option.app_rules.search.placeholder" => "Search by app, id, or tags...",
        "settings.app_rules.filter.all" => "All apps",
        "settings.app_rules.filter.running" => "Running apps",
        "settings.app_rules.filter.saved" => "Saved rules only",
        "settings.app_rules.filter.hidden" => "Hidden apps",
        "settings.app_rules.filter.tagged" => "Tagged apps",
        "settings.app_rules.filter.refresh" => "Custom refresh",
        "settings.app_rules.filter.pinned" => "Pinned apps",
        "settings.app_rules.active.title" => "Active rule",
        "settings.app_rules.active.badge" => "APP RULE",
        "settings.app_rules.hidden.title" => "Hide app in dashboard",
        "settings.app_rules.hidden.description" => {
            "When enabled, this app does not appear in the main grid."
        }
        "settings.app_rules.preserve_aspect.title" => "Preserve aspect ratio",
        "settings.app_rules.preserve_aspect.description" => {
            "Per-app override for the global aspect setting."
        }
        "settings.app_rules.hide_on_select.title" => "Hide Panopticon on select",
        "settings.app_rules.hide_on_select.description" => {
            "Per-app override for hide-on-select behavior."
        }
        "settings.app_rules.refresh_mode.title" => "Thumbnail refresh mode",
        "settings.app_rules.refresh_mode.description" => "Realtime, Frozen, or Interval.",
        "settings.app_rules.refresh_mode.realtime" => "Realtime",
        "settings.app_rules.refresh_mode.frozen" => "Frozen",
        "settings.app_rules.refresh_mode.interval" => "Interval",
        "settings.app_rules.interval.title" => "Interval (ms)",
        "settings.app_rules.interval.description" => {
            "Applied only when mode is Interval."
        }
        "settings.app_rules.tags.title" => "Tags (CSV)",
        "settings.app_rules.tags.description" => "Example: work, browser, stream",
        "settings.app_rules.tags.placeholder" => "work, browser",
        "settings.app_rules.tags.helper" => "Add tags one by one and remove them with a click.",
        "settings.app_rules.tags.add" => "Add tag",
        "settings.app_rules.tags.remove_hint" => "Click a chip to remove it.",
        "settings.app_rules.tags.suggestions_title" => "Quick suggestions",
        "settings.app_rules.tags.suggestion.work" => "work",
        "settings.app_rules.tags.suggestion.browser" => "browser",
        "settings.app_rules.tags.suggestion.dev" => "dev",
        "settings.app_rules.tags.suggestion.stream" => "stream",
        "settings.app_rules.color.title" => "Hex color (optional)",
        "settings.app_rules.color.description" => {
            "Use RRGGBB format, or empty to clear override."
        }
        "settings.app_rules.color.placeholder" => "5CA9FF",
        "settings.app_rules.apply" => "Apply rule",
        "settings.app_rules.reset" => "Reset rule",
        "settings.app_rules.clear_unused" => "Clear inactive rules",
        "settings.app_rules.no_selection" => "Select an application to edit its rule.",
        "settings.app_rules.select_option" => "Select application rule…",
        "settings.app_rules.cleanup.none" => "No inactive rules to clean.",
        "settings.app_rules.cleanup.count" => "{} inactive rules can be cleaned.",
        "settings.page.theme_background.title" => "Theme & Background",
        "settings.page.theme_background.subtitle" => {
            "Pick a theme preset, fine-tune core colours, and combine it with a custom canvas colour and background image."
        }
        "settings.section.theme_grid.title" => "Theme presets",
        "settings.section.theme_grid.helper" => {
            "Pick a preset from a compact list; the theme defines accents, panels, and overall contrast."
        }
        "settings.section.theme_colours.title" => "Theme colours",
        "settings.section.theme_colours.helper" => {
            "Adjust core colours of the active theme. Leave a field empty to keep the preset value."
        }
        "settings.section.canvas_background.title" => "Canvas background",
        "settings.section.canvas_background.helper" => {
            "The canvas colour sits behind the cards and the optional background image."
        }
        "settings.option.custom_canvas_colour.title" => "Custom canvas colour",
        "settings.option.custom_canvas_colour.description" => {
            "Enter a manual RGB hex value if you want a colour outside the quick palette."
        }
        "settings.section.preview.title" => "Preview",
        "settings.section.preview.helper" => {
            "Quick summary of the active background with colour and optional image."
        }
        "settings.section.background_image.title" => "Background image",
        "settings.section.background_image.helper" => {
            "Place an image behind the dashboard and define how it should fit within the canvas."
        }
        "settings.option.image_file.title" => "Image file",
        "settings.option.image_file.description" => {
            "You can paste a path manually or choose it with the native picker."
        }
        "settings.option.image_fit.title" => "Image fit",
        "settings.option.image_fit.description" => {
            "Control whether the image covers, contains, or fills the visible dashboard area."
        }
        "settings.option.image_opacity.title" => "Image opacity",
        "settings.option.image_opacity.description" => {
            "Control how strong the background image should appear over the canvas colour."
        }
        "settings.theme_colours.accent" => "Accent",
        "settings.theme_colours.surface" => "Surface",
        "settings.theme_colours.card" => "Card",
        "settings.theme_colours.text" => "Text",
        "settings.theme_colours.muted" => "Muted",
        "settings.theme_colours.border" => "Border",
        "settings.theme_colours.hint" => "Leave blank to inherit the current preset colour.",
        "settings.fit.cover" => "Cover",
        "settings.fit.contain" => "Contain",
        "settings.fit.fill" => "Fill",
        "settings.fit.preserve" => "Preserve",
        "settings.page.profiles.title" => "Workspaces",
        "settings.page.profiles.subtitle" => {
            "Save complete combinations of settings and open new instances already pointed at the workspace you want."
        }
        "settings.section.edit_profile.title" => "Edit workspace",
        "settings.section.edit_profile.helper" => {
            "Use a short, descriptive name to save or open the current snapshot in another instance."
        }
        "settings.current_profile_card.title" => "Current workspace",
        "settings.option.profile_name.title" => "Workspace name",
        "settings.option.profile_name.description" => {
            "Name used to save this setup or launch another instance with it."
        }
        "settings.section.saved_profiles.title" => "Saved workspaces",
        "settings.section.saved_profiles.helper" => {
            "Summary of detected workspaces plus a reminder of the recommended multi-instance workflow."
        }
        "settings.section.load_profile.title" => "Switch current instance",
        "settings.section.load_profile.helper" => {
            "Load another saved workspace into this running Panopticon window."
        }
        "settings.option.available_profile.title" => "Available workspaces",
        "settings.option.available_profile.description" => {
            "Choose which saved workspace this instance should load right now."
        }
        "settings.tips.title" => "Tips",
        "settings.tips.body" => {
            "- Save the current workspace first if you are about to open another instance.\n- Use simple names like work, stream, or review.\n- Theme, background, and shortcut settings travel with the workspace."
        }
        "settings.page.shortcuts.title" => "Keyboard Shortcuts",
        "settings.page.shortcuts.subtitle" => {
            "Dashboard shortcuts use a single key; global activation accepts Ctrl / Alt / Shift plus a key like P or Space."
        }
        "settings.section.layout_bindings.title" => "Layout bindings",
        "settings.section.layout_bindings.helper" => {
            "Direct assignments for specific layouts, reset, and the general cycle action."
        }
        "settings.shortcut.layout_grid.title" => "Grid layout",
        "settings.shortcut.layout_grid.description" => "Switch to the Grid view instantly.",
        "settings.shortcut.layout_mosaic.title" => "Mosaic layout",
        "settings.shortcut.layout_mosaic.description" => "Switch to the Mosaic layout.",
        "settings.shortcut.layout_bento.title" => "Bento layout",
        "settings.shortcut.layout_bento.description" => "Activate the Bento layout.",
        "settings.shortcut.layout_fibonacci.title" => "Fibonacci layout",
        "settings.shortcut.layout_fibonacci.description" => "Open the Fibonacci composition.",
        "settings.shortcut.layout_columns.title" => "Columns layout",
        "settings.shortcut.layout_columns.description" => "Activate Columns.",
        "settings.shortcut.layout_row.title" => "Row layout",
        "settings.shortcut.layout_row.description" => "Activate the Row view.",
        "settings.shortcut.layout_column.title" => "Column layout",
        "settings.shortcut.layout_column.description" => "Activate the Column view.",
        "settings.shortcut.reset_layout.title" => "Reset layout ratios",
        "settings.shortcut.reset_layout.description" => {
            "Reset custom proportions for the current layout."
        }
        "settings.shortcut.cycle_layout.title" => "Cycle layout",
        "settings.shortcut.cycle_layout.description" => {
            "Advance to the next layout in the internal sequence."
        }
        "settings.section.dashboard_actions.title" => "Dashboard actions",
        "settings.section.dashboard_actions.helper" => {
            "Shortcuts for opening panels, refreshing state, and toggling visible options."
        }
        "settings.shortcut.cycle_theme.title" => "Cycle theme",
        "settings.shortcut.cycle_theme.description" => {
            "Press T for the next theme or Shift+T for the previous one."
        }
        "settings.shortcut.toggle_animations.title" => "Toggle animations",
        "settings.shortcut.toggle_animations.description" => {
            "Enable or disable dashboard transitions."
        }
        "settings.shortcut.toggle_toolbar.title" => "Toggle status bar",
        "settings.shortcut.toggle_toolbar.description" => {
            "Show or hide the dashboard's bottom status bar."
        }
        "settings.shortcut.toggle_window_info.title" => "Toggle window info",
        "settings.shortcut.toggle_window_info.description" => {
            "Show or hide titles and info above thumbnails."
        }
        "settings.shortcut.toggle_always_on_top.title" => "Toggle always on top",
        "settings.shortcut.toggle_always_on_top.description" => {
            "Toggle the always-on-top mode above other apps."
        }
        "settings.shortcut.open_settings.title" => "Open settings",
        "settings.shortcut.open_settings.description" => {
            "Open this window from the main dashboard."
        }
        "settings.shortcut.open_menu.title" => "Open application menu",
        "settings.shortcut.open_menu.description" => {
            "Open the main native menu with quick actions."
        }
        "settings.shortcut.open_command_palette.title" => "Open command palette",
        "settings.shortcut.open_command_palette.description" => {
            "Open the quick command search panel."
        }
        "settings.shortcut.global_activate.title" => "Activate Panopticon globally",
        "settings.shortcut.global_activate.description" => {
            "Bring Panopticon to the foreground from anywhere. Leave empty to disable it."
        }
        "settings.shortcut.refresh_now.title" => "Refresh now",
        "settings.shortcut.refresh_now.description" => {
            "Force a new window enumeration and refresh the dashboard."
        }
        "settings.shortcut.exit_app.title" => "Exit app",
        "settings.shortcut.exit_app.description" => "Close Panopticon from the keyboard.",
        "settings.shortcut.alt_toolbar.title" => "Use Alt as a quick status bar toggle",
        "settings.shortcut.alt_toolbar.description" => {
            "Keep the legacy Windows shortcut to hide or show the status bar with a single Alt press."
        }
        "settings.page.advanced.title" => "Advanced Options",
        "settings.page.advanced.subtitle" => {
            "Manual refresh/update controls and the base cadence used for background window discovery."
        }
        "settings.option.default_layout.title" => "Default layout",
        "settings.option.default_layout.description" => {
            "Choose which layout Panopticon should use each time this profile starts."
        }
        "settings.option.default_layout.docked_description" => {
            "Dock mode automatically uses Column on the sides and Row on the top or bottom, so this selector only applies while floating."
        }
        "settings.option.refresh_interval.title" => "Refresh interval",
        "settings.option.refresh_interval.description" => {
            "Base cadence for enumerating windows and refreshing the dashboard when you do not force a manual refresh."
        }
        "settings.refresh_mode.realtime" => "Realtime",
        "settings.refresh_mode.balanced" => "Balanced",
        "settings.refresh_mode.battery_saver" => "Battery Saver",
        "settings.refresh_mode.manual" => "Manual",
        "settings.section.refresh_performance.title" => "Refresh performance mode",
        "settings.section.refresh_performance.helper" => {
            "Realtime/Balanced/Battery Saver set cadence automatically. Manual uses the explicit interval below."
        }
        "settings.option.refresh_performance_mode.title" => "Refresh performance mode",
        "settings.option.refresh_performance_mode.description" => {
            "Global cadence profile for window discovery and refresh."
        }
        "settings.refresh_mode.active.title" => "Active mode",
        "settings.refresh_mode.active.realtime" => "Realtime (1s)",
        "settings.refresh_mode.active.balanced" => "Balanced (2s)",
        "settings.refresh_mode.active.battery" => "Battery Saver (5s)",
        "settings.refresh_mode.active.manual" => "Manual (uses explicit interval)",
        "settings.section.manual_refresh.title" => "Manual refresh",
        "settings.section.manual_refresh.helper" => {
            "Use this section to force a window refresh or to check for updates immediately."
        }
        "settings.section.dock_thickness.title" => "Dock thickness",
        "settings.section.dock_thickness.helper" => {
            "For a side dock, width is used; for top/bottom, height is used. Values are clamped to safe minimums."
        }
        "settings.section.floating_window_size.title" => "Floating window size",
        "settings.section.floating_window_size.helper" => {
            "When dock mode is disabled, these values track the floating window size and are clamped to safe minimums."
        }
        "settings.option.thumbnail_render_scale.title" => "Thumbnail render scale",
        "settings.option.thumbnail_render_scale.description" => {
            "Choose 25%, 50%, 75%, or 100% thumbnail detail. Lower values trade sharpness for better performance while keeping the card footprint nearly the same."
        }
        "settings.label.width" => "Width",
        "settings.label.height" => "Height",
        "settings.version_label" => "Version:",
        "settings.update_status.idle" => "Update check pending",
        "settings.update_status.checking" => "Checking for updates…",
        "settings.update_status.up_to_date" => "Up to date ({})",
        "settings.update_status.available" => "New version available ({})",
        "settings.update_status.failed" => "Update check failed",
        "settings.persistence_status.failed" => {
            "Changes are active for this session, but could not be saved. Check the logs and try again."
        }
        "settings.persistence_status.failed_title" => "Settings were not saved",
        "settings.persistence.retry" => "Retry save",
        "settings.persistence.open_logs" => "Open logs",
        "settings.update.cancel" => "Cancel update check",
        "settings.option.center_secondary.title" => "Center secondary windows",
        "settings.option.center_secondary.description" => {
            "Open Settings, About, Command Palette and tag dialogs centered on the monitor."
        }
        "settings.app_rules.pinned_slot.title" => "Pinned slot",
        "settings.app_rules.pinned_slot.description" => {
            "0 = unpinned. Positive values reserve a preferred slot."
        }
        "settings.app_rules.pinned_slot.conflict" => "Pinned slot conflict",
        "settings.theme_catalog.title" => "Theme catalog",
        "settings.theme_catalog.description" => {
            "Select any installed theme. Preview cards show a bounded subset."
        }
        "settings.workspace.display_name.title" => "Display name",
        "settings.workspace.display_name.description" => "Friendly name shown in workspace lists.",
        "settings.workspace.description.title" => "Description",
        "settings.workspace.description.description" => {
            "Optional note to remember what this workspace is for."
        }
        "settings.workspace.name_placeholder" => "workspace-a",
        "settings.workspace.display_name_placeholder" => "Work · Deep focus",
        "settings.workspace.description_placeholder" => "Pinned apps + filters for design review sessions",
        "settings.workspace.action.duplicate" => "Duplicate",
        "settings.workspace.action.rename" => "Rename",
        "settings.workspace.action.delete" => "Delete",
        "settings.workspace.action.failed" => "Workspace action failed",
        "settings.workspace.action.completed" => "Workspace action completed",
        "settings.workspace.active" => "ACTIVE",
        "settings.workspace.default" => "DEFAULT",
        "settings.workspace.metadata.title" => "Selected workspace metadata",
        "settings.workspace.name_label" => "Workspace",
        "settings.workspace.updated_label" => "Last updated",
        "settings.workspace.created_label" => "Created",
        "settings.workspace.runtime_status_label" => "Runtime status",
        "settings.workspace.not_saved" => "Not saved yet",
        "settings.workspace.unknown" => "Unknown",
        "settings.workspace.no_runtime" => "No runtime diagnostics yet",
        "settings.workspace.running" => "RUNNING",
        "settings.workspace.modified" => "MODIFIED",
        "settings.workspace.feedback.saved" => "Workspace {} saved successfully.",
        "settings.workspace.feedback.save_failed" => "Failed to save the workspace. Check logs for details.",
        "settings.workspace.feedback.opened" => "Opened a new instance for {}.",
        "settings.workspace.feedback.open_failed" => "Could not open a new instance for this workspace.",
        "settings.workspace.feedback.duplicate_requires_name" => "Duplicate requires a non-default workspace name.",
        "settings.workspace.feedback.duplicate_failed" => "Failed to duplicate the workspace. Check logs for details.",
        "settings.workspace.feedback.duplicated" => "Workspace duplicated into {}.",
        "settings.workspace.feedback.default_rename" => "The default workspace cannot be renamed.",
        "settings.workspace.feedback.rename_requires_name" => "Rename requires a non-default workspace name.",
        "settings.workspace.feedback.same_name" => "Source and target workspace names are identical.",
        "settings.workspace.feedback.rename_title" => "Rename workspace",
        "settings.workspace.feedback.rename_confirm" => "Rename workspace {}?",
        "settings.workspace.feedback.rename_cancelled" => "Rename cancelled.",
        "settings.workspace.feedback.rename_failed" => "Failed to rename the workspace. Check logs for details.",
        "settings.workspace.feedback.renamed" => "Workspace renamed to {}.",
        "settings.workspace.feedback.default_delete" => "The default workspace cannot be deleted.",
        "settings.workspace.feedback.delete_title" => "Delete workspace",
        "settings.workspace.feedback.delete_confirm" => "Delete workspace '{}'? This action cannot be undone.",
        "settings.workspace.feedback.delete_cancelled" => "Delete cancelled.",
        "settings.workspace.feedback.delete_failed" => "Failed to delete the workspace. Check logs for details.",
        "settings.workspace.feedback.deleted" => "Workspace '{}' deleted.",
        "settings.workspace.feedback.loaded" => "Loaded {} in this instance.",
        "settings.workspace.feedback.load_failed" => "Failed to load the selected workspace.",
        "settings.shortcut.search_placeholder" => "Search shortcuts…",
        "settings.shortcut.alt_toolbar.compact" => "Use Alt as quick status bar toggle",
        "settings.shortcut.record" => "Record shortcut",
        "settings.shortcut.stop_recording" => "Stop recording",
        "settings.shortcut.recording_title" => "Recording shortcut",
        "settings.shortcut.recording_idle_title" => "Shortcut recording",
        "settings.shortcut.feedback.select_target" => "Click a Rec button beside a shortcut field to start recording.",
        "settings.shortcut.feedback.global_manual" => "Global activate uses modifier chords (Ctrl/Alt/Shift). Enter that one manually.",
        "settings.shortcut.feedback.press_key" => "Press a key for '{}'. Press Esc to cancel.",
        "settings.shortcut.feedback.stopped" => "Shortcut recording stopped.",
        "settings.shortcut.feedback.cancelled" => "Shortcut recording cancelled.",
        "settings.shortcut.feedback.unsupported" => "Unsupported key. Try letters, digits, Tab, Enter, Space, or Esc.",
        "settings.shortcut.feedback.no_target" => "No shortcut target selected. Click a Rec button first.",
        "settings.shortcut.feedback.unknown_target" => "Unknown shortcut target. Choose a field and try again.",
        "settings.shortcut.feedback.recorded" => "Recorded {}.",
        "settings.shortcut.target.layout_column" => "Layout column",
        "settings.shortcut.target.reset_layout" => "Reset layout",
        "settings.shortcut.target.cycle_layout" => "Cycle layout",
        "settings.shortcut.target.toggle_toolbar" => "Toggle toolbar",
        "settings.shortcut.target.toggle_animations" => "Toggle animations",
        "settings.shortcut.target.toggle_window_info" => "Toggle window info",
        "settings.shortcut.target.open_settings" => "Open settings",
        "settings.shortcut.target.open_menu" => "Open menu",
        "settings.shortcut.target.open_command_palette" => "Open command palette",
        "settings.shortcut.target.refresh_now" => "Refresh now",
        "settings.shortcut.target.exit_app" => "Exit app",
        "settings.shortcut.target.toggle_always_on_top" => "Always on top",
        "settings.shortcut.target.global_activate" => "Global activate",
        "settings.shortcut.target.fallback" => "Shortcut",
        "settings.shortcut.section.global" => "GLOBAL SHORTCUTS",
        "settings.shortcut.section.layout_selection" => "LAYOUT SELECTION",
        "settings.shortcut.section.layout_selection_helper" => "Quickly switch between layouts.",
        "settings.shortcut.section.layout_controls" => "LAYOUT CONTROLS",
        "settings.shortcut.section.layout_controls_helper" => {
            "Additional layout actions and adjustments."
        }
        "settings.shortcut.section.dashboard_actions" => "DASHBOARD ACTIONS",
        "settings.shortcut.section.dashboard_actions_helper" => "Control the dashboard experience.",
        "settings.shortcut.section.ui_toggles" => "UI TOGGLES",
        "settings.shortcut.section.navigation" => "NAVIGATION",
        "settings.shortcut.section.system" => "SYSTEM",
        "settings.layout_presets.title" => "Layout presets",
        "settings.layout_presets.helper" => {
            "Save, apply, or delete named ratio snapshots for the current layout."
        }
        "settings.layout_presets.name_placeholder" => "Focus grid",
        "settings.layout_presets.save_current" => "Save current",
        "settings.layout_presets.saved_title" => "Saved presets",
        "settings.layout_presets.saved_description" => {
            "Apply one preset to restore its layout ratios."
        }
        "settings.layout_presets.apply" => "Apply preset",
        "settings.layout_presets.delete" => "Delete preset",
        "settings.layout_presets.status" => "Preset status",
        "settings.layout_presets.feedback.select_or_save" => "Select a preset to apply or delete, or save current ratios as a new snapshot.",
        "settings.layout_presets.feedback.empty" => "No layout presets saved yet. Save current layout ratios to create one.",
        "settings.layout_presets.feedback.enter_name" => "Enter a preset name before saving.",
        "settings.layout_presets.feedback.save_persist_failed" => "Saved in memory, but failed to persist preset to disk.",
        "settings.layout_presets.feedback.saved" => "Saved layout preset '{}'.",
        "settings.layout_presets.feedback.select_apply" => "Select a preset to apply.",
        "settings.layout_presets.feedback.apply_missing" => "Could not apply the preset. It may have been renamed or deleted.",
        "settings.layout_presets.feedback.apply_persist_failed" => "Applied in memory, but failed to persist preset changes.",
        "settings.layout_presets.feedback.applied" => "Applied layout preset '{}'.",
        "settings.layout_presets.feedback.select_delete" => "Select a preset to delete.",
        "settings.layout_presets.feedback.deleted" => "Deleted layout preset '{}'.",
        "settings.layout_presets.feedback.delete_missing" => "Could not delete the preset. It may have already been removed.",
        "settings.option.dock_position.title" => "Dock position",
        "settings.option.dock_position.description" => {
            "Turn the window into a docked appbar or leave it floating as a free panel."
        }

        // ── Tag dialog ──
        "tag.title" => "Create custom tag",
        "tag.application" => "Application: ",
        "tag.name_label" => "Tag name",
        "tag.preset_colour" => "Preset colour",
        "tag.create_assign" => "Create and assign",

        // ── Theme ──
        "theme.classic_name" => "Classic Panopticon",
        "theme.classic_subtitle" => {
            "Uses the current canvas colour as the base background."
        }

        // ── Actions and dialogs ──
        "action.restore_selected" => "Restore selected",
        "action.restore_all" => "Restore all",
        "action.browse_image" => "Browse image…",
        "action.clear_image" => "Clear image",
        "action.refresh_now" => "Refresh now",
        "action.check_updates" => "Check updates",
        "action.auto_apply" => "Changes apply automatically.",
        "action.about" => "About",
        "action.load_profile" => "Load profile",
        "action.reset_defaults" => "Reset defaults",
        "action.close" => "Close",
        "dialog.choose_background_image" => "Choose dashboard background image",
        "dialog.reset_defaults.title" => "Reset all settings?",
        "dialog.reset_defaults.description" => {
            "This replaces the current workspace settings with Panopticon defaults. This cannot be undone."
        }
        "dialog.reset_defaults.success_title" => "Settings reset",
        "dialog.reset_defaults.success" => "Default settings were restored and applied.",
        "dialog.reset_defaults.failed_title" => "Settings reset failed",
        "dialog.reset_defaults.failed" => {
            "Defaults were applied in memory, but Panopticon could not save them. Check the warning in Settings."
        }
        "dialog.kill_process.title" => "Kill process?",
        "dialog.kill_process.description" => {
            "This force-closes {} and may discard unsaved work. Continue?"
        }
        "dialog.kill_process.success_title" => "Process terminated",
        "dialog.kill_process.success" => "The selected application process was terminated.",
        "dialog.kill_process.failed_title" => "Could not terminate process",
        "dialog.kill_process.failed" => {
            "Panopticon could not terminate the selected process. It may have closed already or require elevated permission."
        }

        // ── Command palette ──
        "command_palette.title" => "Command Palette",
        "command_palette.helper" => "Search and run layout, settings, and system commands.",
        "command_palette.search_placeholder" => "Type a command…",
        "command_palette.commands_title" => "Commands",
        "command_palette.commands_helper" => "Use ↑/↓ to select, Enter to run, and Esc to close.",
        "command_palette.run" => "Run",
        "command_palette.no_available" => "No commands available",
        "command_palette.no_results" => "No commands found",
        "command.category.layout" => "Layout",
        "command.category.theme" => "Theme",
        "command.category.system" => "System",
        "command.category.settings" => "Settings",
        "command.category.filters" => "Filters",
        "command.category.windows" => "Windows",
        "command.category.workspace" => "Workspace",
        "command.layout_cycle" => "Layout: Cycle",
        "command.layout_grid" => "Layout: Grid",
        "command.layout_mosaic" => "Layout: Mosaic",
        "command.layout_bento" => "Layout: Bento",
        "command.layout_fibonacci" => "Layout: Fibonacci",
        "command.layout_columns" => "Layout: Columns",
        "command.layout_row" => "Layout: Row",
        "command.layout_column" => "Layout: Column",
        "command.layout_reset_ratios" => "Layout: Reset ratios",
        "command.theme_cycle" => "Theme: Cycle",
        "command.refresh_now" => "Refresh: Run now",
        "command.restore_all_hidden" => "Windows: Restore all hidden apps",
        "command.open_settings" => "Open Settings",
        "command.settings_behavior" => "Settings: Behavior & Display",
        "command.settings_filters" => "Settings: Filters",
        "command.settings_workspaces" => "Settings: Workspaces",
        "command.settings_shortcuts" => "Settings: Shortcuts",
        "command.settings_advanced" => "Settings: Advanced",
        "command.open_about" => "Open About",
        "command.open_menu" => "Open App Menu",
        "command.clear_all_filters" => "Filters: Clear all",
        "command.clear_monitor_filter" => "Filters: Clear monitor",
        "command.clear_tag_filter" => "Filters: Clear tag",
        "command.clear_app_filter" => "Filters: Clear app",
        "command.toggle_animations" => "Toggle Animations",
        "command.toggle_toolbar" => "Toggle Status Bar",
        "command.toggle_window_info" => "Toggle Window Info",
        "command.toggle_always_on_top" => "Toggle Always On Top",
        "command.workspace_load_default" => "Workspace: Load default",
        "command.workspace_open_default" => "Workspace: Open default in new instance",
        "command.workspace_load" => "Workspace: Load {}",
        "command.workspace_open" => "Workspace: Open {} in new instance",
        "command.filter_monitor" => "Filters: Monitor {}",
        "command.filter_tag" => "Filters: Tag {}",
        "command.hide_app" => "Windows: Hide {}",
        "command.filter_app" => "Filters: App {}",
        "command.restore_hidden_app" => "Windows: Restore hidden {}",
        "command.exit" => "Exit Panopticon",

        // ── About window ──
        "about.title" => "About Panopticon",
        "about.subtitle" => {
            "A native Windows dashboard for exploring open windows through live DWM thumbnails."
        }
        "about.version_title" => "Version",
        "about.update_available" => "Update available",
        "about.description_title" => "Application",
        "about.description_body" => {
            "Panopticon helps you preview, organize, and activate live desktop windows from a single local control room."
        }
        "about.credits_title" => "Credits",
        "about.credits_body" => {
            "Created by gvastethecreator.\nBuilt with Rust, Slint, windows-rs, and the Desktop Window Manager thumbnail APIs.\nUI icons by HugeIcons.\nLicense: MIT."
        }

        // ── Validation / CLI ──
        "settings.profile_invalid_chars" => {
            "Profile name contains invalid Windows filename characters: {}"
        }
        "settings.workspace_invalid_chars" => {
            "Workspace name contains invalid Windows filename characters: {}"
        }
        "settings.profile_empty_name" => "Profile name cannot be empty",
        "settings.workspace_empty_name" => "Workspace name cannot be empty",
        "cli.usage_heading" => "Usage:",
        "cli.options_heading" => "Options:",
        "cli.profile_option_help" => {
            "Load or create the named profile from %APPDATA%\\Panopticon\\profiles\\<name>.toml"
        }
        "cli.workspace_option_help" => {
            "Load or create the named workspace from %APPDATA%\\Panopticon\\workspaces\\<name>.toml"
        }
        "cli.help_option_help" => "Show this help text",
        "cli.help_option_version" => "Show the current Panopticon version",
        "cli.missing_profile_value" => "Missing value for --profile",
        "cli.missing_workspace_value" => "Missing value for --workspace",
        "cli.unknown_argument" => "Unknown argument: {}",

        // ── Fallback ──
        other => {
            tracing::warn!(key = other, "missing i18n key");
            "[?]"
        }
    }
}

// ── Spanish translations ─────────────────────────────────────

#[allow(
    clippy::too_many_lines,
    clippy::match_same_arms,
    reason = "translation catalogs intentionally reuse the same copy for multiple keys"
)]
fn es(key: &str) -> Option<&'static str> {
    Some(match key {
        // ── App identity ──
        "app.name" => "Panopticon",
        "window.main_title" => "Panopticon",
        "window.settings_title" => "Panopticon — Configuración",
        "window.tag_title" => "Panopticon — Crear etiqueta",
        "window.about_title" => "Panopticon — Acerca de",
        "window.command_palette_title" => "Panopticon — Paleta de comandos",

        // ── Locales ──
        "locale.english" => "Inglés",
        "locale.spanish" => "Español",

        // ── Layout labels ──
        "layout.grid" => "Grid",
        "layout.mosaic" => "Mosaic",
        "layout.bento" => "Bento",
        "layout.fibonacci" => "Fibonacci",
        "layout.columns" => "Columnas",
        "layout.row" => "Fila",
        "layout.column" => "Columna",

        // ── Window context menu ──
        "menu.hide_from_layout" => "Ocultar del layout",
        "menu.pin_position" => "Fijar app en esta ubicación",
        "menu.preserve_aspect" => "Respetar relación de aspecto",
        "menu.hide_on_select" => "Ocultar Panopticon al abrir esta app",
        "menu.create_tag" => "Crear etiqueta personalizada…",
        "menu.thumbnail_refresh" => "Modo de refresco del thumbnail",
        "menu.thumbnail_refresh_realtime" => "Tiempo real",
        "menu.thumbnail_refresh_frozen" => "Congelado",
        "menu.thumbnail_refresh_interval" => "Intervalo",
        "menu.cell_color" => "Color de la celda",
        "menu.use_theme_color" => "Usar color del tema",
        "menu.close_window" => "Cerrar ventana",
        "menu.kill_process" => "Matar proceso",

        // ── Colour presets ──
        "color.amber" => "Usar ámbar",
        "color.sky" => "Usar cielo",
        "color.mint" => "Usar menta",
        "color.rose" => "Usar rosa",
        "color.violet" => "Usar violeta",
        "color.sun" => "Usar sol",
        "tag.color.amber" => "Ámbar",
        "tag.color.sky" => "Cielo",
        "tag.color.mint" => "Menta",
        "tag.color.rose" => "Rosa",
        "tag.color.violet" => "Violeta",
        "tag.color.sun" => "Sol",

        // ── Tray tooltip ──
        "tray.tooltip" => "Panopticon — Vista en vivo de ventanas",

        // ── Tray menu ──
        "tray.visibility" => "Visibilidad",
        "tray.show" => "Mostrar Panopticon",
        "tray.hide" => "Ocultar al tray",
        "tray.refresh" => "Refrescar ventanas",
        "tray.open_settings" => "Abrir configuración",
        "tray.open_about" => "Acerca de Panopticon",
        "tray.profiles" => "Workspaces",
        "tray.profile_default" => "default",
        "tray.workspaces" => "Workspaces",
        "tray.workspace_default" => "default",
        "tray.layout" => "Layout",
        "tray.next_layout" => "Siguiente layout",
        "tray.lock_layout" => "Bloquear cambio de layout",
        "tray.lock_resize" => "Bloquear redimensionado de celdas",
        "tray.dock_position" => "Posición de dock",
        "tray.group_by" => "Agrupar ventanas por",
        "tray.display" => "Pantalla",
        "tray.show_toolbar" => "Mostrar barra de estado",
        "tray.show_info" => "Mostrar info de ventanas",
        "tray.show_icons" => "Mostrar iconos en celdas",
        "tray.always_on_top" => "Mantener Panopticon encima",
        "tray.behaviour" => "Comportamiento",
        "tray.minimize_to_tray" => "Ocultar al minimizar",
        "tray.close_to_tray" => "Ocultar al cerrar",
        "tray.cycle_refresh" => "Ciclar intervalo de refresco ({})",
        "tray.animate" => "Animar transiciones",
        "tray.default_aspect" => "Default: preservar relación de aspecto",
        "tray.default_hide" => "Default: ocultar al activar",
        "tray.start_tray" => "Iniciar oculto en tray",
        "tray.filters" => "Filtros",
        "tray.filter_monitor" => "Filtrar por monitor",
        "tray.all_monitors" => "Todos los monitores",
        "tray.filter_tag" => "Filtrar por etiqueta",
        "tray.all_tags" => "Todas las etiquetas",
        "tray.filter_app" => "Filtrar por aplicación",
        "tray.all_apps" => "Todas las aplicaciones",
        "tray.restore_hidden" => "Restaurar apps ocultas",
        "tray.restore_all" => "Restaurar todas las apps ocultas",
        "tray.exit" => "Salir",

        // ── Dock submenu ──
        "dock.none" => "Flotante (sin dock)",
        "dock.left" => "Izquierda",
        "dock.right" => "Derecha",
        "dock.top" => "Arriba",
        "dock.bottom" => "Abajo",

        // ── Grouping submenu ──
        "group.none" => "Sin agrupación",
        "group.application" => "Aplicación",
        "group.monitor" => "Monitor",
        "group.title" => "Título de ventana",
        "group.class" => "Clase de ventana",
        "filter.grouped_by" => "agrupado por:",

        // ── UI labels (Slint) ──
        "ui.minimized" => "minimizada",
        "ui.last_seen" => "ÚLTIMA VISTA",
        "ui.visible" => "visibles",
        "ui.hidden" => "ocultas",
        "ui.always_on_top" => "siempre visible",
        "ui.normal_window" => "ventana normal",
        "ui.toolbar_hint" => "click der. barra de estado / M: menú  ·  Esc salir",
        "ui.anim_on" => "anim on",
        "ui.anim_off" => "anim off",

        // ── Empty state ──
        "ui.empty_message" => "No hay ventanas disponibles",
        "ui.empty_helper" => {
            "Abrí o restaurá cualquier ventana del escritorio.\nPanopticon seguirá vigilando desde el tray."
        }

        // ── Settings ──
        "settings.hidden_app_fallback" => "App oculta",
        "settings.dock_hint" => "En modo dock esta opción queda desactivada automáticamente.",
        "settings.filters_hint" => {
            "Los filtros y el agrupado también se reflejan en la barra de estado y reordenan las celdas visibles."
        }
        "settings.no_saved_profiles" => "Sin workspaces guardados",
        "settings.default_profile" => "default",
        "settings.saved_profiles" => "Workspaces guardados: default",
        "settings.saved_profiles_fmt" => "Workspaces guardados: {}",
        "settings.current_profile" => "Workspace actual: ",
        "settings.profile_label" => "Workspace:",
        "settings.save_profile" => "Guardar workspace",
        "settings.open_instance" => "Abrir otra instancia",
        "settings.no_hidden_hint" => "No hay apps ocultas para restaurar ahora mismo.",
        "settings.no_hidden" => "No hay apps ocultas",
        "settings.hidden_one" => "1 app oculta lista para restaurar",
        "settings.hidden_many" => "{} apps ocultas listas para restaurar",
        "settings.title" => "Configuración",
        "settings.subtitle" => {
            "Personaliza el dashboard, los fondos, los atajos y el comportamiento general."
        }
        "settings.profile_badge" => "Perfil",
        "settings.nav.behaviour_display.title" => "Comportamiento y vista",
        "settings.nav.behaviour_display.subtitle" => {
            "Comportamiento de ventana, tray y chrome visible"
        }
        "settings.nav.filters.title" => "Filtros",
        "settings.nav.filters.subtitle" => "Herramientas por monitor, tag, app y estado oculto",
        "settings.nav.theme_background.title" => "Tema y fondo",
        "settings.nav.theme_background.subtitle" => {
            "Presets de tema, color sólido de canvas e imagen"
        }
        "settings.nav.profiles.title" => "Perfiles (Workspaces)",
        "settings.nav.profiles.subtitle" => "Guardá y abrí configuraciones con nombre",
        "settings.nav.shortcuts.title" => "Atajos de teclado",
        "settings.nav.shortcuts.subtitle" => "Personalizá el mapa de teclas del dashboard",
        "settings.nav.advanced.title" => "Opciones avanzadas",
        "settings.nav.advanced.subtitle" => {
            "Cadencia de refresco y herramientas manuales de runtime"
        }
        "settings.page.behaviour_display.title" => "Comportamiento y vista",
        "settings.page.behaviour_display.subtitle" => {
            "Ajustá cómo se comporta la ventana principal, qué información se muestra y cómo responde al tray."
        }
        "settings.section.behaviour.title" => "Comportamiento",
        "settings.section.behaviour.helper" => {
            "Cada opción añade contexto con un pequeño resumen para no tener que adivinar qué hace."
        }
        "settings.option.language.title" => "Idioma",
        "settings.option.language.description" => {
            "Elegí el idioma de la aplicación. Inglés es el predeterminado y también está disponible español."
        }
        "settings.option.always_on_top.title" => "Siempre visible",
        "settings.option.always_on_top.description" => {
            "Mantiene Panopticon por encima de las demás ventanas incluso al cambiar de aplicación."
        }
        "settings.option.animate_transitions.title" => "Animar transiciones",
        "settings.option.animate_transitions.description" => {
            "Suaviza cambios de layout, filtros y reacomodos visuales entre miniaturas."
        }
        "settings.option.minimize_to_tray.title" => "Ocultar al minimizar",
        "settings.option.minimize_to_tray.description" => {
            "Al minimizar, la app desaparece del escritorio y sigue viva desde la bandeja del sistema."
        }
        "settings.option.close_to_tray.title" => "Ocultar al cerrar",
        "settings.option.close_to_tray.description" => {
            "Interpreta el cierre de ventana como ocultar a la bandeja en lugar de salir."
        }
        "settings.option.preserve_aspect_ratio.title" => "Preservar relación de aspecto por defecto",
        "settings.option.preserve_aspect_ratio.description" => {
            "Las nuevas apps respetarán mejor la proporción original de sus thumbnails."
        }
        "settings.option.hide_on_select.title" => "Ocultar al seleccionar una app",
        "settings.option.hide_on_select.description" => {
            "Oculta Panopticon cuando activás una ventana desde el dashboard."
        }
        "settings.option.start_in_tray.title" => "Iniciar oculto en tray",
        "settings.option.start_in_tray.description" => {
            "Inicia directamente en segundo plano para un arranque más silencioso."
        }
        "settings.option.run_at_startup.title" => "Ejecutar al iniciar",
        "settings.option.run_at_startup.description" => {
            "Registra Panopticon en la sesión actual de Windows para que se inicie automáticamente al entrar al sistema."
        }
        "settings.option.lock_layout.title" => "Bloquear cambios de layout",
        "settings.option.lock_layout.description" => {
            "Bloquea cambios de layout por teclado o desde el menú de la aplicación."
        }
        "settings.option.lock_cell_resize.title" => "Bloquear redimensionado de celdas",
        "settings.option.lock_cell_resize.description" => {
            "Desactiva el arrastre de separadores para proteger la composición actual."
        }
        "settings.section.display.title" => "Vista",
        "settings.section.display.helper" => {
            "Controles para layout inicial, posición del dock, tamaño y legibilidad del dashboard."
        }
        "settings.option.show_toolbar.title" => "Mostrar barra de estado",
        "settings.option.show_toolbar.description" => {
            "Muestra la barra de estado del dashboard con resumen y acceso rápido al menú."
        }
        "settings.toolbar_position.top" => "Arriba",
        "settings.toolbar_position.bottom" => "Abajo",
        "settings.option.toolbar_position.title" => "Posición de la barra de estado",
        "settings.option.toolbar_position.description" => {
            "Elegí si la barra de estado se mantiene arriba o abajo del dashboard."
        }
        "settings.option.show_info.title" => "Mostrar info de ventana sobre las miniaturas",
        "settings.option.show_info.description" => {
            "Añade el nombre de ventana y aplicación encima de cada preview para leer el contexto de un vistazo."
        }
        "settings.option.show_app_icons.title" => "Mostrar iconos de apps en celdas",
        "settings.option.show_app_icons.description" => {
            "Pinta el icono del proceso dentro de cada celda para identificar apps más rápido."
        }
        "settings.page.filters.title" => "Filtros",
        "settings.page.filters.subtitle" => {
            "Acotá el dashboard por monitor, tags, aplicaciones o grupos, y recuperá apps ocultas sin salir de esta vista."
        }
        "settings.option.monitor_filter.title" => "Filtro por monitor",
        "settings.option.monitor_filter.description" => {
            "Limita el dashboard a un monitor específico cuando trabajás con varias pantallas."
        }
        "settings.option.tag_filter.title" => "Filtro por etiqueta",
        "settings.option.tag_filter.description" => {
            "Muestra solo aplicaciones asociadas a una etiqueta manual concreta."
        }
        "settings.option.app_filter.title" => "Filtro por aplicación",
        "settings.option.app_filter.description" => {
            "Aíslá una app concreta cuando querés revisar solo su grupo de ventanas."
        }
        "settings.option.group_windows.title" => "Agrupar ventanas por",
        "settings.option.group_windows.description" => {
            "Reordena visualmente la lista sin filtrar contenido, ideal para encontrar patrones."
        }
        "settings.section.hidden_apps.title" => "Aplicaciones ocultas",
        "settings.section.hidden_apps.helper" => {
            "Recuperá apps ocultas una a una o de forma masiva desde el estado persistido."
        }
        "settings.section.app_rules.title" => "Gestor de reglas por app",
        "settings.section.app_rules.helper" => {
            "Edita reglas por app para visibilidad, aspecto, ocultar al seleccionar, modo de refresco, tags y color."
        }
        "settings.option.app_rules.app.title" => "Aplicación",
        "settings.option.app_rules.app.description" => {
            "Incluye apps en ejecución y apps con reglas guardadas."
        }
        "settings.option.app_rules.search.placeholder" => "Buscar por app, id o tags...",
        "settings.app_rules.filter.all" => "Todas las apps",
        "settings.app_rules.filter.running" => "Apps en ejecución",
        "settings.app_rules.filter.saved" => "Sólo reglas guardadas",
        "settings.app_rules.filter.hidden" => "Apps ocultas",
        "settings.app_rules.filter.tagged" => "Apps etiquetadas",
        "settings.app_rules.filter.refresh" => "Refresco personalizado",
        "settings.app_rules.filter.pinned" => "Apps fijadas",
        "settings.app_rules.active.title" => "Regla activa",
        "settings.app_rules.active.badge" => "REGLA APP",
        "settings.app_rules.hidden.title" => "Ocultar app en dashboard",
        "settings.app_rules.hidden.description" => {
            "Si está activo, la app no aparece en la grilla principal."
        }
        "settings.app_rules.preserve_aspect.title" => "Preservar aspect ratio",
        "settings.app_rules.preserve_aspect.description" => {
            "Override por app del ajuste global de aspecto."
        }
        "settings.app_rules.hide_on_select.title" => "Ocultar Panopticon al seleccionar",
        "settings.app_rules.hide_on_select.description" => {
            "Override por app del comportamiento hide-on-select."
        }
        "settings.app_rules.refresh_mode.title" => "Modo de refresh del thumbnail",
        "settings.app_rules.refresh_mode.description" => "Realtime, Frozen o Interval.",
        "settings.app_rules.refresh_mode.realtime" => "Realtime",
        "settings.app_rules.refresh_mode.frozen" => "Frozen",
        "settings.app_rules.refresh_mode.interval" => "Interval",
        "settings.app_rules.interval.title" => "Intervalo (ms)",
        "settings.app_rules.interval.description" => {
            "Se aplica cuando el modo es Interval."
        }
        "settings.app_rules.tags.title" => "Tags (CSV)",
        "settings.app_rules.tags.description" => "Ejemplo: work, browser, stream",
        "settings.app_rules.tags.placeholder" => "work, browser",
        "settings.app_rules.tags.helper" => "Añade tags una por una y elimínalas con un click.",
        "settings.app_rules.tags.add" => "Añadir tag",
        "settings.app_rules.tags.remove_hint" => "Haz click en un chip para quitarlo.",
        "settings.app_rules.tags.suggestions_title" => "Sugerencias rápidas",
        "settings.app_rules.tags.suggestion.work" => "work",
        "settings.app_rules.tags.suggestion.browser" => "browser",
        "settings.app_rules.tags.suggestion.dev" => "dev",
        "settings.app_rules.tags.suggestion.stream" => "stream",
        "settings.app_rules.color.title" => "Color hex (opcional)",
        "settings.app_rules.color.description" => {
            "Formato RRGGBB o vacío para limpiar override."
        }
        "settings.app_rules.color.placeholder" => "5CA9FF",
        "settings.app_rules.apply" => "Aplicar regla",
        "settings.app_rules.reset" => "Resetear regla",
        "settings.app_rules.clear_unused" => "Limpiar reglas inactivas",
        "settings.app_rules.no_selection" => "Selecciona una aplicación para editar su regla.",
        "settings.app_rules.select_option" => "Seleccionar regla de aplicación…",
        "settings.app_rules.cleanup.none" => "No hay reglas inactivas para limpiar.",
        "settings.app_rules.cleanup.count" => "{} reglas inactivas se pueden limpiar.",
        "settings.page.theme_background.title" => "Tema y fondo",
        "settings.page.theme_background.subtitle" => {
            "Elegí un preset de tema, ajustá sus colores principales y combínalo con un color propio de canvas y una imagen de fondo."
        }
        "settings.section.theme_grid.title" => "Presets de tema",
        "settings.section.theme_grid.helper" => {
            "Seleccioná un preset desde una lista compacta; el tema define acentos, paneles y contraste general."
        }
        "settings.section.theme_colours.title" => "Colores del tema",
        "settings.section.theme_colours.helper" => {
            "Ajustá los colores principales del tema activo. Dejá un campo vacío para conservar el valor del preset."
        }
        "settings.section.canvas_background.title" => "Fondo del canvas",
        "settings.section.canvas_background.helper" => {
            "El color de canvas vive detrás de las cards y de la imagen opcional de fondo."
        }
        "settings.option.custom_canvas_colour.title" => "Color personalizado del canvas",
        "settings.option.custom_canvas_colour.description" => {
            "Introduce un RGB hex manual si querés un color fuera de la paleta rápida."
        }
        "settings.section.preview.title" => "Vista previa",
        "settings.section.preview.helper" => {
            "Resumen rápido del fondo activo con color e imagen opcional."
        }
        "settings.section.background_image.title" => "Imagen de fondo",
        "settings.section.background_image.helper" => {
            "Usá una imagen detrás del dashboard y definí cómo debe ajustarse dentro del canvas."
        }
        "settings.option.image_file.title" => "Archivo de imagen",
        "settings.option.image_file.description" => {
            "Podés pegar una ruta manualmente o elegirla con el selector nativo."
        }
        "settings.option.image_fit.title" => "Ajuste de imagen",
        "settings.option.image_fit.description" => {
            "Controla si la imagen cubre, contiene o rellena el área visible del dashboard."
        }
        "settings.option.image_opacity.title" => "Opacidad de la imagen",
        "settings.option.image_opacity.description" => {
            "Controla qué tan presente debe verse la imagen de fondo sobre el color del canvas."
        }
        "settings.theme_colours.accent" => "Acento",
        "settings.theme_colours.surface" => "Superficie",
        "settings.theme_colours.card" => "Tarjeta",
        "settings.theme_colours.text" => "Texto",
        "settings.theme_colours.muted" => "Atenuado",
        "settings.theme_colours.border" => "Borde",
        "settings.theme_colours.hint" => "Dejá el campo vacío para heredar el color actual del preset.",
        "settings.fit.cover" => "Cubrir",
        "settings.fit.contain" => "Contener",
        "settings.fit.fill" => "Rellenar",
        "settings.fit.preserve" => "Preservar",
        "settings.page.profiles.title" => "Workspaces",
        "settings.page.profiles.subtitle" => {
            "Guardá combinaciones completas de ajustes y abrí nuevas instancias ya apuntando al workspace que quieras."
        }
        "settings.section.edit_profile.title" => "Editar workspace",
        "settings.section.edit_profile.helper" => {
            "Usá un nombre corto y descriptivo para guardar o abrir el snapshot actual en otra instancia."
        }
        "settings.current_profile_card.title" => "Workspace actual",
        "settings.option.profile_name.title" => "Nombre del workspace",
        "settings.option.profile_name.description" => {
            "Nombre usado para guardar esta configuración o lanzar otra instancia con ella."
        }
        "settings.section.saved_profiles.title" => "Workspaces guardados",
        "settings.section.saved_profiles.helper" => {
            "Resumen de workspaces detectados y recordatorio del flujo recomendado para trabajar con varias instancias."
        }
        "settings.section.load_profile.title" => "Cambiar esta instancia",
        "settings.section.load_profile.helper" => {
            "Carga otro workspace guardado dentro de esta ventana de Panopticon ya abierta."
        }
        "settings.option.available_profile.title" => "Workspaces disponibles",
        "settings.option.available_profile.description" => {
            "Elegí qué workspace guardado debe cargar esta instancia ahora mismo."
        }
        "settings.tips.title" => "Consejos",
        "settings.tips.body" => {
            "- Guarda primero el workspace actual si vas a abrir otra instancia.\n- Usa nombres simples como work, stream o review.\n- Los ajustes de tema, fondo y shortcuts viajan con el workspace."
        }
        "settings.page.shortcuts.title" => "Atajos de teclado",
        "settings.page.shortcuts.subtitle" => {
            "Los atajos del dashboard usan una sola tecla; la activación global acepta Ctrl / Alt / Shift más una tecla como P o Space."
        }
        "settings.section.layout_bindings.title" => "Atajos de layout",
        "settings.section.layout_bindings.helper" => {
            "Asignaciones directas para layouts concretos, reset y ciclo general."
        }
        "settings.shortcut.layout_grid.title" => "Layout Grid",
        "settings.shortcut.layout_grid.description" => "Activa la vista Grid al instante.",
        "settings.shortcut.layout_mosaic.title" => "Layout Mosaic",
        "settings.shortcut.layout_mosaic.description" => "Cambia a la distribución Mosaic.",
        "settings.shortcut.layout_bento.title" => "Layout Bento",
        "settings.shortcut.layout_bento.description" => "Activa el layout Bento.",
        "settings.shortcut.layout_fibonacci.title" => "Layout Fibonacci",
        "settings.shortcut.layout_fibonacci.description" => "Abre la composición Fibonacci.",
        "settings.shortcut.layout_columns.title" => "Layout Columnas",
        "settings.shortcut.layout_columns.description" => "Activa Columnas.",
        "settings.shortcut.layout_row.title" => "Layout Fila",
        "settings.shortcut.layout_row.description" => "Activa la vista Fila.",
        "settings.shortcut.layout_column.title" => "Layout Columna",
        "settings.shortcut.layout_column.description" => "Activa la vista Columna.",
        "settings.shortcut.reset_layout.title" => "Resetear proporciones del layout",
        "settings.shortcut.reset_layout.description" => {
            "Restablece proporciones personalizadas del layout actual."
        }
        "settings.shortcut.cycle_layout.title" => "Ciclar layout",
        "settings.shortcut.cycle_layout.description" => {
            "Avanza al siguiente layout en la secuencia interna."
        }
        "settings.section.dashboard_actions.title" => "Acciones del dashboard",
        "settings.section.dashboard_actions.helper" => {
            "Atajos para abrir paneles, refrescar estado y alternar opciones visibles."
        }
        "settings.shortcut.cycle_theme.title" => "Ciclar tema",
        "settings.shortcut.cycle_theme.description" => {
            "Pulsa T para el siguiente tema o Shift+T para volver al anterior."
        }
        "settings.shortcut.toggle_animations.title" => "Alternar animaciones",
        "settings.shortcut.toggle_animations.description" => {
            "Activa o desactiva transiciones del dashboard."
        }
        "settings.shortcut.toggle_toolbar.title" => "Alternar barra de estado",
        "settings.shortcut.toggle_toolbar.description" => {
            "Muestra u oculta la barra de estado inferior del dashboard."
        }
        "settings.shortcut.toggle_window_info.title" => "Alternar info de ventanas",
        "settings.shortcut.toggle_window_info.description" => {
            "Muestra u oculta títulos e info encima de las miniaturas."
        }
        "settings.shortcut.toggle_always_on_top.title" => "Alternar siempre visible",
        "settings.shortcut.toggle_always_on_top.description" => {
            "Conmuta el modo siempre visible por encima de otras apps."
        }
        "settings.shortcut.open_settings.title" => "Abrir configuración",
        "settings.shortcut.open_settings.description" => {
            "Abre esta ventana desde el dashboard principal."
        }
        "settings.shortcut.open_menu.title" => "Abrir menú de la aplicación",
        "settings.shortcut.open_menu.description" => {
            "Abre el menú nativo principal con acciones rápidas."
        }
        "settings.shortcut.open_command_palette.title" => "Abrir paleta de comandos",
        "settings.shortcut.open_command_palette.description" => {
            "Abre el panel de búsqueda rápida de comandos."
        }
        "settings.shortcut.global_activate.title" => "Activar Panopticon globalmente",
        "settings.shortcut.global_activate.description" => {
            "Trae Panopticon al frente desde cualquier lugar. Déjalo vacío para desactivarlo."
        }
        "settings.shortcut.refresh_now.title" => "Refrescar ahora",
        "settings.shortcut.refresh_now.description" => {
            "Fuerza una nueva enumeración de ventanas y refresca el dashboard."
        }
        "settings.shortcut.exit_app.title" => "Salir de la app",
        "settings.shortcut.exit_app.description" => "Cierra Panopticon desde teclado.",
        "settings.shortcut.alt_toolbar.title" => "Usar Alt como atajo rápido para la barra de estado",
        "settings.shortcut.alt_toolbar.description" => {
            "Mantiene el atajo legacy de Windows para esconder o mostrar la barra de estado con una sola pulsación de Alt."
        }
        "settings.page.advanced.title" => "Opciones avanzadas",
        "settings.page.advanced.subtitle" => {
            "Controles de refresco/actualización manual y la cadencia base usada para descubrir ventanas en segundo plano."
        }
        "settings.option.default_layout.title" => "Layout por defecto",
        "settings.option.default_layout.description" => {
            "Define con qué layout debe arrancar Panopticon cada vez que abras este perfil."
        }
        "settings.option.default_layout.docked_description" => {
            "El modo dock usa automáticamente Column en los laterales y Row en arriba o abajo, así que este selector sólo aplica en modo flotante."
        }
        "settings.option.refresh_interval.title" => "Intervalo de refresco",
        "settings.option.refresh_interval.description" => {
            "Cadencia base para enumerar ventanas y actualizar el dashboard cuando no fuerzas un refresh manual."
        }
        "settings.refresh_mode.realtime" => "Tiempo real",
        "settings.refresh_mode.balanced" => "Balanceado",
        "settings.refresh_mode.battery_saver" => "Ahorro de batería",
        "settings.refresh_mode.manual" => "Manual",
        "settings.section.refresh_performance.title" => "Modo de rendimiento de refresco",
        "settings.section.refresh_performance.helper" => {
            "Realtime/Balanceado/Ahorro de batería fijan la cadencia automáticamente. Manual usa el intervalo explícito de abajo."
        }
        "settings.option.refresh_performance_mode.title" => "Modo de rendimiento de refresco",
        "settings.option.refresh_performance_mode.description" => {
            "Perfil global de cadencia para descubrimiento y refresco de ventanas."
        }
        "settings.refresh_mode.active.title" => "Modo activo",
        "settings.refresh_mode.active.realtime" => "Tiempo real (1s)",
        "settings.refresh_mode.active.balanced" => "Balanceado (2s)",
        "settings.refresh_mode.active.battery" => "Ahorro de batería (5s)",
        "settings.refresh_mode.active.manual" => "Manual (usa intervalo explícito)",
        "settings.section.manual_refresh.title" => "Refresco manual",
        "settings.section.manual_refresh.helper" => {
            "Usa esta sección para forzar un refresco de ventanas o comprobar actualizaciones al instante."
        }
        "settings.section.dock_thickness.title" => "Grosor del dock",
        "settings.section.dock_thickness.helper" => {
            "Para dock lateral se usa width; para top/bottom se usa height. Los valores se ajustan a mínimos seguros."
        }
        "settings.section.floating_window_size.title" => "Tamaño de ventana flotante",
        "settings.section.floating_window_size.helper" => {
            "Cuando el dock está desactivado, estos valores siguen el tamaño de la ventana flotante y se ajustan a mínimos seguros."
        }
        "settings.option.thumbnail_render_scale.title" => "Escala de render de thumbnails",
        "settings.option.thumbnail_render_scale.description" => {
            "Elegí 25%, 50%, 75% o 100% de detalle para los thumbnails. Valores menores sacrifican nitidez para ganar rendimiento manteniendo casi intacta la huella visual de la tarjeta."
        }
        "settings.label.width" => "Ancho",
        "settings.label.height" => "Alto",
        "settings.version_label" => "Versión:",
        "settings.update_status.idle" => "Comprobación de actualizaciones pendiente",
        "settings.update_status.checking" => "Buscando actualizaciones…",
        "settings.update_status.up_to_date" => "Al día ({})",
        "settings.update_status.available" => "Nueva versión disponible ({})",
        "settings.update_status.failed" => "No se pudo comprobar actualizaciones",
        "settings.persistence_status.failed" => {
            "Los cambios están activos en esta sesión, pero no se pudieron guardar. Revisa los logs e inténtalo de nuevo."
        }
        "settings.persistence_status.failed_title" => "No se guardó la configuración",
        "settings.persistence.retry" => "Reintentar guardado",
        "settings.persistence.open_logs" => "Abrir logs",
        "settings.update.cancel" => "Cancelar comprobación",
        "settings.option.center_secondary.title" => "Centrar ventanas secundarias",
        "settings.option.center_secondary.description" => {
            "Abre Configuración, Acerca de, la Paleta de comandos y los diálogos de etiquetas centrados en el monitor."
        }
        "settings.app_rules.pinned_slot.title" => "Posición fijada",
        "settings.app_rules.pinned_slot.description" => {
            "0 = sin fijar. Los valores positivos reservan una posición preferida."
        }
        "settings.app_rules.pinned_slot.conflict" => "Conflicto de posición fijada",
        "settings.theme_catalog.title" => "Catálogo de temas",
        "settings.theme_catalog.description" => {
            "Selecciona cualquier tema instalado. Las tarjetas muestran una vista previa acotada."
        }
        "settings.workspace.display_name.title" => "Nombre visible",
        "settings.workspace.display_name.description" => "Nombre amigable mostrado en las listas de workspaces.",
        "settings.workspace.description.title" => "Descripción",
        "settings.workspace.description.description" => {
            "Nota opcional para recordar el propósito de este workspace."
        }
        "settings.workspace.name_placeholder" => "workspace-a",
        "settings.workspace.display_name_placeholder" => "Trabajo · Foco profundo",
        "settings.workspace.description_placeholder" => "Apps fijadas y filtros para sesiones de revisión de diseño",
        "settings.workspace.action.duplicate" => "Duplicar",
        "settings.workspace.action.rename" => "Renombrar",
        "settings.workspace.action.delete" => "Eliminar",
        "settings.workspace.action.failed" => "La acción del workspace falló",
        "settings.workspace.action.completed" => "Acción del workspace completada",
        "settings.workspace.active" => "ACTIVO",
        "settings.workspace.default" => "PREDETERMINADO",
        "settings.workspace.metadata.title" => "Datos del workspace seleccionado",
        "settings.workspace.name_label" => "Workspace",
        "settings.workspace.updated_label" => "Última actualización",
        "settings.workspace.created_label" => "Creado",
        "settings.workspace.runtime_status_label" => "Estado de ejecución",
        "settings.workspace.not_saved" => "Aún no guardado",
        "settings.workspace.unknown" => "Desconocido",
        "settings.workspace.no_runtime" => "Aún no hay diagnóstico de ejecución",
        "settings.workspace.running" => "EN EJECUCIÓN",
        "settings.workspace.modified" => "MODIFICADO",
        "settings.workspace.feedback.saved" => "Workspace {} guardado correctamente.",
        "settings.workspace.feedback.save_failed" => "No se pudo guardar el workspace. Revisa los logs.",
        "settings.workspace.feedback.opened" => "Se abrió una nueva instancia para {}.",
        "settings.workspace.feedback.open_failed" => "No se pudo abrir una nueva instancia para este workspace.",
        "settings.workspace.feedback.duplicate_requires_name" => "Duplicar requiere un nombre distinto de default.",
        "settings.workspace.feedback.duplicate_failed" => "No se pudo duplicar el workspace. Revisa los logs.",
        "settings.workspace.feedback.duplicated" => "Workspace duplicado como {}.",
        "settings.workspace.feedback.default_rename" => "El workspace default no se puede renombrar.",
        "settings.workspace.feedback.rename_requires_name" => "Renombrar requiere un nombre distinto de default.",
        "settings.workspace.feedback.same_name" => "Los nombres de origen y destino son iguales.",
        "settings.workspace.feedback.rename_title" => "Renombrar workspace",
        "settings.workspace.feedback.rename_confirm" => "¿Renombrar el workspace {}?",
        "settings.workspace.feedback.rename_cancelled" => "Cambio de nombre cancelado.",
        "settings.workspace.feedback.rename_failed" => "No se pudo renombrar el workspace. Revisa los logs.",
        "settings.workspace.feedback.renamed" => "Workspace renombrado como {}.",
        "settings.workspace.feedback.default_delete" => "El workspace default no se puede eliminar.",
        "settings.workspace.feedback.delete_title" => "Eliminar workspace",
        "settings.workspace.feedback.delete_confirm" => "¿Eliminar el workspace '{}'? Esta acción no se puede deshacer.",
        "settings.workspace.feedback.delete_cancelled" => "Eliminación cancelada.",
        "settings.workspace.feedback.delete_failed" => "No se pudo eliminar el workspace. Revisa los logs.",
        "settings.workspace.feedback.deleted" => "Workspace '{}' eliminado.",
        "settings.workspace.feedback.loaded" => "Se cargó {} en esta instancia.",
        "settings.workspace.feedback.load_failed" => "No se pudo cargar el workspace seleccionado.",
        "settings.shortcut.search_placeholder" => "Buscar atajos…",
        "settings.shortcut.alt_toolbar.compact" => "Usar Alt para alternar la barra de estado",
        "settings.shortcut.record" => "Grabar atajo",
        "settings.shortcut.stop_recording" => "Detener grabación",
        "settings.shortcut.recording_title" => "Grabando atajo",
        "settings.shortcut.recording_idle_title" => "Grabación de atajo",
        "settings.shortcut.feedback.select_target" => "Haz clic en Grabar junto a un atajo para iniciar la captura.",
        "settings.shortcut.feedback.global_manual" => "El atajo global usa modificadores (Ctrl/Alt/Shift). Introdúcelo manualmente.",
        "settings.shortcut.feedback.press_key" => "Pulsa una tecla para '{}'. Pulsa Esc para cancelar.",
        "settings.shortcut.feedback.stopped" => "Grabación de atajo detenida.",
        "settings.shortcut.feedback.cancelled" => "Grabación de atajo cancelada.",
        "settings.shortcut.feedback.unsupported" => "Tecla no compatible. Prueba letras, dígitos, Tab, Enter, Espacio o Esc.",
        "settings.shortcut.feedback.no_target" => "No hay un atajo seleccionado. Pulsa primero un botón Grabar.",
        "settings.shortcut.feedback.unknown_target" => "Atajo desconocido. Elige un campo y vuelve a intentarlo.",
        "settings.shortcut.feedback.recorded" => "Grabado: {}.",
        "settings.shortcut.target.layout_column" => "Layout columna",
        "settings.shortcut.target.reset_layout" => "Restablecer layout",
        "settings.shortcut.target.cycle_layout" => "Cambiar layout",
        "settings.shortcut.target.toggle_toolbar" => "Alternar barra de estado",
        "settings.shortcut.target.toggle_animations" => "Alternar animaciones",
        "settings.shortcut.target.toggle_window_info" => "Alternar info de ventana",
        "settings.shortcut.target.open_settings" => "Abrir configuración",
        "settings.shortcut.target.open_menu" => "Abrir menú",
        "settings.shortcut.target.open_command_palette" => "Abrir paleta de comandos",
        "settings.shortcut.target.refresh_now" => "Refrescar ahora",
        "settings.shortcut.target.exit_app" => "Salir de la aplicación",
        "settings.shortcut.target.toggle_always_on_top" => "Siempre visible",
        "settings.shortcut.target.global_activate" => "Activación global",
        "settings.shortcut.target.fallback" => "Atajo",
        "settings.shortcut.section.global" => "ATAJOS GLOBALES",
        "settings.shortcut.section.layout_selection" => "SELECCIÓN DE LAYOUT",
        "settings.shortcut.section.layout_selection_helper" => "Cambia rápidamente entre layouts.",
        "settings.shortcut.section.layout_controls" => "CONTROLES DE LAYOUT",
        "settings.shortcut.section.layout_controls_helper" => {
            "Acciones y ajustes adicionales del layout."
        }
        "settings.shortcut.section.dashboard_actions" => "ACCIONES DEL DASHBOARD",
        "settings.shortcut.section.dashboard_actions_helper" => "Controla la experiencia del dashboard.",
        "settings.shortcut.section.ui_toggles" => "CONTROLES DE INTERFAZ",
        "settings.shortcut.section.navigation" => "NAVEGACIÓN",
        "settings.shortcut.section.system" => "SISTEMA",
        "settings.layout_presets.title" => "Presets de layout",
        "settings.layout_presets.helper" => {
            "Guarda, aplica o elimina proporciones con nombre para el layout actual."
        }
        "settings.layout_presets.name_placeholder" => "Grid de foco",
        "settings.layout_presets.save_current" => "Guardar actual",
        "settings.layout_presets.saved_title" => "Presets guardados",
        "settings.layout_presets.saved_description" => {
            "Aplica un preset para restaurar sus proporciones de layout."
        }
        "settings.layout_presets.apply" => "Aplicar preset",
        "settings.layout_presets.delete" => "Eliminar preset",
        "settings.layout_presets.status" => "Estado del preset",
        "settings.layout_presets.feedback.select_or_save" => "Selecciona un preset para aplicarlo o eliminarlo, o guarda las proporciones actuales.",
        "settings.layout_presets.feedback.empty" => "Todavía no hay presets guardados. Guarda las proporciones actuales para crear uno.",
        "settings.layout_presets.feedback.enter_name" => "Escribe un nombre para el preset antes de guardarlo.",
        "settings.layout_presets.feedback.save_persist_failed" => "Se guardó en memoria, pero no se pudo persistir el preset en disco.",
        "settings.layout_presets.feedback.saved" => "Preset de layout '{}' guardado.",
        "settings.layout_presets.feedback.select_apply" => "Selecciona un preset para aplicarlo.",
        "settings.layout_presets.feedback.apply_missing" => "No se pudo aplicar el preset. Puede haberse renombrado o eliminado.",
        "settings.layout_presets.feedback.apply_persist_failed" => "Se aplicó en memoria, pero no se pudieron persistir los cambios.",
        "settings.layout_presets.feedback.applied" => "Preset de layout '{}' aplicado.",
        "settings.layout_presets.feedback.select_delete" => "Selecciona un preset para eliminarlo.",
        "settings.layout_presets.feedback.deleted" => "Preset de layout '{}' eliminado.",
        "settings.layout_presets.feedback.delete_missing" => "No se pudo eliminar el preset. Puede que ya no exista.",
        "settings.option.dock_position.title" => "Posición del dock",
        "settings.option.dock_position.description" => {
            "Convierte la ventana en appbar anclada o la deja flotando como panel libre."
        }

        // ── Tag dialog ──
        "tag.title" => "Crear etiqueta personalizada",
        "tag.application" => "Aplicación: ",
        "tag.name_label" => "Nombre de la etiqueta",
        "tag.preset_colour" => "Color predefinido",
        "tag.create_assign" => "Crear y asignar",

        // ── Theme ──
        "theme.classic_name" => "Panopticon clásico",
        "theme.classic_subtitle" => {
            "Usa el color actual del canvas como fondo base."
        }

        // ── Actions and dialogs ──
        "action.restore_selected" => "Restaurar seleccionado",
        "action.restore_all" => "Restaurar todo",
        "action.browse_image" => "Buscar imagen…",
        "action.clear_image" => "Limpiar imagen",
        "action.refresh_now" => "Refrescar ahora",
        "action.check_updates" => "Buscar actualizaciones",
        "action.auto_apply" => "Los cambios se aplican automáticamente.",
        "action.about" => "Acerca de",
        "action.load_profile" => "Cargar perfil",
        "action.reset_defaults" => "Restablecer valores por defecto",
        "action.close" => "Cerrar",
        "dialog.choose_background_image" => "Elegir imagen de fondo del dashboard",
        "dialog.reset_defaults.title" => "¿Restablecer toda la configuración?",
        "dialog.reset_defaults.description" => {
            "Esto reemplaza la configuración del workspace actual por los valores iniciales de Panopticon. No se puede deshacer."
        }
        "dialog.reset_defaults.success_title" => "Configuración restablecida",
        "dialog.reset_defaults.success" => "Se restauraron y aplicaron los valores iniciales.",
        "dialog.reset_defaults.failed_title" => "No se pudo guardar el restablecimiento",
        "dialog.reset_defaults.failed" => {
            "Los valores iniciales se aplicaron en memoria, pero Panopticon no pudo guardarlos. Revisa el aviso en Configuración."
        }
        "dialog.kill_process.title" => "¿Finalizar proceso?",
        "dialog.kill_process.description" => {
            "Esto cerrará {} por la fuerza y puede descartar trabajo sin guardar. ¿Continuar?"
        }
        "dialog.kill_process.success_title" => "Proceso finalizado",
        "dialog.kill_process.success" => "El proceso de la aplicación seleccionada fue finalizado.",
        "dialog.kill_process.failed_title" => "No se pudo finalizar el proceso",
        "dialog.kill_process.failed" => {
            "Panopticon no pudo finalizar el proceso seleccionado. Puede que ya se haya cerrado o que requiera permisos elevados."
        }

        // ── Command palette ──
        "command_palette.title" => "Paleta de comandos",
        "command_palette.helper" => "Busca y ejecuta comandos de layout, configuración y sistema.",
        "command_palette.search_placeholder" => "Escribe un comando…",
        "command_palette.commands_title" => "Comandos",
        "command_palette.commands_helper" => "Usa ↑/↓ para seleccionar, Enter para ejecutar y Esc para cerrar.",
        "command_palette.run" => "Ejecutar",
        "command_palette.no_available" => "No hay comandos disponibles",
        "command_palette.no_results" => "No se encontraron comandos",
        "command.category.layout" => "Layout",
        "command.category.theme" => "Tema",
        "command.category.system" => "Sistema",
        "command.category.settings" => "Configuración",
        "command.category.filters" => "Filtros",
        "command.category.windows" => "Ventanas",
        "command.category.workspace" => "Workspace",
        "command.layout_cycle" => "Layout: Siguiente",
        "command.layout_grid" => "Layout: Grid",
        "command.layout_mosaic" => "Layout: Mosaic",
        "command.layout_bento" => "Layout: Bento",
        "command.layout_fibonacci" => "Layout: Fibonacci",
        "command.layout_columns" => "Layout: Columnas",
        "command.layout_row" => "Layout: Fila",
        "command.layout_column" => "Layout: Columna",
        "command.layout_reset_ratios" => "Layout: Restablecer proporciones",
        "command.theme_cycle" => "Tema: Siguiente",
        "command.refresh_now" => "Refrescar ahora",
        "command.restore_all_hidden" => "Ventanas: Restaurar todas las apps ocultas",
        "command.open_settings" => "Abrir configuración",
        "command.settings_behavior" => "Configuración: Comportamiento y pantalla",
        "command.settings_filters" => "Configuración: Filtros",
        "command.settings_workspaces" => "Configuración: Workspaces",
        "command.settings_shortcuts" => "Configuración: Atajos",
        "command.settings_advanced" => "Configuración: Opciones avanzadas",
        "command.open_about" => "Abrir Acerca de",
        "command.open_menu" => "Abrir menú de la app",
        "command.clear_all_filters" => "Filtros: Limpiar todos",
        "command.clear_monitor_filter" => "Filtros: Limpiar monitor",
        "command.clear_tag_filter" => "Filtros: Limpiar etiqueta",
        "command.clear_app_filter" => "Filtros: Limpiar aplicación",
        "command.toggle_animations" => "Activar o desactivar animaciones",
        "command.toggle_toolbar" => "Mostrar u ocultar barra de estado",
        "command.toggle_window_info" => "Mostrar u ocultar datos de ventana",
        "command.toggle_always_on_top" => "Activar o desactivar siempre visible",
        "command.workspace_load_default" => "Workspace: Cargar default",
        "command.workspace_open_default" => "Workspace: Abrir default en otra instancia",
        "command.workspace_load" => "Workspace: Cargar {}",
        "command.workspace_open" => "Workspace: Abrir {} en otra instancia",
        "command.filter_monitor" => "Filtros: Monitor {}",
        "command.filter_tag" => "Filtros: Etiqueta {}",
        "command.hide_app" => "Ventanas: Ocultar {}",
        "command.filter_app" => "Filtros: Aplicación {}",
        "command.restore_hidden_app" => "Ventanas: Restaurar {}",
        "command.exit" => "Salir de Panopticon",

        // ── About window ──
        "about.title" => "Acerca de Panopticon",
        "about.subtitle" => {
            "Un dashboard nativo para Windows pensado para explorar ventanas abiertas mediante thumbnails DWM en vivo."
        }
        "about.version_title" => "Versión",
        "about.update_available" => "Nueva versión",
        "about.description_title" => "Aplicación",
        "about.description_body" => {
            "Panopticon te ayuda a previsualizar, organizar y activar ventanas del escritorio desde un único panel local de control."
        }
        "about.credits_title" => "Créditos",
        "about.credits_body" => {
            "Creado por gvastethecreator.\nConstruido con Rust, Slint, windows-rs y las APIs de thumbnails del Desktop Window Manager.\nIconografía de UI por HugeIcons.\nLicencia: MIT."
        }

        // ── Validation / CLI ──
        "settings.profile_invalid_chars" => {
            "El nombre del perfil contiene caracteres inválidos para archivos de Windows: {}"
        }
        "settings.workspace_invalid_chars" => {
            "El nombre del workspace contiene caracteres inválidos para archivos de Windows: {}"
        }
        "settings.profile_empty_name" => "El nombre del perfil no puede estar vacío",
        "settings.workspace_empty_name" => "El nombre del workspace no puede estar vacío",
        "cli.usage_heading" => "Uso:",
        "cli.options_heading" => "Opciones:",
        "cli.profile_option_help" => {
            "Carga o crea el perfil indicado desde %APPDATA%\\Panopticon\\profiles\\<nombre>.toml"
        }
        "cli.workspace_option_help" => {
            "Carga o crea el workspace indicado desde %APPDATA%\\Panopticon\\workspaces\\<nombre>.toml"
        }
        "cli.help_option_help" => "Muestra este texto de ayuda",
        "cli.help_option_version" => "Muestra la versión actual de Panopticon",
        "cli.missing_profile_value" => "Falta el valor para --profile",
        "cli.missing_workspace_value" => "Falta el valor para --workspace",
        "cli.unknown_argument" => "Argumento desconocido: {}",

        _ => return None,
    })
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_key_returns_value() {
        assert_eq!(en("menu.close_window"), "Close window");
    }

    #[test]
    fn spanish_key_returns_value() {
        assert_eq!(es("menu.close_window"), Some("Cerrar ventana"));
    }

    #[test]
    fn unknown_key_returns_fallback() {
        assert_eq!(en("nonexistent.key"), "[?]");
    }

    #[test]
    fn spanish_missing_key_falls_back() {
        assert_eq!(es("nonexistent.key"), None);
    }

    #[test]
    fn parse_locale_tag_spanish() {
        assert_eq!(parse_locale_tag("es-ES"), Locale::Spanish);
        assert_eq!(parse_locale_tag("es-MX"), Locale::Spanish);
        assert_eq!(parse_locale_tag("es"), Locale::Spanish);
    }

    #[test]
    fn parse_locale_tag_english() {
        assert_eq!(parse_locale_tag("en-US"), Locale::English);
        assert_eq!(parse_locale_tag("en"), Locale::English);
        assert_eq!(parse_locale_tag("fr-FR"), Locale::English);
    }

    #[test]
    fn set_locale_updates_current_locale() {
        assert_eq!(set_locale(Locale::Spanish), Locale::Spanish);
        assert_eq!(current(), Locale::Spanish);
        assert_eq!(set_locale(Locale::English), Locale::English);
        assert_eq!(current(), Locale::English);
    }

    #[test]
    fn critical_settings_and_command_keys_are_complete_in_both_locales() {
        const CRITICAL_KEYS: &[&str] = &[
            "window.command_palette_title",
            "settings.persistence_status.failed",
            "settings.persistence_status.failed_title",
            "settings.persistence.retry",
            "settings.persistence.open_logs",
            "settings.update.cancel",
            "settings.option.center_secondary.title",
            "settings.app_rules.pinned_slot.title",
            "settings.theme_catalog.title",
            "settings.workspace.metadata.title",
            "settings.workspace.description_placeholder",
            "settings.workspace.feedback.rename_confirm",
            "settings.workspace.feedback.delete_confirm",
            "settings.shortcut.search_placeholder",
            "settings.shortcut.record",
            "settings.shortcut.feedback.press_key",
            "settings.shortcut.feedback.recorded",
            "settings.shortcut.target.open_command_palette",
            "settings.layout_presets.title",
            "settings.layout_presets.feedback.empty",
            "settings.layout_presets.feedback.applied",
            "dialog.reset_defaults.title",
            "dialog.kill_process.title",
            "command_palette.title",
            "command_palette.helper",
            "command_palette.search_placeholder",
            "command_palette.commands_title",
            "command_palette.commands_helper",
            "command_palette.run",
            "command_palette.no_available",
            "command_palette.no_results",
            "command.category.layout",
            "command.category.theme",
            "command.category.system",
            "command.category.settings",
            "command.category.filters",
            "command.category.windows",
            "command.category.workspace",
            "command.layout_cycle",
            "command.layout_grid",
            "command.layout_mosaic",
            "command.layout_bento",
            "command.layout_fibonacci",
            "command.layout_columns",
            "command.layout_row",
            "command.layout_column",
            "command.layout_reset_ratios",
            "command.theme_cycle",
            "command.refresh_now",
            "command.restore_all_hidden",
            "command.open_settings",
            "command.settings_behavior",
            "command.settings_filters",
            "command.settings_workspaces",
            "command.settings_shortcuts",
            "command.settings_advanced",
            "command.open_about",
            "command.open_menu",
            "command.clear_all_filters",
            "command.clear_monitor_filter",
            "command.clear_tag_filter",
            "command.clear_app_filter",
            "command.toggle_animations",
            "command.toggle_toolbar",
            "command.toggle_window_info",
            "command.toggle_always_on_top",
            "command.workspace_load_default",
            "command.workspace_open_default",
            "command.workspace_load",
            "command.workspace_open",
            "command.filter_monitor",
            "command.filter_tag",
            "command.hide_app",
            "command.filter_app",
            "command.restore_hidden_app",
            "command.exit",
        ];

        for key in CRITICAL_KEYS {
            assert_ne!(en(key), "[?]", "missing English translation: {key}");
            assert!(es(key).is_some(), "missing Spanish translation: {key}");
        }
    }
}
