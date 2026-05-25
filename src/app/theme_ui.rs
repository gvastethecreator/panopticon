//! Theme snapshot application, interpolation, and Slint globals sync.

use slint::language::ColorScheme;
use slint::ComponentHandle;

use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;

use crate::{
    AboutWindow, CommandPaletteWindow, MainWindow, Palette, SettingsWindow, TagDialogWindow, Theme,
};

fn palette_color_scheme_for_theme(resolved: &theme_catalog::UiTheme) -> ColorScheme {
    if theme_catalog::is_ui_theme_dark(resolved) {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

fn apply_palette_color_scheme<Component>(window: &Component, resolved: &theme_catalog::UiTheme)
where
    Component: ComponentHandle,
    for<'a> Palette<'a>: slint::Global<'a, Component>,
{
    window
        .global::<Palette>()
        .set_color_scheme(palette_color_scheme_for_theme(resolved));
}

// ───────────────────────── Macro ─────────────────────────

macro_rules! apply_runtime_theme {
    ($window:expr, $resolved:expr) => {{
        let globals = $window.global::<Theme>();
        globals.set_bg(hex_to_slint_color(&$resolved.bg_hex));
        globals.set_toolbar_bg(hex_to_slint_color(&$resolved.toolbar_bg_hex));
        globals.set_panel_bg(hex_to_slint_color(&$resolved.panel_bg_hex));
        globals.set_card_bg(hex_to_slint_color(&$resolved.card_bg_hex));
        globals.set_border(hex_to_slint_color(&$resolved.border_hex));
        globals.set_accent(hex_to_slint_color(&$resolved.accent_hex));
        globals.set_accent_soft(hex_to_slint_color(&$resolved.accent_soft_hex));
        globals.set_text(hex_to_slint_color(&$resolved.text_hex));
        globals.set_label(hex_to_slint_color(&$resolved.label_hex));
        globals.set_muted(hex_to_slint_color(&$resolved.muted_hex));
        globals.set_hover_border(hex_to_slint_color(&$resolved.hover_border_hex));
        globals.set_placeholder(hex_to_slint_color(&$resolved.placeholder_hex));
        globals.set_footer_bg(hex_to_slint_color(&$resolved.footer_bg_hex));
        globals.set_surface(hex_to_slint_color(&$resolved.surface_hex));
        globals.set_dark_scheme(theme_catalog::is_ui_theme_dark($resolved));
    }};
}

// ───────────────────────── Snapshot apply ─────────────────────────

pub(crate) fn apply_main_window_palette_color_scheme(
    window: &MainWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
}

pub(crate) fn apply_settings_window_palette_color_scheme(
    window: &SettingsWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
}

pub(crate) fn apply_tag_dialog_palette_color_scheme(
    window: &TagDialogWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
}

pub(crate) fn apply_about_window_palette_color_scheme(
    window: &AboutWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
}

pub(crate) fn apply_command_palette_window_palette_color_scheme(
    window: &CommandPaletteWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
}

pub(crate) fn apply_main_window_theme_snapshot(
    window: &MainWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_settings_window_theme_snapshot(
    window: &SettingsWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_tag_dialog_theme_snapshot(
    window: &TagDialogWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_about_window_theme_snapshot(
    window: &AboutWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_command_palette_window_theme_snapshot(
    window: &CommandPaletteWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_palette_color_scheme(window, resolved);
    apply_runtime_theme!(window, resolved);
}

// ───────────────────────── Accent / color helpers ─────────────────────────

pub(crate) fn default_thumbnail_accent_color(
    settings: &AppSettings,
    theme: &theme_catalog::UiTheme,
) -> slint::Color {
    settings.active_tag_filter.as_deref().map_or_else(
        || hex_to_slint_color(&theme.accent_hex),
        |tag| hex_to_slint_color(&settings.tag_color_hex(tag)),
    )
}

pub(crate) fn thumbnail_accent_color(
    settings: &AppSettings,
    theme: &theme_catalog::UiTheme,
    app_id: &str,
) -> slint::Color {
    settings.app_color_hex(app_id).map_or_else(
        || default_thumbnail_accent_color(settings, theme),
        hex_to_slint_color,
    )
}

pub(crate) fn hex_to_slint_color(hex: &str) -> slint::Color {
    let bytes = hex.as_bytes();
    let r = if bytes.len() >= 2 {
        hex_byte(bytes[0], bytes[1])
    } else {
        0xD2
    };
    let g = if bytes.len() >= 4 {
        hex_byte(bytes[2], bytes[3])
    } else {
        0x9A
    };
    let b = if bytes.len() >= 6 {
        hex_byte(bytes[4], bytes[5])
    } else {
        0x5C
    };
    slint::Color::from_rgb_u8(r, g, b)
}

#[inline]
fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

#[inline]
fn hex_byte(hi: u8, lo: u8) -> u8 {
    hex_nibble(hi) << 4 | hex_nibble(lo)
}
