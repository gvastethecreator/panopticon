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
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
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
    theme_id
        .and_then(|id| theme_presets().iter().find(|preset| preset.id == id))
        .map_or_else(
            || classic_theme(fallback_background_hex),
            |preset| preset.ui.clone(),
        )
}

#[must_use]
pub fn interpolate_ui_theme(from: &UiTheme, to: &UiTheme, t: f32) -> UiTheme {
    let t = t.clamp(0.0, 1.0);
    UiTheme {
        id: to.id.clone(),
        label: to.label.clone(),
        bg_hex: interpolate_hex(&from.bg_hex, &to.bg_hex, t),
        toolbar_bg_hex: interpolate_hex(&from.toolbar_bg_hex, &to.toolbar_bg_hex, t),
        panel_bg_hex: interpolate_hex(&from.panel_bg_hex, &to.panel_bg_hex, t),
        card_bg_hex: interpolate_hex(&from.card_bg_hex, &to.card_bg_hex, t),
        border_hex: interpolate_hex(&from.border_hex, &to.border_hex, t),
        accent_hex: interpolate_hex(&from.accent_hex, &to.accent_hex, t),
        accent_soft_hex: interpolate_hex(&from.accent_soft_hex, &to.accent_soft_hex, t),
        text_hex: interpolate_hex(&from.text_hex, &to.text_hex, t),
        label_hex: interpolate_hex(&from.label_hex, &to.label_hex, t),
        muted_hex: interpolate_hex(&from.muted_hex, &to.muted_hex, t),
        hover_border_hex: interpolate_hex(&from.hover_border_hex, &to.hover_border_hex, t),
        placeholder_hex: interpolate_hex(&from.placeholder_hex, &to.placeholder_hex, t),
        footer_bg_hex: interpolate_hex(&from.footer_bg_hex, &to.footer_bg_hex, t),
        surface_hex: interpolate_hex(&from.surface_hex, &to.surface_hex, t),
    }
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

fn interpolate_hex(from: &str, to: &str, t: f32) -> String {
    let Some(from_rgb) = parse_hex_rgb(from) else {
        return to.to_owned();
    };
    let Some(to_rgb) = parse_hex_rgb(to) else {
        return from.to_owned();
    };

    to_hex(mix(from_rgb, to_rgb, t))
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
