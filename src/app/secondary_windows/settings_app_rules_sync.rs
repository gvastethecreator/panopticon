//! App-rule list and editor builders for the settings window.
//!
//! Owns the app-rule entry collection, filtering, list-model sync,
//! and the per-rule editor panel state.

use std::collections::{BTreeMap, BTreeSet};

use panopticon::settings::{AppSettings, ThumbnailRefreshMode};
use panopticon::ui_option_ops::{app_option_label, parse_option_value, OPTION_SEPARATOR};
use slint::SharedString;

use crate::{AppState, SettingsWindow};
use super::settings_helpers::{build_string_model, selected_model_value};

/// Build the filtered app-rule list model and sync it to the settings window.
pub(super) fn populate_app_rules_list(
    window: &SettingsWindow,
    state: &AppState,
    runtime: &super::RuntimeUiOptions,
) {
    let app_rule_entries = collect_app_rule_entries(state, runtime);

    let previous_app_rule_selection = selected_model_value(
        &window.get_app_rules_options(),
        window.get_app_rules_index(),
    )
    .and_then(|value| parse_option_value(&value));

    let app_rule_search = window.get_app_rules_search().trim().to_ascii_lowercase();
    let app_rule_filter = window.get_app_rules_filter_index();
    let filtered_app_rule_entries =
        filter_app_rule_entries(app_rule_entries, app_rule_filter, app_rule_search.as_str());

    let mut app_rule_options =
        vec![panopticon::i18n::t("settings.app_rules.select_option").to_owned()];
    app_rule_options.extend(
        filtered_app_rule_entries
            .iter()
            .map(|entry| app_option_label(&entry.option)),
    );
    let app_rule_index = previous_app_rule_selection
        .as_deref()
        .and_then(|selected| {
            filtered_app_rule_entries
                .iter()
                .position(|entry| entry.option.app_id == selected)
        })
        .map_or(0, |index| index as i32 + 1);
    window.set_app_rules_options(build_string_model(app_rule_options));
    window.set_app_rules_index(app_rule_index);

    let running_app_ids: BTreeSet<&str> =
        runtime.apps.iter().map(|app| app.app_id.as_str()).collect();
    let inactive_rule_count = state
        .settings
        .app_rules
        .keys()
        .filter(|app_id| !running_app_ids.contains(app_id.as_str()))
        .count();
    let cleanup_summary = if inactive_rule_count == 0 {
        panopticon::i18n::t("settings.app_rules.cleanup.none").to_owned()
    } else {
        panopticon::i18n::t_fmt(
            "settings.app_rules.cleanup.count",
            &inactive_rule_count.to_string(),
        )
    };
    window.set_app_rules_can_clear_unused(inactive_rule_count > 0);
    window.set_app_rules_unused_summary(SharedString::from(cleanup_summary));
}

fn collect_app_rule_entries(
    state: &AppState,
    runtime: &super::RuntimeUiOptions,
) -> Vec<super::AppRuleListEntry> {
    let mut by_id: BTreeMap<String, String> = BTreeMap::new();

    for app in &runtime.apps {
        by_id.insert(app.app_id.clone(), app.label.clone());
    }

    for (app_id, rule) in &state.settings.app_rules {
        if app_id.trim().is_empty() {
            continue;
        }
        by_id.entry(app_id.clone()).or_insert_with(|| {
            if rule.display_name.trim().is_empty() {
                app_id.clone()
            } else {
                rule.display_name.clone()
            }
        });
    }

    let mut entries: Vec<super::AppRuleListEntry> = by_id
        .into_iter()
        .map(|(app_id, label)| {
            let rule = state.settings.app_rules.get(&app_id);
            let tags = state.settings.tags_for(&app_id);
            let searchable_blob = format!(
                "{} {} {}",
                label.to_ascii_lowercase(),
                app_id.to_ascii_lowercase(),
                tags.join(" ").to_ascii_lowercase()
            );

            super::AppRuleListEntry {
                option: panopticon::settings::AppSelectionEntry {
                    app_id: app_id.clone(),
                    label,
                },
                is_running: runtime.apps.iter().any(|app| app.app_id == app_id),
                has_saved_rule: rule.is_some(),
                is_hidden: rule.is_some_and(|saved| saved.hidden),
                has_tags: !tags.is_empty(),
                has_custom_refresh: rule.is_some_and(|saved| {
                    matches!(
                        saved.thumbnail_refresh_mode,
                        ThumbnailRefreshMode::Frozen | ThumbnailRefreshMode::Interval
                    )
                }),
                is_pinned: rule.is_some_and(|saved| saved.pinned_position.is_some()),
                searchable_blob,
            }
        })
        .collect();

    entries.sort_by(|left, right| {
        left.option
            .label
            .to_ascii_lowercase()
            .cmp(&right.option.label.to_ascii_lowercase())
            .then_with(|| left.option.app_id.cmp(&right.option.app_id))
    });
    entries
}

fn filter_app_rule_entries(
    entries: Vec<super::AppRuleListEntry>,
    filter_index: i32,
    search_query: &str,
) -> Vec<super::AppRuleListEntry> {
    entries
        .into_iter()
        .filter(|entry| {
            let matches_filter = match filter_index {
                1 => entry.is_running,
                2 => entry.has_saved_rule,
                3 => entry.is_hidden,
                4 => entry.has_tags,
                5 => entry.has_custom_refresh,
                6 => entry.is_pinned,
                _ => true,
            };

            if !matches_filter {
                return false;
            }

            if search_query.is_empty() {
                return true;
            }

            entry.searchable_blob.contains(search_query)
        })
        .collect()
}

