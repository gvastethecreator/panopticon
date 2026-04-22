//! Theme presets derived from the bundled terminal-theme catalog.

use std::sync::OnceLock;

use serde::Deserialize;

const CLASSIC_THEME_LABEL: &str = "Classic Panopticon";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiTheme {
    pub id: Option<String>,
    pub label: String,
    pub bg_hex: String,
    pub toolbar_bg_hex: String,
    pub panel_bg_hex: String,
    pub card_bg_hex: String,
    pub border_hex: String,
    pub accent_hex: String,
    pub accent_soft_hex: String,
    pub text_hex: String,
    pub label_hex: String,
    pub muted_hex: String,
    pub hover_border_hex: String,
    pub placeholder_hex: String,
    pub footer_bg_hex: String,
    pub surface_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreset {
    pub id: String,
    pub label: String,
    pub ui: UiTheme,
}

#[derive(Debug, Clone, Deserialize)]
struct ThemeCatalogEntry {
    name: String,
    #[serde(default)]
    variant: String,
    background: String,
    foreground: String,
    cursor: String,
    color_01: String,
    color_05: String,
    color_06: String,
    color_11: String,
    color_13: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[must_use]
pub fn theme_labels() -> Vec<String> {
    let mut labels = vec![CLASSIC_THEME_LABEL.to_owned()];
    labels.extend(theme_presets().iter().map(|preset| preset.label.clone()));
    labels
}

#[must_use]
pub fn theme_index(theme_id: Option<&str>) -> i32 {
    theme_id
        .and_then(|id| theme_presets().iter().position(|preset| preset.id == id))
        .map_or(0, |index| index as i32 + 1)
}

#[must_use]
pub fn theme_id_by_index(index: i32) -> Option<String> {
    (index > 0)
        .then(|| (index - 1) as usize)
        .and_then(|index| theme_presets().get(index))
        .map(|preset| preset.id.clone())
}

pub fn theme_presets() -> &'static [ThemePreset] {
    static PRESETS: OnceLock<Vec<ThemePreset>> = OnceLock::new();
    PRESETS
        .get_or_init(|| {
            serde_json::from_str::<Vec<ThemeCatalogEntry>>(include_str!("../assets/themes.json"))
                .unwrap_or_default()
                .iter()
                .map(ThemePreset::from_catalog)
                .collect()
        })
        .as_slice()
}

#[must_use]
pub fn resolve_ui_theme(theme_id: Option<&str>, fallback_background_hex: &str) -> UiTheme {
    theme_id.and_then(resolve_preset_ui_theme).map_or_else(
        || classic_theme(fallback_background_hex),
        |preset| theme_with_background_override(&preset, fallback_background_hex),
    )
}

#[must_use]
pub fn theme_base_background_hex(theme_id: Option<&str>, fallback_background_hex: &str) -> String {
    theme_id.and_then(resolve_preset_ui_theme).map_or_else(
        || classic_theme(fallback_background_hex).bg_hex,
        |preset| preset.bg_hex,
    )
}

/// Pre-parsed RGB representation of a `UiTheme` for fast interpolation.
///
/// Avoids re-parsing 15 hex strings on every animation frame.
#[derive(Debug, Clone, Copy)]
pub struct RgbThemeSnapshot {
    pub bg: Rgb,
    pub toolbar_bg: Rgb,
    pub panel_bg: Rgb,
    pub card_bg: Rgb,
    pub border: Rgb,
    pub accent: Rgb,
    pub accent_soft: Rgb,
    pub text: Rgb,
    pub label: Rgb,
    pub muted: Rgb,
    pub hover_border: Rgb,
    pub placeholder: Rgb,
    pub footer_bg: Rgb,
    pub surface: Rgb,
}

static DEFAULT_RGB: Rgb = Rgb {
    r: 0x18,
    g: 0x15,
    b: 0x13,
};

impl RgbThemeSnapshot {
    /// Parse a `UiTheme` into pre-parsed RGB values.
    #[must_use]
    pub fn from_ui_theme(theme: &UiTheme) -> Self {
        Self {
            bg: parse_hex_rgb(&theme.bg_hex).unwrap_or(DEFAULT_RGB),
            toolbar_bg: parse_hex_rgb(&theme.toolbar_bg_hex).unwrap_or(DEFAULT_RGB),
            panel_bg: parse_hex_rgb(&theme.panel_bg_hex).unwrap_or(DEFAULT_RGB),
            card_bg: parse_hex_rgb(&theme.card_bg_hex).unwrap_or(DEFAULT_RGB),
            border: parse_hex_rgb(&theme.border_hex).unwrap_or(DEFAULT_RGB),
            accent: parse_hex_rgb(&theme.accent_hex).unwrap_or(DEFAULT_RGB),
            accent_soft: parse_hex_rgb(&theme.accent_soft_hex).unwrap_or(DEFAULT_RGB),
            text: parse_hex_rgb(&theme.text_hex).unwrap_or(DEFAULT_RGB),
            label: parse_hex_rgb(&theme.label_hex).unwrap_or(DEFAULT_RGB),
            muted: parse_hex_rgb(&theme.muted_hex).unwrap_or(DEFAULT_RGB),
            hover_border: parse_hex_rgb(&theme.hover_border_hex).unwrap_or(DEFAULT_RGB),
            placeholder: parse_hex_rgb(&theme.placeholder_hex).unwrap_or(DEFAULT_RGB),
            footer_bg: parse_hex_rgb(&theme.footer_bg_hex).unwrap_or(DEFAULT_RGB),
            surface: parse_hex_rgb(&theme.surface_hex).unwrap_or(DEFAULT_RGB),
        }
    }

    /// Interpolate between two snapshots and produce the resulting `UiTheme`.
    #[must_use]
    pub fn interpolate(&self, to: &Self, t: f32, target_theme: &UiTheme) -> UiTheme {
        let t = t.clamp(0.0, 1.0);
        UiTheme {
            id: target_theme.id.clone(),
            label: target_theme.label.clone(),
            bg_hex: to_hex(mix(self.bg, to.bg, t)),
            toolbar_bg_hex: to_hex(mix(self.toolbar_bg, to.toolbar_bg, t)),
            panel_bg_hex: to_hex(mix(self.panel_bg, to.panel_bg, t)),
            card_bg_hex: to_hex(mix(self.card_bg, to.card_bg, t)),
            border_hex: to_hex(mix(self.border, to.border, t)),
            accent_hex: to_hex(mix(self.accent, to.accent, t)),
            accent_soft_hex: to_hex(mix(self.accent_soft, to.accent_soft, t)),
            text_hex: to_hex(mix(self.text, to.text, t)),
            label_hex: to_hex(mix(self.label, to.label, t)),
            muted_hex: to_hex(mix(self.muted, to.muted, t)),
            hover_border_hex: to_hex(mix(self.hover_border, to.hover_border, t)),
            placeholder_hex: to_hex(mix(self.placeholder, to.placeholder, t)),
            footer_bg_hex: to_hex(mix(self.footer_bg, to.footer_bg, t)),
            surface_hex: to_hex(mix(self.surface, to.surface, t)),
        }
    }
}

#[must_use]
pub fn interpolate_ui_theme(from: &UiTheme, to: &UiTheme, t: f32) -> UiTheme {
    let t = t.clamp(0.0, 1.0);
    let from_rgb = RgbThemeSnapshot::from_ui_theme(from);
    let to_rgb = RgbThemeSnapshot::from_ui_theme(to);
    from_rgb.interpolate(&to_rgb, t, to)
}

impl ThemePreset {
    fn from_catalog(entry: &ThemeCatalogEntry) -> Self {
        let id = theme_catalog_id(&entry.name, &entry.variant);
        let bg = parse_hex_rgb(&entry.background).unwrap_or(Rgb {
            r: 0x18,
            g: 0x15,
            b: 0x13,
        });
        let fg = parse_hex_rgb(&entry.foreground).unwrap_or(Rgb {
            r: 0xE6,
            g: 0xE2,
            b: 0xDE,
        });

        let accent = parse_hex_rgb(&entry.cursor)
            .filter(|cursor| contrast_ratio(*cursor, bg) >= 1.25)
            .or_else(|| parse_hex_rgb(&entry.color_13))
            .or_else(|| parse_hex_rgb(&entry.color_05))
            .or_else(|| parse_hex_rgb(&entry.color_06))
            .unwrap_or(Rgb {
                r: 0xD2,
                g: 0x9A,
                b: 0x5C,
            });

        let toolbar = elevate(bg, 0.03);
        let panel = elevate(bg, 0.08);
        let card = elevate(bg, 0.05);
        let footer = elevate(bg, 0.025);
        let surface = elevate(bg, 0.04);
        let border = mix(bg, fg, 0.22);
        let label = mix(bg, fg, 0.78);
        let muted = mix(bg, fg, 0.56);
        let accent_soft = mix(bg, accent, 0.34);
        let placeholder = if is_dark(bg) {
            mix(bg, parse_hex_rgb(&entry.color_01).unwrap_or(bg), 0.55)
        } else {
            mix(bg, parse_hex_rgb(&entry.color_11).unwrap_or(bg), 0.35)
        };

        let label_text = display_label(&entry.name, &entry.variant);

        let ui = UiTheme {
            id: Some(id.clone()),
            label: label_text.clone(),
            bg_hex: to_hex(bg),
            toolbar_bg_hex: to_hex(toolbar),
            panel_bg_hex: to_hex(panel),
            card_bg_hex: to_hex(card),
            border_hex: to_hex(border),
            accent_hex: to_hex(accent),
            accent_soft_hex: to_hex(accent_soft),
            text_hex: to_hex(fg),
            label_hex: to_hex(label),
            muted_hex: to_hex(muted),
            hover_border_hex: to_hex(accent),
            placeholder_hex: to_hex(placeholder),
            footer_bg_hex: to_hex(footer),
            surface_hex: to_hex(surface),
        };

        Self {
            id,
            label: label_text,
            ui,
        }
    }
}

fn resolve_preset_ui_theme(theme_id: &str) -> Option<UiTheme> {
    theme_presets()
        .iter()
        .find(|preset| preset.id == theme_id)
        .map(|preset| preset.ui.clone())
}

fn theme_catalog_id(name: &str, variant: &str) -> String {
    let mut parts = vec![slugify_theme_part(name)];
    let variant = slugify_theme_part(variant);
    if !variant.is_empty() {
        parts.push(variant);
    }
    parts.join("--")
}

fn slugify_theme_part(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_dash = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_owned()
}

fn classic_theme(fallback_background_hex: &str) -> UiTheme {
    let bg = parse_hex_rgb(fallback_background_hex).unwrap_or(Rgb {
        r: 0x18,
        g: 0x15,
        b: 0x13,
    });
    UiTheme {
        id: None,
        label: CLASSIC_THEME_LABEL.to_owned(),
        bg_hex: to_hex(bg),
        toolbar_bg_hex: "1A1814".to_owned(),
        panel_bg_hex: "2A2522".to_owned(),
        card_bg_hex: "1E1C1A".to_owned(),
        border_hex: "3A3530".to_owned(),
        accent_hex: "D29A5C".to_owned(),
        accent_soft_hex: "5C4A38".to_owned(),
        text_hex: "E6E2DE".to_owned(),
        label_hex: "C8C1BA".to_owned(),
        muted_hex: "8D867F".to_owned(),
        hover_border_hex: "D29A5C".to_owned(),
        placeholder_hex: "141210".to_owned(),
        footer_bg_hex: "1A1814".to_owned(),
        surface_hex: "221E1C".to_owned(),
    }
}

fn theme_with_background_override(base: &UiTheme, background_hex: &str) -> UiTheme {
    let bg = parse_hex_rgb(background_hex)
        .or_else(|| parse_hex_rgb(&base.bg_hex))
        .unwrap_or(Rgb {
            r: 0x18,
            g: 0x15,
            b: 0x13,
        });
    let fg = parse_hex_rgb(&base.text_hex).unwrap_or(Rgb {
        r: 0xE6,
        g: 0xE2,
        b: 0xDE,
    });
    let accent = parse_hex_rgb(&base.accent_hex).unwrap_or(Rgb {
        r: 0xD2,
        g: 0x9A,
        b: 0x5C,
    });
    let placeholder_seed = parse_hex_rgb(&base.placeholder_hex).unwrap_or(bg);

    UiTheme {
        id: base.id.clone(),
        label: base.label.clone(),
        bg_hex: to_hex(bg),
        toolbar_bg_hex: to_hex(elevate(bg, 0.03)),
        panel_bg_hex: to_hex(elevate(bg, 0.08)),
        card_bg_hex: to_hex(elevate(bg, 0.05)),
        border_hex: to_hex(mix(bg, fg, 0.22)),
        accent_hex: to_hex(accent),
        accent_soft_hex: to_hex(mix(bg, accent, 0.34)),
        text_hex: to_hex(fg),
        label_hex: to_hex(mix(bg, fg, 0.78)),
        muted_hex: to_hex(mix(bg, fg, 0.56)),
        hover_border_hex: to_hex(accent),
        placeholder_hex: to_hex(if is_dark(bg) {
            mix(bg, placeholder_seed, 0.55)
        } else {
            mix(bg, placeholder_seed, 0.35)
        }),
        footer_bg_hex: to_hex(elevate(bg, 0.025)),
        surface_hex: to_hex(elevate(bg, 0.04)),
    }
}

fn display_label(name: &str, variant: &str) -> String {
    let name = name.trim();
    let variant = variant.trim();
    if variant.is_empty()
        || name
            .to_ascii_lowercase()
            .contains(&variant.to_ascii_lowercase())
    {
        name.to_owned()
    } else {
        format!("{name} ({variant})")
    }
}

fn parse_hex_rgb(hex: &str) -> Option<Rgb> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(Rgb {
        r: u8::from_str_radix(&hex[0..2], 16).ok()?,
        g: u8::from_str_radix(&hex[2..4], 16).ok()?,
        b: u8::from_str_radix(&hex[4..6], 16).ok()?,
    })
}

