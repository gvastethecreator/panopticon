use std::collections::{BTreeMap, BTreeSet};

use panopticon::settings::{
    AppSettings, ThumbnailRefreshMode, MIN_DOCK_COLUMN_THICKNESS, MIN_DOCK_ROW_THICKNESS,
    MIN_FIXED_WINDOW_HEIGHT, MIN_FIXED_WINDOW_WIDTH,
};
use panopticon::theme as theme_catalog;
use panopticon::ui_option_ops::{
    app_option_label, current_workspace_label, hidden_app_option_label, parse_option_value,
    OPTION_SEPARATOR,
};
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{collect_available_apps, collect_available_monitors};
use slint::SharedString;

use crate::{AppState, SettingsWindow};

use super::super::settings_ui::populate_settings_window;
use super::super::theme_ui::apply_settings_window_theme_snapshot;
use super::{
    available_workspace_summaries, build_string_model, known_workspaces_label,
    selected_model_value, selected_workspace_from_settings_window, sync_layout_preset_controls,
    AppRuleListEntry, RuntimeUiOptions,
};

pub(super) fn apply_runtime_settings_window_changes(
    window: &SettingsWindow,
    settings: &mut AppSettings,
) {
    let monitor = selected_model_value(
        &window.get_monitor_filter_options(),
        window.get_monitor_filter_index(),
    );
    settings.set_monitor_filter(
        monitor
            .as_deref()
            .filter(|value| *value != panopticon::i18n::t("tray.all_monitors")),
    );

    let tag = selected_model_value(
        &window.get_tag_filter_options(),
        window.get_tag_filter_index(),
    );
    settings.set_tag_filter(
        tag.as_deref()
            .filter(|value| *value != panopticon::i18n::t("tray.all_tags")),
    );

    let app = selected_model_value(
        &window.get_app_filter_options(),
        window.get_app_filter_index(),
    )
    .and_then(|value| parse_option_value(&value));

    if let Some(app) = app.as_deref() {
        settings.set_tag_filter(None);
        settings.set_app_filter(Some(app));
    } else {
        settings.set_app_filter(None);
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "runtime settings population intentionally centralizes all combo/model synchronization"
)]
pub(super) fn populate_settings_window_runtime_fields(window: &SettingsWindow, state: &AppState) {
    let runtime = collect_runtime_ui_options(state);
    let app_rule_entries = collect_app_rule_entries(state, &runtime);
    let workspace_summaries = available_workspace_summaries(state, &runtime);
    let workspace_options: Vec<String> = workspace_summaries
        .iter()
        .map(|summary| summary.option_label.clone())
        .collect();
    let fallback_fixed_width = u32::try_from(state.last_size.0)
        .ok()
        .filter(|value| *value > 0)
        .map_or(MIN_FIXED_WINDOW_WIDTH, |value| {
            value.max(MIN_FIXED_WINDOW_WIDTH)
        });
    let fallback_fixed_height = u32::try_from(state.last_size.1)
        .ok()
        .filter(|value| *value > 0)
        .map_or(MIN_FIXED_WINDOW_HEIGHT, |value| {
            value.max(MIN_FIXED_WINDOW_HEIGHT)
        });

    window.set_theme_options(build_string_model(theme_catalog::theme_labels()));
    window.set_app_version_text(SharedString::from(state.app_version.clone()));
    window.set_update_status_text(SharedString::from(localized_update_status_text(
        &state.update_status,
    )));
    window.set_update_check_running(matches!(state.update_status, crate::UpdateStatus::Checking));
    window.set_fixed_width_value(
        state
            .settings
            .fixed_width
            .unwrap_or(fallback_fixed_width)
            .max(MIN_FIXED_WINDOW_WIDTH) as i32,
    );
    window.set_fixed_height_value(
        state
            .settings
            .fixed_height
            .unwrap_or(fallback_fixed_height)
            .max(MIN_FIXED_WINDOW_HEIGHT) as i32,
    );
    window.set_dock_column_thickness_value(
        state
            .settings
            .dock_column_thickness
            .unwrap_or(MIN_DOCK_COLUMN_THICKNESS)
            .max(MIN_DOCK_COLUMN_THICKNESS) as i32,
    );
    window.set_dock_row_thickness_value(
        state
            .settings
            .dock_row_thickness
            .unwrap_or(MIN_DOCK_ROW_THICKNESS)
            .max(MIN_DOCK_ROW_THICKNESS) as i32,
    );
    window.set_current_profile_label(SharedString::from(current_workspace_label(
        state.workspace_name.as_deref(),
    )));
    window.set_profile_name(SharedString::from(
        state.workspace_name.clone().unwrap_or_default(),
    ));
    window.set_profile_display_name(SharedString::from(
        state.settings.workspace.display_name.clone(),
    ));
    window.set_profile_description(SharedString::from(
        state.settings.workspace.description.clone(),
    ));
    window.set_known_profiles_label(SharedString::from(known_workspaces_label()));
    let requested_workspace =
        selected_workspace_from_settings_window(window).or_else(|| state.workspace_name.clone());
    let profile_index = workspace_summaries
        .iter()
        .position(|summary| summary.workspace_name == requested_workspace)
        .or_else(|| {
            workspace_summaries
                .iter()
                .position(|summary| summary.workspace_name.is_none())
        })
        .map_or(0, |index| index as i32);
    window.set_available_profile_options(build_string_model(workspace_options));
    window.set_available_profile_index(profile_index);

    if let Some(selected_summary) = workspace_summaries.get(profile_index as usize) {
        window.set_profile_name(SharedString::from(
            selected_summary.workspace_name.clone().unwrap_or_default(),
        ));
        window.set_profile_display_name(SharedString::from(selected_summary.display_name.clone()));
        window.set_profile_description(SharedString::from(selected_summary.description.clone()));
        window.set_selected_profile_name(SharedString::from(selected_summary.option_label.clone()));
        window.set_selected_profile_is_default(selected_summary.is_default);
        window.set_selected_profile_created_at(SharedString::from(
            selected_summary.created_at_label.clone(),
        ));
        window.set_selected_profile_updated_at(SharedString::from(
            selected_summary.updated_at_label.clone(),
        ));
        window.set_selected_profile_running(selected_summary.is_running);
        window.set_selected_profile_modified(selected_summary.is_modified);
        window.set_selected_profile_missing_apps(selected_summary.missing_apps as i32);
        window.set_selected_profile_status_summary(SharedString::from(
            selected_summary.status_summary.clone(),
        ));
    } else {
        window.set_selected_profile_name(SharedString::from("default"));
        window.set_selected_profile_is_default(true);
        window.set_selected_profile_created_at(SharedString::from(""));
        window.set_selected_profile_updated_at(SharedString::from(""));
        window.set_selected_profile_running(false);
        window.set_selected_profile_modified(false);
        window.set_selected_profile_missing_apps(0);
        window.set_selected_profile_status_summary(SharedString::from(""));
    }

    sync_layout_preset_controls(window, &state.settings);

    let mut monitor_options = vec![panopticon::i18n::t("tray.all_monitors").to_owned()];
    monitor_options.extend(runtime.monitors.iter().cloned());
    let monitor_index = state
        .settings
        .active_monitor_filter
        .as_deref()
        .and_then(|current| {
            runtime
                .monitors
                .iter()
                .position(|monitor| monitor == current)
        })
        .map_or(0, |index| index as i32 + 1);
    window.set_monitor_filter_options(build_string_model(monitor_options));
    window.set_monitor_filter_index(monitor_index);

    let mut tag_options = vec![panopticon::i18n::t("tray.all_tags").to_owned()];
    tag_options.extend(runtime.tags.iter().cloned());
    let tag_index = state
        .settings
        .active_tag_filter
        .as_deref()
        .and_then(|current| runtime.tags.iter().position(|tag| tag == current))
        .map_or(0, |index| index as i32 + 1);
    window.set_tag_filter_options(build_string_model(tag_options));
    window.set_tag_filter_index(tag_index);

    let mut app_options = vec![panopticon::i18n::t("tray.all_apps").to_owned()];
    app_options.extend(runtime.apps.iter().map(app_option_label));
    let app_index = state
        .settings
        .active_app_filter
        .as_deref()
        .and_then(|current| runtime.apps.iter().position(|app| app.app_id == current))
        .map_or(0, |index| index as i32 + 1);
    window.set_app_filter_options(build_string_model(app_options));
    window.set_app_filter_index(app_index);

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

    sync_selected_app_rule_editor(window, &state.settings);

    if runtime.hidden_apps.is_empty() {
        window.set_hidden_app_options(build_string_model(vec![panopticon::i18n::t(
            "settings.no_hidden",
        )
        .to_owned()]));
        window.set_hidden_app_index(0);
        window.set_can_restore_hidden(false);
        window.set_hidden_apps_summary(SharedString::from(panopticon::i18n::t(
            "settings.no_hidden",
        )));
    } else {
        let hidden_options: Vec<String> = runtime
            .hidden_apps
            .iter()
            .map(hidden_app_option_label)
            .collect();
        let summary = if runtime.hidden_apps.len() == 1 {
            panopticon::i18n::t("settings.hidden_one").to_owned()
        } else {
            panopticon::i18n::t_fmt(
                "settings.hidden_many",
                &runtime.hidden_apps.len().to_string(),
            )
        };
        window.set_hidden_app_options(build_string_model(hidden_options));
        window.set_hidden_app_index(0);
        window.set_can_restore_hidden(true);
        window.set_hidden_apps_summary(SharedString::from(summary));
    }
}

