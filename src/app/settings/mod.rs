//! Settings window data binding and pure helpers.
//!
//! Extracted incrementally from `secondary_windows/` using the
//! strangler-fig pattern: each function is moved here, the old
//! location becomes a thin re-export, and once all callers are
//! migrated the re-export is removed.

pub mod app_rules_sync;
pub mod callbacks;
pub mod filter_sync;
pub mod helpers;
pub mod preset_sync;
pub mod sync;
pub mod translations;
pub mod ui;

use slint::{Model, ModelRc, SharedString, VecModel};

/// Parse a 6-digit RGB hex string (with or without `#` prefix).
///
/// Returns `None` if the input is not exactly 6 hex digits.
pub fn parse_rgb_hex(input: &str) -> Option<(i32, i32, i32)> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let red = i32::from(u8::from_str_radix(&hex[0..2], 16).ok()?);
    let green = i32::from(u8::from_str_radix(&hex[2..4], 16).ok()?);
    let blue = i32::from(u8::from_str_radix(&hex[4..6], 16).ok()?);
    Some((red, green, blue))
}

/// Extract `(r, g, b)` from a hex string, falling back to the default dark-grey
/// background colour when the input is malformed.
pub fn rgb_components_from_hex(hex: &str) -> (u8, u8, u8) {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("18"), 16).unwrap_or(0x18);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("15"), 16).unwrap_or(0x15);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("13"), 16).unwrap_or(0x13);
    (red, green, blue)
}

/// Build a Slint string model from a vector of Rust strings.
pub fn build_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let values: Vec<SharedString> = values.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(values))
}

/// Extract the string value at `index` from a Slint string model.
pub fn selected_model_value(model: &ModelRc<SharedString>, index: i32) -> Option<String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.row_data(index))
        .map(|value| value.to_string())
}

/// Human-readable label for a shortcut recording target.
pub fn shortcut_recording_label(target: &str) -> &'static str {
    match target {
        "layout_column" => "Layout column",
        "reset_layout" => "Reset layout",
        "cycle_layout" => "Cycle layout",
        "toggle_toolbar" => "Toggle toolbar",
        "toggle_animations" => "Toggle animations",
        "toggle_window_info" => "Toggle window info",
        "open_settings" => "Open settings",
        "open_menu" => "Open menu",
        "open_command_palette" => "Open command palette",
        "refresh_now" => "Refresh now",
        "exit_app" => "Exit app",
        "toggle_always_on_top" => "Always on top",
        "global_activate" => "Global activate",
        _ => "Shortcut",
    }
}

/// Normalise a raw key-press string into a canonical shortcut name.
///
/// Returns `None` for multi-character or control sequences that are not
/// recognised shortcuts.
pub fn normalize_recorded_shortcut(key_text: &str) -> Option<String> {
    match key_text {
        "\t" => Some("Tab".to_owned()),
        "\n" | "\r" => Some("Enter".to_owned()),
        " " => Some("Space".to_owned()),
        _ => {
            let mut chars = key_text.chars();
            let first = chars.next()?;
            if chars.next().is_some() || first.is_control() {
                return None;
            }

            if first.is_ascii_alphanumeric() {
                Some(first.to_ascii_uppercase().to_string())
            } else {
                Some(first.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rgb_components_from_hex;

    #[test]
    fn rgb_components_from_hex_parses_valid_hex() {
        let (r, g, b) = rgb_components_from_hex("#AABBCC");
        assert_eq!(r, 0xAA);
        assert_eq!(g, 0xBB);
        assert_eq!(b, 0xCC);
    }

    #[test]
    fn rgb_components_from_hex_defaults_on_short_input() {
        let (r, g, b) = rgb_components_from_hex("#12");
        assert_eq!(r, 0x12);
        assert_eq!(g, 0x15);
        assert_eq!(b, 0x13);
    }
}
