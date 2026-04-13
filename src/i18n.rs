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
        "window.main_title" => "Panopticon — Window Viewer",
        "window.settings_title" => "Panopticon — Settings",
        "window.tag_title" => "Panopticon — Create tag",

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
        "tray.layout" => "Layout",
        "tray.next_layout" => "Next layout",
        "tray.lock_layout" => "Lock layout switching",
        "tray.lock_resize" => "Lock cell / column resizing",
        "tray.dock_position" => "Dock position",
        "tray.group_by" => "Group windows by",
        "tray.display" => "Display",
        "tray.show_toolbar" => "Show header",
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
        "ui.toolbar_hint" => "right-click header / M: menu  ·  Esc exit",
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
            "Filters and grouping are also reflected in the header and reorder visible cells."
        }
        "settings.no_saved_profiles" => "No saved profiles",
        "settings.saved_profiles" => "Saved profiles: default",
        "settings.saved_profiles_fmt" => "Saved profiles: default, {}",
        "settings.current_profile" => "Current profile: ",
        "settings.profile_label" => "Profile:",
        "settings.save_profile" => "Save profile",
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
        "settings.nav.profiles.title" => "Profiles",
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
        "settings.option.lock_layout.title" => "Lock layout changes",
        "settings.option.lock_layout.description" => {
            "Prevent layout changes from the keyboard or the top toolbar."
        }
        "settings.option.lock_cell_resize.title" => "Lock cell resizing",
        "settings.option.lock_cell_resize.description" => {
            "Disable separator dragging to protect the current composition."
        }
        "settings.section.display.title" => "Display",
        "settings.section.display.helper" => {
            "Controls that change readability and the information shown on the dashboard."
        }
        "settings.option.show_toolbar.title" => "Show toolbar",
        "settings.option.show_toolbar.description" => {
            "Show the top header with status summary and quick access to the menu."
        }
        "settings.option.show_info.title" => "Show window info above thumbnails",
        "settings.option.show_info.description" => {
            "Add the window and application name above each preview for quick context."
        }
        "settings.option.show_app_icons.title" => "Show app icons in cells",
        "settings.option.show_app_icons.description" => {
            "Render the process icon inside each cell to identify apps faster."
        }
        "settings.option.use_system_backdrop.title" => "Use system backdrop (Win11)",
        "settings.option.use_system_backdrop.description" => {
            "Enable more native system materials and borders when Windows 11 supports them."
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
        "settings.page.theme_background.title" => "Theme & Background",
        "settings.page.theme_background.subtitle" => {
            "Combine a theme library with a custom canvas colour and an adjustable background image."
        }
        "settings.section.theme_grid.title" => "Theme grid",
        "settings.section.theme_grid.helper" => {
            "Pick a preset from a scrollable grid; the theme defines accents, panels, and overall contrast."
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
        "settings.fit.cover" => "Cover",
        "settings.fit.contain" => "Contain",
        "settings.fit.fill" => "Fill",
        "settings.fit.preserve" => "Preserve",
        "settings.page.profiles.title" => "Profiles",
        "settings.page.profiles.subtitle" => {
            "Save complete combinations of settings and open new instances already pointed at the profile you want."
        }
        "settings.section.edit_profile.title" => "Edit profile",
        "settings.section.edit_profile.helper" => {
            "Use a short, descriptive name to save or open the current snapshot in another instance."
        }
        "settings.current_profile_card.title" => "Current profile",
        "settings.option.profile_name.title" => "Profile name",
        "settings.option.profile_name.description" => {
            "Name used to save this setup or launch another instance with it."
        }
        "settings.section.saved_profiles.title" => "Saved profiles",
        "settings.section.saved_profiles.helper" => {
            "Summary of detected profiles plus a reminder of the recommended multi-instance workflow."
        }
        "settings.tips.title" => "Tips",
        "settings.tips.body" => {
            "- Save the current profile first if you are about to open another instance.\n- Use simple names like work, stream, or review.\n- Theme, background, and shortcut settings travel with the profile."
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
            "Rotate between presets without opening the settings window."
        }
        "settings.shortcut.toggle_animations.title" => "Toggle animations",
        "settings.shortcut.toggle_animations.description" => {
            "Enable or disable dashboard transitions."
        }
        "settings.shortcut.toggle_toolbar.title" => "Toggle toolbar",
        "settings.shortcut.toggle_toolbar.description" => {
            "Show or hide the dashboard's top bar."
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
        "settings.shortcut.alt_toolbar.title" => "Use Alt as a quick toolbar toggle",
        "settings.shortcut.alt_toolbar.description" => {
            "Keep the legacy Windows shortcut to hide or show the toolbar with a single Alt press."
        }
        "settings.page.advanced.title" => "Advanced Options",
        "settings.page.advanced.subtitle" => {
            "Initial layout, refresh cadence, docked dimensions, and appbar position."
        }
        "settings.option.default_layout.title" => "Default layout",
        "settings.option.default_layout.description" => {
            "Choose which layout Panopticon should use each time this profile starts."
        }
        "settings.option.refresh_interval.title" => "Refresh interval",
        "settings.option.refresh_interval.description" => {
            "Base cadence for enumerating windows and refreshing the dashboard when you do not force a manual refresh."
        }
        "settings.section.manual_refresh.title" => "Manual refresh",
        "settings.section.manual_refresh.helper" => {
            "Use this when you want to re-enumerate windows immediately instead of waiting for the timer."
        }
        "settings.section.dock_thickness.title" => "Dock thickness",
        "settings.section.dock_thickness.helper" => {
            "For a side dock, width is used; for top/bottom, height is used. 0 leaves the size automatic."
        }
        "settings.label.width" => "Width",
        "settings.label.height" => "Height",
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
        "action.auto_apply" => "Changes apply automatically.",
        "action.reset_defaults" => "Reset defaults",
        "action.close" => "Close",
        "dialog.choose_background_image" => "Choose dashboard background image",

        // ── Validation / CLI ──
        "settings.profile_invalid_chars" => {
            "Profile name contains invalid Windows filename characters: {}"
        }
        "settings.profile_empty_name" => "Profile name cannot be empty",
        "cli.usage_heading" => "Usage:",
        "cli.options_heading" => "Options:",
        "cli.profile_option_help" => {
            "Load or create the named profile from %APPDATA%\\Panopticon\\profiles\\<name>.toml"
        }
        "cli.help_option_help" => "Show this help text",
        "cli.help_option_version" => "Show the current Panopticon version",
        "cli.missing_profile_value" => "Missing value for --profile",
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
        "window.main_title" => "Panopticon — Visor de ventanas",
        "window.settings_title" => "Panopticon — Configuración",
        "window.tag_title" => "Panopticon — Crear etiqueta",

        // ── Locales ──
        "locale.english" => "Inglés",
        "locale.spanish" => "Español",

        // ── Layout labels ──
        "layout.grid" => "Grid",
        "layout.mosaic" => "Mosaic",
        "layout.bento" => "Bento",
        "layout.fibonacci" => "Fibonacci",
        "layout.columns" => "Columns",
        "layout.row" => "Row",
        "layout.column" => "Column",

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
        "tray.layout" => "Layout",
        "tray.next_layout" => "Siguiente layout",
        "tray.lock_layout" => "Bloquear cambio de layout",
        "tray.lock_resize" => "Bloquear redimensionado de celdas",
        "tray.dock_position" => "Posición de dock",
        "tray.group_by" => "Agrupar ventanas por",
        "tray.display" => "Pantalla",
        "tray.show_toolbar" => "Mostrar header",
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
        "ui.toolbar_hint" => "click der. header / M: menú  ·  Esc salir",
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
            "Los filtros y el agrupado también se reflejan en el header y reordenan las celdas visibles."
        }
        "settings.no_saved_profiles" => "Sin perfiles guardados",
        "settings.saved_profiles" => "Perfiles guardados: default",
        "settings.saved_profiles_fmt" => "Perfiles guardados: default, {}",
        "settings.current_profile" => "Perfil actual: ",
        "settings.profile_label" => "Perfil:",
        "settings.save_profile" => "Guardar perfil",
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
        "settings.nav.profiles.title" => "Perfiles",
        "settings.nav.profiles.subtitle" => "Guardá y abrí configuraciones con nombre",
        "settings.nav.shortcuts.title" => "Atajos de teclado",
        "settings.nav.shortcuts.subtitle" => "Personalizá el mapa de teclas del dashboard",
        "settings.nav.advanced.title" => "Opciones avanzadas",
        "settings.nav.advanced.subtitle" => {
            "Layout, cadencia de refresco y comportamiento del dock"
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
        "settings.option.lock_layout.title" => "Bloquear cambios de layout",
        "settings.option.lock_layout.description" => {
            "Bloquea cambios de layout por teclado o desde la barra superior."
        }
        "settings.option.lock_cell_resize.title" => "Bloquear redimensionado de celdas",
        "settings.option.lock_cell_resize.description" => {
            "Desactiva el arrastre de separadores para proteger la composición actual."
        }
        "settings.section.display.title" => "Vista",
        "settings.section.display.helper" => {
            "Controles que cambian la legibilidad y el contenido visible del dashboard."
        }
        "settings.option.show_toolbar.title" => "Mostrar barra superior",
        "settings.option.show_toolbar.description" => {
            "Muestra la cabecera superior con resumen de estado y acceso rápido al menú."
        }
        "settings.option.show_info.title" => "Mostrar info de ventana sobre las miniaturas",
        "settings.option.show_info.description" => {
            "Añade el nombre de ventana y aplicación encima de cada preview para leer el contexto de un vistazo."
        }
        "settings.option.show_app_icons.title" => "Mostrar iconos de apps en celdas",
        "settings.option.show_app_icons.description" => {
            "Pinta el icono del proceso dentro de cada celda para identificar apps más rápido."
        }
        "settings.option.use_system_backdrop.title" => "Usar backdrop del sistema (Win11)",
        "settings.option.use_system_backdrop.description" => {
            "Activa materiales del sistema y bordes más nativos cuando Windows 11 los soporta."
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
        "settings.page.theme_background.title" => "Tema y fondo",
        "settings.page.theme_background.subtitle" => {
            "Combina una librería de themes con un color de canvas propio y una imagen de fondo ajustable."
        }
        "settings.section.theme_grid.title" => "Grilla de temas",
        "settings.section.theme_grid.helper" => {
            "Seleccioná un preset desde una grilla desplazable; el tema define acentos, paneles y contraste general."
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
        "settings.fit.cover" => "Cubrir",
        "settings.fit.contain" => "Contener",
        "settings.fit.fill" => "Rellenar",
        "settings.fit.preserve" => "Preservar",
        "settings.page.profiles.title" => "Perfiles",
        "settings.page.profiles.subtitle" => {
            "Guardá combinaciones completas de ajustes y abrí nuevas instancias ya apuntando al perfil que quieras."
        }
        "settings.section.edit_profile.title" => "Editar perfil",
        "settings.section.edit_profile.helper" => {
            "Usá un nombre corto y descriptivo para guardar o abrir el snapshot actual en otra instancia."
        }
        "settings.current_profile_card.title" => "Perfil actual",
        "settings.option.profile_name.title" => "Nombre del perfil",
        "settings.option.profile_name.description" => {
            "Nombre usado para guardar esta configuración o lanzar otra instancia con ella."
        }
        "settings.section.saved_profiles.title" => "Perfiles guardados",
        "settings.section.saved_profiles.helper" => {
            "Resumen de perfiles detectados y recordatorio del flujo recomendado para trabajar con varias instancias."
        }
        "settings.tips.title" => "Consejos",
        "settings.tips.body" => {
            "- Guarda primero el perfil actual si vas a abrir otra instancia.\n- Usa nombres simples como work, stream o review.\n- Los ajustes de tema, fondo y shortcuts viajan con el perfil."
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
        "settings.shortcut.layout_columns.title" => "Layout Columns",
        "settings.shortcut.layout_columns.description" => "Activa Columns.",
        "settings.shortcut.layout_row.title" => "Layout Row",
        "settings.shortcut.layout_row.description" => "Activa la vista Row.",
        "settings.shortcut.layout_column.title" => "Layout Column",
        "settings.shortcut.layout_column.description" => "Activa la vista Column.",
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
            "Rota entre presets sin abrir la ventana de configuración."
        }
        "settings.shortcut.toggle_animations.title" => "Alternar animaciones",
        "settings.shortcut.toggle_animations.description" => {
            "Activa o desactiva transiciones del dashboard."
        }
        "settings.shortcut.toggle_toolbar.title" => "Alternar barra superior",
        "settings.shortcut.toggle_toolbar.description" => {
            "Muestra u oculta la barra superior del dashboard."
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
        "settings.shortcut.alt_toolbar.title" => "Usar Alt como atajo rápido para la barra",
        "settings.shortcut.alt_toolbar.description" => {
            "Mantiene el atajo legacy de Windows para esconder o mostrar la toolbar con una sola pulsación de Alt."
        }
        "settings.page.advanced.title" => "Opciones avanzadas",
        "settings.page.advanced.subtitle" => {
            "Ajustes de layout inicial, refresco, dimensiones dockeadas y posición del appbar."
        }
        "settings.option.default_layout.title" => "Layout por defecto",
        "settings.option.default_layout.description" => {
            "Define con qué layout debe arrancar Panopticon cada vez que abras este perfil."
        }
        "settings.option.refresh_interval.title" => "Intervalo de refresco",
        "settings.option.refresh_interval.description" => {
            "Cadencia base para enumerar ventanas y actualizar el dashboard cuando no fuerzas un refresh manual."
        }
        "settings.section.manual_refresh.title" => "Refresco manual",
        "settings.section.manual_refresh.helper" => {
            "Úsalo cuando quieras reenumerar ventanas inmediatamente sin esperar al temporizador."
        }
        "settings.section.dock_thickness.title" => "Grosor del dock",
        "settings.section.dock_thickness.helper" => {
            "Para dock lateral se usa width; para top/bottom se usa height. 0 deja el tamaño en automático."
        }
        "settings.label.width" => "Ancho",
        "settings.label.height" => "Alto",
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
        "action.auto_apply" => "Los cambios se aplican automáticamente.",
        "action.reset_defaults" => "Restablecer valores por defecto",
        "action.close" => "Cerrar",
        "dialog.choose_background_image" => "Elegir imagen de fondo del dashboard",

        // ── Validation / CLI ──
        "settings.profile_invalid_chars" => {
            "El nombre del perfil contiene caracteres inválidos para archivos de Windows: {}"
        }
        "settings.profile_empty_name" => "El nombre del perfil no puede estar vacío",
        "cli.usage_heading" => "Uso:",
        "cli.options_heading" => "Opciones:",
        "cli.profile_option_help" => {
            "Carga o crea el perfil indicado desde %APPDATA%\\Panopticon\\profiles\\<nombre>.toml"
        }
        "cli.help_option_help" => "Muestra este texto de ayuda",
        "cli.help_option_version" => "Muestra la versión actual de Panopticon",
        "cli.missing_profile_value" => "Falta el valor para --profile",
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
}