/// Sync the app-rule editor panel to the currently selected rule.
pub(super) fn sync_selected_app_rule_editor(window: &SettingsWindow, settings: &AppSettings) {
    let selected = selected_model_value(
        &window.get_app_rules_options(),
        window.get_app_rules_index(),
    );
    let Some(selected_option) = selected else {
        clear_app_rule_editor(window);
        return;
    };
    let Some(app_id) = parse_option_value(&selected_option) else {
        clear_app_rule_editor(window);
        return;
    };

    let label = selected_option
        .split_once(OPTION_SEPARATOR)
        .map_or_else(|| app_id.clone(), |(display, _)| display.trim().to_owned());
    let tags = settings.tags_for(&app_id).join(", ");
    let tags_vec = settings.tags_for(&app_id);
    let color_hex = settings.app_color_hex(&app_id).unwrap_or_default();
    let selected_slot = settings.pinned_position_for(&app_id);
    let pinned_slot = selected_slot.map_or(0, |slot| {
        i32::try_from(slot.saturating_add(1)).unwrap_or(i32::MAX)
    });
    let conflict_summary = settings
        .pinned_slot_conflicts()
        .into_iter()
        .find(|(slot, labels)| {
            selected_slot.is_some_and(|current| current == *slot) && labels.len() > 1
        })
        .map_or_else(String::new, |(slot, labels)| {
            format!("Slot #{} is shared by {}", slot + 1, labels.join(", "))
        });

    window.set_app_rules_has_selection(true);
    window.set_app_rules_selected_app_label(SharedString::from(label));
    window.set_app_rules_hidden(settings.is_hidden(&app_id));
    window.set_app_rules_preserve_aspect(settings.preserve_aspect_ratio_for(&app_id));
    window.set_app_rules_hide_on_select(settings.hide_on_select_for(&app_id));
    window.set_app_rules_refresh_mode_index(refresh_mode_to_index(
        settings.thumbnail_refresh_mode_for(&app_id),
    ));
    window.set_app_rules_refresh_interval_ms(
        settings.thumbnail_refresh_interval_ms_for(&app_id) as i32,
    );
    window.set_app_rules_tags(SharedString::from(tags));
    sync_app_rule_tags_editor(window, &tags_vec, false);
    window.set_app_rules_color_hex(SharedString::from(color_hex));
    window.set_app_rules_pin_slot(pinned_slot);
    window.set_app_rules_pin_conflict_summary(SharedString::from(conflict_summary));
}

fn clear_app_rule_editor(window: &SettingsWindow) {
    window.set_app_rules_has_selection(false);
    window.set_app_rules_selected_app_label(SharedString::from(""));
    window.set_app_rules_hidden(false);
    window.set_app_rules_preserve_aspect(false);
    window.set_app_rules_hide_on_select(false);
    window.set_app_rules_refresh_mode_index(0);
    window.set_app_rules_refresh_interval_ms(5_000);
    window.set_app_rules_tags(SharedString::from(""));
    window.set_app_rules_tag_chips(build_string_model(Vec::new()));
    window.set_app_rules_tag_input(SharedString::from(""));
    window.set_app_rules_color_hex(SharedString::from(""));
    window.set_app_rules_pin_slot(0);
    window.set_app_rules_pin_conflict_summary(SharedString::from(""));
}

/// Sync the tag-chips model and tags text in the app-rule editor.
pub(super) fn sync_app_rule_tags_editor(
    window: &SettingsWindow,
    tags: &[String],
    clear_input: bool,
) {
    let chips: Vec<String> = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();

    window.set_app_rules_tag_chips(build_string_model(chips.clone()));
    window.set_app_rules_tags(SharedString::from(chips.join(", ")));
    if clear_input {
        window.set_app_rules_tag_input(SharedString::from(""));
    }
}

fn refresh_mode_to_index(mode: ThumbnailRefreshMode) -> i32 {
    match mode {
        ThumbnailRefreshMode::Realtime => 0,
        ThumbnailRefreshMode::Frozen => 1,
        ThumbnailRefreshMode::Interval => 2,
    }
}

/// Convert a refresh-mode dropdown index back to the domain type.
pub(super) fn refresh_mode_from_index(index: i32) -> ThumbnailRefreshMode {
    match index {
        1 => ThumbnailRefreshMode::Frozen,
        2 => ThumbnailRefreshMode::Interval,
        _ => ThumbnailRefreshMode::Realtime,
    }
}

/// Parse a comma-separated tag string into normalized, sorted, deduplicated tags.
pub(super) fn parse_tags_csv(raw: &str) -> Vec<String> {
    let mut tags: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::parse_tags_csv;

    #[test]
    fn parse_tags_csv_normalizes_sorts_and_deduplicates() {
        assert_eq!(
            parse_tags_csv(" Work ,alpha,work, Beta ,,ALPHA "),
            vec!["alpha".to_owned(), "beta".to_owned(), "work".to_owned()]
        );
    }
}