fn to_hex(rgb: Rgb) -> String {
    format!("{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}

fn mix(from: Rgb, to: Rgb, ratio: f32) -> Rgb {
    let ratio = ratio.clamp(0.0, 1.0);
    let inverse = 1.0 - ratio;
    Rgb {
        r: (f32::from(from.r) * inverse + f32::from(to.r) * ratio).round() as u8,
        g: (f32::from(from.g) * inverse + f32::from(to.g) * ratio).round() as u8,
        b: (f32::from(from.b) * inverse + f32::from(to.b) * ratio).round() as u8,
    }
}

fn elevate(bg: Rgb, amount: f32) -> Rgb {
    if is_dark(bg) {
        mix(
            bg,
            Rgb {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
            },
            amount,
        )
    } else {
        mix(
            bg,
            Rgb {
                r: 0x00,
                g: 0x00,
                b: 0x00,
            },
            amount,
        )
    }
}

fn is_dark(rgb: Rgb) -> bool {
    luminance(rgb) < 0.45
}

fn luminance(rgb: Rgb) -> f32 {
    let channel = |value: u8| {
        let v = f32::from(value) / 255.0;
        if v <= 0.039_28 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb.r) + 0.7152 * channel(rgb.g) + 0.0722 * channel(rgb.b)
}

fn contrast_ratio(left: Rgb, right: Rgb) -> f32 {
    let a = luminance(left) + 0.05;
    let b = luminance(right) + 0.05;
    if a > b {
        a / b
    } else {
        b / a
    }
}

#[cfg(test)]
mod tests {
    use super::{
        interpolate_ui_theme, resolve_ui_theme, theme_id_by_index, theme_index, theme_labels,
        theme_presets,
    };

    #[test]
    fn bundled_theme_catalog_is_available() {
        assert!(!theme_presets().is_empty());
        assert_eq!(
            theme_labels().first().map(String::as_str),
            Some("Classic Panopticon")
        );
    }

    #[test]
    fn theme_index_roundtrip_works() {
        let first = theme_id_by_index(1).expect("first bundled theme id");
        assert_eq!(theme_index(Some(&first)), 1);
    }

    #[test]
    fn unknown_theme_falls_back_to_classic() {
        let theme = resolve_ui_theme(Some("missing"), "181513");
        assert_eq!(theme.label, "Classic Panopticon");
        assert_eq!(theme.bg_hex, "181513");
    }

    #[test]
    fn bundled_theme_uses_override_background_color() {
        let bundled = theme_presets().first().expect("bundled theme exists");
        let resolved = resolve_ui_theme(Some(&bundled.id), "224466");

        assert_eq!(resolved.bg_hex, "224466");
        assert_eq!(resolved.accent_hex, bundled.ui.accent_hex);
    }

    #[test]
    fn ui_theme_interpolation_respects_endpoints() {
        let from = resolve_ui_theme(None, "181513");
        let to = resolve_ui_theme(None, "223344");

        assert_eq!(interpolate_ui_theme(&from, &to, 0.0), from);
        assert_eq!(interpolate_ui_theme(&from, &to, 1.0).bg_hex, to.bg_hex);
        assert_eq!(
            interpolate_ui_theme(&from, &to, 1.0).accent_hex,
            to.accent_hex
        );
    }
}