fn collect_app_rule_entries(state: &AppState, runtime: &RuntimeUiOptions) -> Vec<AppRuleListEntry> {
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

    let mut entries: Vec<AppRuleListEntry> = by_id
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

            AppRuleListEntry {
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
    entries: Vec<AppRuleListEntry>,
    filter_index: i32,
    search_query: &str,
) -> Vec<AppRuleListEntry> {
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
        settings.thumbnail_refresh_interval_ms_for(&app_id) as i32
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

pub(super) fn refresh_mode_from_index(index: i32) -> ThumbnailRefreshMode {
    match index {
        1 => ThumbnailRefreshMode::Frozen,
        2 => ThumbnailRefreshMode::Interval,
        _ => ThumbnailRefreshMode::Realtime,
    }
}

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

pub(super) fn sync_settings_window_from_state(window: &SettingsWindow, state: &AppState) {
    let draft_profile_name = window.get_profile_name();
    let draft_profile_display_name = window.get_profile_display_name();
    let draft_profile_description = window.get_profile_description();
    crate::populate_tr_global(window);
    window.set_suspend_live_apply(true);
    populate_settings_window(window, &state.settings);
    populate_settings_window_runtime_fields(window, state);
    let resolved_theme = theme_catalog::resolve_ui_theme(
        state.settings.theme_id.as_deref(),
        &state.settings.background_color_hex,
        &state.settings.theme_color_overrides,
    );
    apply_settings_window_theme_snapshot(window, &resolved_theme);
    if !draft_profile_name.is_empty() {
        window.set_profile_name(draft_profile_name);
    }
    if !draft_profile_display_name.is_empty() {
        window.set_profile_display_name(draft_profile_display_name);
    }
    if !draft_profile_description.is_empty() {
        window.set_profile_description(draft_profile_description);
    }
    window.set_suspend_live_apply(false);
}

pub(super) fn collect_runtime_ui_options(state: &AppState) -> RuntimeUiOptions {
    let windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();

    RuntimeUiOptions {
        monitors: collect_available_monitors(&windows),
        tags: state.settings.known_tags(),
        apps: collect_available_apps(&windows),
        hidden_apps: state.settings.hidden_app_entries(),
    }
}

fn localized_update_status_text(status: &crate::UpdateStatus) -> String {
    match status {
        crate::UpdateStatus::Idle => panopticon::i18n::t("settings.update_status.idle").to_owned(),
        crate::UpdateStatus::Checking => {
            panopticon::i18n::t("settings.update_status.checking").to_owned()
        }
        crate::UpdateStatus::UpToDate { latest_version } => {
            panopticon::i18n::t_fmt("settings.update_status.up_to_date", latest_version)
        }
        crate::UpdateStatus::Available { latest_version, .. } => {
            panopticon::i18n::t_fmt("settings.update_status.available", latest_version)
        }
        crate::UpdateStatus::Failed => {
            panopticon::i18n::t("settings.update_status.failed").to_owned()
        }
    }
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
