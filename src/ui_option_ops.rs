//! Pure helpers for profile labels, option encoding, and tag-dialog defaults.

use crate::settings::{AppSelectionEntry, HiddenAppEntry};

/// Separator used when combining a label with its opaque application identifier.
pub const OPTION_SEPARATOR: &str = " — ";

/// Return a stable user-facing label for the active profile.
#[must_use]
pub fn current_profile_label(profile_name: Option<&str>) -> String {
    profile_name.unwrap_or("default").to_owned()
}

/// Convert a combo-box profile label back into an optional profile name.
#[must_use]
pub fn selected_profile_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Build a combo-box option for a selectable application.
#[must_use]
pub fn app_option_label(app: &AppSelectionEntry) -> String {
    format!("{}{}{}", app.label, OPTION_SEPARATOR, app.app_id)
}

/// Build a combo-box option for a hidden application.
#[must_use]
pub fn hidden_app_option_label(app: &HiddenAppEntry) -> String {
    format!("{}{}{}", app.label, OPTION_SEPARATOR, app.app_id)
}

/// Parse an opaque application identifier back out of a combo-box option.
#[must_use]
pub fn parse_option_value(value: &str) -> Option<String> {
    value
        .rsplit_once(OPTION_SEPARATOR)
        .map(|(_, raw)| raw.trim().to_owned())
        .filter(|raw| !raw.is_empty())
}

/// Suggest a normalized tag name from an application label.
#[must_use]
pub fn suggested_tag_name(label: &str) -> String {
    let lowered = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();

    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Map a persisted tag colour hex string to the tag dialog's preset index.
#[must_use]
pub fn tag_color_index(color_hex: &str) -> i32 {
    match color_hex.to_ascii_uppercase().as_str() {
        "5CA9FF" => 1,
        "3CCF91" => 2,
        "FF6B8A" => 3,
        "9B7BFF" => 4,
        "F4B740" => 5,
        _ => 0,
    }
}

/// Map the tag dialog preset index back to a persisted colour hex string.
#[must_use]
pub fn tag_color_hex(index: i32) -> String {
    match index {
        1 => "5CA9FF",
        2 => "3CCF91",
        3 => "FF6B8A",
        4 => "9B7BFF",
        5 => "F4B740",
        _ => "D29A5C",
    }
    .to_owned()
}
