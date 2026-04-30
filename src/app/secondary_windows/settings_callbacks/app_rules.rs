use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use panopticon::settings::ThumbnailRefreshMode;
use panopticon::ui_option_ops::parse_option_value;

use crate::{AppState, MainWindow, SettingsWindow};
use crate::app::runtime_support::{refresh_ui, update_settings};
use crate::app::window_sync::refresh_windows;

use super::super::{
    collect_runtime_ui_options, parse_tags_csv, populate_settings_window_runtime_fields,
    refresh_mode_from_index, selected_model_value, sync_app_rule_tags_editor,
    sync_selected_app_rule_editor,
};

pub(super) fn register_app_rule_callbacks(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    register_app_rules_select_app_callback(settings_window, state);
    register_app_rules_refresh_list_callback(settings_window, state);
    register_app_rules_apply_selected_callback(settings_window, state, main_weak);
    register_app_rules_reset_selected_callback(settings_window, state, main_weak);
    register_app_rules_clear_unused_callback(settings_window, state, main_weak);
    register_app_rules_add_tag_callback(settings_window);
    register_app_rules_remove_tag_callback(settings_window);
    register_app_rules_apply_tag_suggestion_callback(settings_window);
}

fn register_app_rules_select_app_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_app_rules_select_app({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let state_guard = state.borrow();
                sync_selected_app_rule_editor(settings_window, &state_guard.settings);
            });
        }
    });
}

fn register_app_rules_refresh_list_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
) {
    settings_window.on_app_rules_refresh_list({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let state_guard = state.borrow();
                settings_window.set_suspend_live_apply(true);
                populate_settings_window_runtime_fields(settings_window, &state_guard);
                settings_window.set_suspend_live_apply(false);
            });
        }
    });
}

fn register_app_rules_apply_selected_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_app_rules_apply_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let selected = selected_model_value(
                    &settings_window.get_app_rules_options(),
                    settings_window.get_app_rules_index(),
                );
                let Some(selected_option) = selected else {
                    return;
                };
                let Some(app_id) = parse_option_value(&selected_option) else {
                    return;
                };

                let display_name = settings_window
                    .get_app_rules_selected_app_label()
                    .to_string();
                let hidden = settings_window.get_app_rules_hidden();
                let preserve_aspect = settings_window.get_app_rules_preserve_aspect();
                let hide_on_select = settings_window.get_app_rules_hide_on_select();
                let refresh_mode =
                    refresh_mode_from_index(settings_window.get_app_rules_refresh_mode_index());
                let refresh_interval_ms = settings_window
                    .get_app_rules_refresh_interval_ms()
                    .clamp(500, 60_000) as u32;
                let tags_csv = settings_window.get_app_rules_tags().to_string();
                let color_hex = settings_window.get_app_rules_color_hex().to_string();
                let pin_slot_value = settings_window.get_app_rules_pin_slot().max(0) as usize;

                update_settings(&state, |settings| {
                    let default_preserve = settings.preserve_aspect_ratio;
                    let default_hide = settings.hide_on_select;
                    let rule = settings.app_rules.entry(app_id.clone()).or_default();

                    if !display_name.trim().is_empty() {
                        display_name.trim().clone_into(&mut rule.display_name);
                    }

                    rule.hidden = hidden;
                    rule.preserve_aspect_ratio = preserve_aspect;
                    rule.preserve_aspect_ratio_override =
                        (preserve_aspect != default_preserve).then_some(preserve_aspect);

                    let effective_hide = if settings.dock_edge.is_some() {
                        false
                    } else {
                        hide_on_select
                    };
                    rule.hide_on_select = effective_hide;
                    rule.hide_on_select_override =
                        (effective_hide != default_hide).then_some(effective_hide);

                    rule.thumbnail_refresh_mode = refresh_mode;
                    rule.thumbnail_refresh_interval_ms = (refresh_mode
                        == ThumbnailRefreshMode::Interval)
                        .then_some(refresh_interval_ms.max(500));

                    rule.tags = parse_tags_csv(&tags_csv);

                    let color = color_hex.trim().trim_start_matches('#');
                    rule.color_hex = if color.is_empty() {
                        None
                    } else {
                        Some(color.to_owned())
                    };

                    rule.pinned_position = if hidden || pin_slot_value == 0 {
                        None
                    } else {
                        Some(pin_slot_value - 1)
                    };
                });

                let _ = refresh_windows(&state);
                refresh_ui(&state, &main_weak);
            });
        }
    });
}

fn register_app_rules_reset_selected_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_app_rules_reset_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let selected = selected_model_value(
                    &settings_window.get_app_rules_options(),
                    settings_window.get_app_rules_index(),
                );
                let Some(selected_option) = selected else {
                    return;
                };
                let Some(app_id) = parse_option_value(&selected_option) else {
                    return;
                };

                update_settings(&state, |settings| {
                    settings.app_rules.remove(&app_id);
                });

                let _ = refresh_windows(&state);
                refresh_ui(&state, &main_weak);
            });
        }
    });
}

fn register_app_rules_clear_unused_callback(
    settings_window: &SettingsWindow,
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    settings_window.on_app_rules_clear_unused({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let running_app_ids: BTreeSet<String> = {
                let state_guard = state.borrow();
                collect_runtime_ui_options(&state_guard)
                    .apps
                    .into_iter()
                    .map(|entry| entry.app_id)
                    .collect()
            };

            update_settings(&state, |settings| {
                settings
                    .app_rules
                    .retain(|app_id, _| running_app_ids.contains(app_id));
            });

            let _ = refresh_windows(&state);
            refresh_ui(&state, &main_weak);
        }
    });
}

fn register_app_rules_add_tag_callback(settings_window: &SettingsWindow) {
    settings_window.on_app_rules_add_tag(|| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
            let draft = settings_window
                .get_app_rules_tag_input()
                .trim()
                .to_ascii_lowercase();
            if draft.is_empty() {
                return;
            }

            tags.push(draft);
            tags.sort();
            tags.dedup();
            sync_app_rule_tags_editor(settings_window, &tags, true);
        });
    });
}

fn register_app_rules_remove_tag_callback(settings_window: &SettingsWindow) {
    settings_window.on_app_rules_remove_tag(|tag| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
            tags.retain(|candidate| candidate != tag.as_str());
            sync_app_rule_tags_editor(settings_window, &tags, false);
        });
    });
}

fn register_app_rules_apply_tag_suggestion_callback(settings_window: &SettingsWindow) {
    settings_window.on_app_rules_apply_tag_suggestion(|suggestion| {
        crate::SETTINGS_WIN.with(|handle| {
            let guard = handle.borrow();
            let Some(settings_window) = guard.as_ref() else {
                return;
            };

            let mut tags = parse_tags_csv(settings_window.get_app_rules_tags().as_ref());
            let normalized = suggestion.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                return;
            }

            tags.push(normalized);
            tags.sort();
            tags.dedup();
            sync_app_rule_tags_editor(settings_window, &tags, false);
        });
    });
}
