//! Shared `SettingsWindow` helpers used across callbacks and runtime sync.

use std::cell::Cell;

use panopticon::settings::AppSettings;
use slint::{Model, ModelRc, SharedString, VecModel};

use crate::SettingsWindow;

thread_local! {
    static BG_COLOR_SYNC_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn set_layout_preset_summary(window: &SettingsWindow, message: &str) {
    window.set_layout_preset_summary(SharedString::from(message));
}

pub(super) fn selected_layout_preset_name(window: &SettingsWindow) -> Option<String> {
    selected_model_value(
        &window.get_layout_preset_options(),
        window.get_layout_preset_index(),
    )
    .filter(|value| !value.trim().is_empty())
    .or_else(|| {
        let typed = window.get_layout_preset_name().trim().to_owned();
        (!typed.is_empty()).then_some(typed)
    })
}

pub(super) fn sync_layout_preset_controls(window: &SettingsWindow, settings: &AppSettings) {
    let names = settings.layout_preset_names();
    let selected_before = selected_model_value(
        &window.get_layout_preset_options(),
        window.get_layout_preset_index(),
    );
    let selected_index = selected_before
        .as_deref()
        .and_then(|name| names.iter().position(|candidate| candidate == name))
        .map_or(0, |index| index as i32);

    let has_presets = !names.is_empty();
    window.set_layout_preset_options(build_string_model(names));
    window.set_layout_preset_index(if has_presets { selected_index } else { 0 });

    if window.get_layout_preset_summary().trim().is_empty() {
        let summary = if has_presets {
            "Select a preset to apply or delete, or save current ratios as a new snapshot."
        } else {
            "No layout presets saved yet. Save current layout ratios to create one."
        };
        window.set_layout_preset_summary(SharedString::from(summary));
    }
}

pub(super) fn stop_shortcut_recording(window: &SettingsWindow, hint: &str) {
    window.set_shortcut_recording_mode(false);
    window.set_shortcut_recording_target(SharedString::from(""));
    window.set_shortcut_recording_hint(SharedString::from(hint));
}

pub(super) fn shortcut_recording_label(target: &str) -> &str {
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

pub(super) fn normalize_recorded_shortcut(key_text: &str) -> Option<String> {
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

pub(super) fn apply_recorded_shortcut_binding(
    window: &SettingsWindow,
    target: &str,
    binding: &str,
) -> bool {
    let binding = SharedString::from(binding);
    match target {
        "layout_column" => window.set_shortcut_layout_column(binding),
        "reset_layout" => window.set_shortcut_reset_layout(binding),
        "cycle_layout" => window.set_shortcut_cycle_layout(binding),
        "toggle_toolbar" => window.set_shortcut_toggle_toolbar(binding),
        "toggle_animations" => window.set_shortcut_toggle_animations(binding),
        "toggle_window_info" => window.set_shortcut_toggle_window_info(binding),
        "open_settings" => window.set_shortcut_open_settings(binding),
        "open_menu" => window.set_shortcut_open_menu(binding),
        "open_command_palette" => window.set_shortcut_open_command_palette(binding),
        "refresh_now" => window.set_shortcut_refresh_now(binding),
        "exit_app" => window.set_shortcut_exit_app(binding),
        "toggle_always_on_top" => window.set_shortcut_toggle_always_on_top(binding),
        "global_activate" => window.set_shortcut_global_activate(binding),
        _ => return false,
    }

    true
}

pub(super) fn build_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let values: Vec<SharedString> = values.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(values))
}

pub(super) fn selected_model_value(model: &ModelRc<SharedString>, index: i32) -> Option<String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.row_data(index))
        .map(|value| value.to_string())
}

pub(super) fn apply_background_color(window: &SettingsWindow, red: i32, green: i32, blue: i32) {
    let already_syncing = BG_COLOR_SYNC_IN_PROGRESS.with(|flag| {
        if flag.get() {
            true
        } else {
            flag.set(true);
            false
        }
    });
    if already_syncing {
        tracing::debug!("skipping re-entrant background color sync");
        return;
    }

    let red = clamp_rgb(red);
    let green = clamp_rgb(green);
    let blue = clamp_rgb(blue);
    let hex = format!("{red:02X}{green:02X}{blue:02X}");

    window.set_bg_red_value(red);
    window.set_bg_green_value(green);
    window.set_bg_blue_value(blue);
    window.set_bg_color_hex(SharedString::from(hex));
    window.set_bg_preview_color(slint::Color::from_rgb_u8(
        red as u8,
        green as u8,
        blue as u8,
    ));
    if !window.get_suspend_live_apply() {
        window.invoke_apply();
    }

    BG_COLOR_SYNC_IN_PROGRESS.with(|flag| flag.set(false));
}

fn clamp_rgb(value: i32) -> i32 {
    value.clamp(0, 255)
}

pub(super) fn parse_rgb_hex(input: &str) -> Option<(i32, i32, i32)> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    let red = i32::from(u8::from_str_radix(&hex[0..2], 16).ok()?);
    let green = i32::from(u8::from_str_radix(&hex[2..4], 16).ok()?);
    let blue = i32::from(u8::from_str_radix(&hex[4..6], 16).ok()?);
    Some((red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::{normalize_recorded_shortcut, parse_rgb_hex};

    #[test]
    fn recorded_shortcut_normalizes_special_keys() {
        assert_eq!(normalize_recorded_shortcut("\t"), Some("Tab".to_owned()));
        assert_eq!(normalize_recorded_shortcut("\n"), Some("Enter".to_owned()));
        assert_eq!(normalize_recorded_shortcut(" "), Some("Space".to_owned()));
    }

    #[test]
    fn recorded_shortcut_uppercases_single_alphanumeric_keys() {
        assert_eq!(normalize_recorded_shortcut("a"), Some("A".to_owned()));
        assert_eq!(normalize_recorded_shortcut("7"), Some("7".to_owned()));
    }

    #[test]
    fn recorded_shortcut_rejects_control_and_multi_character_inputs() {
        assert_eq!(normalize_recorded_shortcut(""), None);
        assert_eq!(normalize_recorded_shortcut("ab"), None);
        assert_eq!(normalize_recorded_shortcut("\u{0001}"), None);
    }

    #[test]
    fn parse_rgb_hex_accepts_plain_and_prefixed_values() {
        assert_eq!(parse_rgb_hex("#112233"), Some((17, 34, 51)));
        assert_eq!(parse_rgb_hex("A0B1C2"), Some((160, 177, 194)));
    }

    #[test]
    fn parse_rgb_hex_rejects_invalid_shapes() {
        assert_eq!(parse_rgb_hex("#123"), None);
        assert_eq!(parse_rgb_hex("#GG1122"), None);
    }
}
