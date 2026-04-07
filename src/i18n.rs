//! Internationalization support for Panopticon.
//!
//! Detects the system locale at startup and provides translated strings
//! for the UI. Supports English (default) and Spanish.
//!
//! # Locale detection order
//!
//! 1. `PANOPTICON_LANG` environment variable (`en`, `es`).
//! 2. Windows system locale via `GetUserDefaultLocaleName`.
//! 3. Falls back to English.

use std::sync::OnceLock;

// ── Locale type ──────────────────────────────────────────────

/// Supported UI locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    English,
    Spanish,
}

static LOCALE: OnceLock<Locale> = OnceLock::new();

/// Detect and lock the UI locale. Call once at startup.
pub fn init() {
    let locale = detect_locale();
    let _ = LOCALE.set(locale);
    tracing::info!(?locale, "i18n locale resolved");
}

/// Return the active locale.
#[must_use]
pub fn current() -> Locale {
    LOCALE.get().copied().unwrap_or(Locale::English)
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

fn detect_locale() -> Locale {
    // 1. Environment override
    if let Ok(lang) = std::env::var("PANOPTICON_LANG") {
        return parse_locale_tag(&lang);
    }

    // 2. Windows system locale
    detect_windows_locale()
}

fn detect_windows_locale() -> Locale {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
    }

    let mut buf = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
                              // SAFETY: `buf` is a stack-allocated array with known size; the function
                              // writes at most `cch_locale_name` wide chars including the NUL terminator.
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), 85) };
    if len > 1 {
        let name = String::from_utf16_lossy(&buf[..(len as usize).saturating_sub(1)]);
        return parse_locale_tag(&name);
    }

    Locale::English
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

#[allow(clippy::too_many_lines)]
fn en(key: &str) -> &'static str {
    match key {
        // ── Window context menu ──
        "menu.hide_from_layout" => "Hide from layout",
        "menu.pin_position" => "Pin app at this position",
        "menu.preserve_aspect" => "Preserve aspect ratio",
        "menu.hide_on_select" => "Hide Panopticon when activating this app",
        "menu.create_tag" => "Create custom tag\u{2026}",
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

        // ── Tray tooltip ──
        "tray.tooltip" => "Panopticon \u{2014} Live window overview",

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
        "ui.toolbar_hint" => "right-click header / M: menu  \u{00b7}  Esc exit",
        "ui.anim_on" => "anim on",
        "ui.anim_off" => "anim off",

        // ── Empty state ──
        "ui.empty_message" => "No windows available to preview",
        "ui.empty_helper" => {
            "Open or restore any desktop window.\nPanopticon will keep watching from the tray."
        }

        // ── Settings ──
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

        // ── Tag dialog ──
        "tag.title" => "Create custom tag",
        "tag.application" => "Application: ",
        "tag.name_label" => "Tag name",
        "tag.preset_colour" => "Preset colour",
        "tag.create_assign" => "Create and assign",

        // ── Fallback ──
        other => {
            tracing::warn!(key = other, "missing i18n key");
            "[?]"
        }
    }
}

// ── Spanish translations ─────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn es(key: &str) -> Option<&'static str> {
    Some(match key {
        // ── Window context menu ──
        "menu.hide_from_layout" => "Ocultar del layout",
        "menu.pin_position" => "Fijar app en esta ubicaci\u{00f3}n",
        "menu.preserve_aspect" => "Respetar relaci\u{00f3}n de aspecto",
        "menu.hide_on_select" => "Ocultar Panopticon al abrir esta app",
        "menu.create_tag" => "Crear etiqueta personalizada\u{2026}",
        "menu.cell_color" => "Color de la celda",
        "menu.use_theme_color" => "Usar color del theme",
        "menu.close_window" => "Cerrar ventana",
        "menu.kill_process" => "Matar proceso",

        // ── Colour presets ──
        "color.amber" => "Usar \u{00e1}mbar",
        "color.sky" => "Usar cielo",
        "color.mint" => "Usar menta",
        "color.rose" => "Usar rosa",
        "color.violet" => "Usar violeta",
        "color.sun" => "Usar sol",

        // ── Tray tooltip ──
        "tray.tooltip" => "Panopticon \u{2014} Vista en vivo de ventanas",

        // ── Tray menu ──
        "tray.visibility" => "Visibilidad",
        "tray.show" => "Mostrar Panopticon",
        "tray.hide" => "Ocultar al tray",
        "tray.refresh" => "Refrescar ventanas",
        "tray.open_settings" => "Abrir configuraci\u{00f3}n",
        "tray.layout" => "Layout",
        "tray.next_layout" => "Siguiente layout",
        "tray.lock_layout" => "Bloquear cambio de layout",
        "tray.lock_resize" => "Bloquear redimensionado de celdas",
        "tray.dock_position" => "Posici\u{00f3}n de dock",
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
        "tray.default_aspect" => "Default: preservar relaci\u{00f3}n de aspecto",
        "tray.default_hide" => "Default: ocultar al activar",
        "tray.start_tray" => "Iniciar oculto en tray",
        "tray.filters" => "Filtros",
        "tray.filter_monitor" => "Filtrar por monitor",
        "tray.all_monitors" => "Todos los monitores",
        "tray.filter_tag" => "Filtrar por etiqueta",
        "tray.all_tags" => "Todas las etiquetas",
        "tray.filter_app" => "Filtrar por aplicaci\u{00f3}n",
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
        "group.none" => "Sin agrupaci\u{00f3}n",
        "group.application" => "Aplicaci\u{00f3}n",
        "group.monitor" => "Monitor",
        "group.title" => "T\u{00ed}tulo de ventana",
        "group.class" => "Clase de ventana",
        "filter.grouped_by" => "agrupado por:",

        // ── UI labels (Slint) ──
        "ui.minimized" => "minimizada",
        "ui.last_seen" => "\u{00da}LTIMA VISTA",
        "ui.visible" => "visibles",
        "ui.hidden" => "ocultas",
        "ui.always_on_top" => "siempre visible",
        "ui.normal_window" => "ventana normal",
        "ui.toolbar_hint" => "click der. header / M: men\u{00fa}  \u{00b7}  Esc salir",
        "ui.anim_on" => "anim on",
        "ui.anim_off" => "anim off",

        // ── Empty state ──
        "ui.empty_message" => "No hay ventanas disponibles",
        "ui.empty_helper" => "Abr\u{00ed} o restaur\u{00e1} cualquier ventana del escritorio.\nPanopticon seguir\u{00e1} vigilando desde el tray.",

        // ── Settings ──
        "settings.dock_hint" => "En modo dock esta opci\u{00f3}n queda desactivada autom\u{00e1}ticamente.",
        "settings.filters_hint" => "Los filtros y el agrupado tambi\u{00e9}n se reflejan en el header y reordenan las celdas visibles.",
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

        // ── Tag dialog ──
        "tag.title" => "Crear etiqueta personalizada",
        "tag.application" => "Aplicaci\u{00f3}n: ",
        "tag.name_label" => "Nombre de la etiqueta",
        "tag.preset_colour" => "Color predefinido",
        "tag.create_assign" => "Crear y asignar",

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
}
