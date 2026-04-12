//! Secondary Slint windows: settings, tag dialog, and profile helpers.

use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use panopticon::settings::{AppSelectionEntry, AppSettings, HiddenAppEntry};
use panopticon::theme as theme_catalog;
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use slint::{CloseRequestResponse, ComponentHandle, Model, ModelRc, SharedString, VecModel};

use super::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, center_window_on_screen,
    keep_dialog_above_owner, reposition_appbar, restore_floating_style, unregister_appbar,
};
use super::settings_ui::{apply_settings_window_changes, populate_settings_window};
use super::theme_ui::{apply_settings_window_theme_snapshot, apply_tag_dialog_theme_snapshot};
use super::tray::apply_window_icons;
use crate::{AppState, MainWindow, SettingsWindow, TagDialogWindow};

struct RuntimeUiOptions {
    monitors: Vec<String>,
    tags: Vec<String>,
    apps: Vec<AppSelectionEntry>,
    hidden_apps: Vec<HiddenAppEntry>,
}

pub(crate) fn ensure_default_profiles_exist(settings: &AppSettings) {
    match AppSettings::list_profiles() {
        Ok(profiles) if profiles.is_empty() => {
            for profile_name in ["profile-1", "profile-2"] {
                if let Err(error) = settings.save(Some(profile_name)) {
                    tracing::error!(%error, profile = profile_name, "failed to seed default profile");
                }
            }
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "failed to inspect saved profiles"),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "coordinates the SettingsWindow lifecycle and its callback wiring in one place"
)]
pub(crate) fn open_settings_window(
    state: &Rc<RefCell<AppState>>,
    main_weak: &slint::Weak<MainWindow>,
) {
    let already_open = crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                apply_window_icons(hwnd, &state.icons);
                keep_dialog_above_owner(hwnd, state.hwnd, &state.settings);
                center_window_on_screen(hwnd);
            }
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let settings_window = match SettingsWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create settings window");
            return;
        }
    };
    crate::populate_tr_global(&settings_window);

    {
        let state = state.borrow();
        populate_settings_window(&settings_window, &state.settings);
        populate_settings_window_runtime_fields(&settings_window, &state);
        apply_settings_window_theme_snapshot(&settings_window, &state.current_theme);
    }

    settings_window.on_save_profile({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let requested = panopticon::settings::normalize_profile_name(
                    &settings_window.get_profile_name(),
                );
                let Some(profile_name) = requested else {
                    tracing::warn!("ignoring empty/invalid profile save request");
                    return;
                };

                let settings_snapshot = state.borrow().settings.normalized();
                if save_settings_as_profile(&settings_snapshot, &profile_name) {
                    settings_window
                        .set_known_profiles_label(SharedString::from(known_profiles_label()));
                }
            });
        }
    });

    settings_window.on_open_profile_instance({
        let state = state.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };

                let current_profile = state.borrow().profile_name.clone();
                let requested = panopticon::settings::normalize_profile_name(
                    &settings_window.get_profile_name(),
                )
                .or(current_profile);

                let settings_snapshot = state.borrow().settings.normalized();
                if let Some(profile_name) = requested.as_deref() {
                    let _ = save_settings_as_profile(&settings_snapshot, profile_name);
                } else if let Err(error) = settings_snapshot.save(None) {
                    tracing::error!(%error, "failed to save default profile before launching instance");
                }

                let _ = launch_additional_instance(requested.as_deref());
                settings_window.set_known_profiles_label(SharedString::from(known_profiles_label()));
            });
        }
    });

    settings_window.on_reset_to_defaults({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            {
                let mut state = state.borrow_mut();
                let profile = state.profile_name.clone();
                state.settings = AppSettings::default();
                state.settings = state.settings.normalized();
                state.current_layout = state.settings.initial_layout;
                let _ = state.settings.save(profile.as_deref());
            }
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                if let Some(settings_window) = guard.as_ref() {
                    let state_ref = state.borrow();
                    populate_settings_window(settings_window, &state_ref.settings);
                    populate_settings_window_runtime_fields(settings_window, &state_ref);
                    apply_settings_window_theme_snapshot(settings_window, &state_ref.current_theme);
                }
            });
            let state_ref = state.borrow();
            apply_window_appearance(state_ref.hwnd, &state_ref.settings);
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            let _ = crate::refresh_windows(&state);
            if let Some(main_window) = main_weak.upgrade() {
                crate::recompute_and_update_ui(&state, &main_window);
            }
        }
    });

    settings_window.on_refresh_now({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });

    settings_window.on_restore_hidden_selected({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let Some(option) = selected_model_value(
                    &settings_window.get_hidden_app_options(),
                    settings_window.get_hidden_app_index(),
                ) else {
                    return;
                };
                let Some(app_id) = parse_option_value(&option) else {
                    return;
                };

                crate::update_settings(&state, |settings| {
                    let _ = settings.restore_hidden_app(&app_id);
                });
                let _ = crate::refresh_windows(&state);
                crate::refresh_ui(&state, &main_weak);
            });
        }
    });

    settings_window.on_restore_hidden_all({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::update_settings(&state, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            let _ = crate::refresh_windows(&state);
            crate::refresh_ui(&state, &main_weak);
        }
    });

    settings_window.on_apply({
        let state = state.clone();
        let main_weak = main_weak.clone();
        move || {
            crate::SETTINGS_WIN.with(|handle| {
                let guard = handle.borrow();
                let Some(settings_window) = guard.as_ref() else {
                    return;
                };
                let mut state_guard = state.borrow_mut();
                let prev_dock_edge = state_guard.settings.dock_edge;
                let layout =
                    apply_settings_window_changes(settings_window, &mut state_guard.settings);
                apply_runtime_settings_window_changes(settings_window, &mut state_guard.settings);
                state_guard.current_layout = layout;
                state_guard.settings = state_guard.settings.normalized();
                let _ = state_guard
                    .settings
                    .save(state_guard.profile_name.as_deref());
                let hwnd = state_guard.hwnd;
                let always_on_top = state_guard.settings.always_on_top;
                let new_dock_edge = state_guard.settings.dock_edge;
                let settings_clone = state_guard.settings.clone();
                let profile_name = state_guard.profile_name.clone();

                if prev_dock_edge != new_dock_edge {
                    if state_guard.is_appbar {
                        unregister_appbar(hwnd);
                        state_guard.is_appbar = false;
                    }
                    if new_dock_edge.is_some() {
                        apply_dock_mode(&mut state_guard);
                    } else {
                        restore_floating_style(hwnd);
                    }
                } else if state_guard.is_appbar {
                    reposition_appbar(&mut state_guard);
                }

                drop(state_guard);
                let _ = crate::refresh_windows(&state);
                apply_window_appearance(hwnd, &settings_clone);
                apply_topmost_mode(hwnd, always_on_top);
                settings_window
                    .set_known_profiles_label(SharedString::from(known_profiles_label()));
                settings_window.set_current_profile_label(SharedString::from(
                    current_profile_label(profile_name.as_deref()),
                ));
                {
                    let refreshed = state.borrow();
                    populate_settings_window(settings_window, &refreshed.settings);
                    populate_settings_window_runtime_fields(settings_window, &refreshed);
                    apply_settings_window_theme_snapshot(settings_window, &refreshed.current_theme);
                }
                if let Some(main_window) = main_weak.upgrade() {
                    crate::recompute_and_update_ui(&state, &main_window);
                }

                crate::TAG_DIALOG_WIN.with(|dialog| {
                    if let Some(dialog) = dialog.borrow().as_ref() {
                        if let Some(dialog_hwnd) = crate::get_hwnd(dialog.window()) {
                            keep_dialog_above_owner(dialog_hwnd, hwnd, &settings_clone);
                        }
                    }
                });
            });
        }
    });

    settings_window.on_closed(|| {
        let taken = crate::SETTINGS_WIN.with(|handle| handle.borrow_mut().take());
        if let Some(window) = taken {
            window.hide().ok();
        }
    });

    if let Err(error) = settings_window.show() {
        tracing::error!(%error, "failed to show settings window");
        return;
    }
    if let Some(settings_hwnd) = crate::get_hwnd(settings_window.window()) {
        let state = state.borrow();
        apply_window_icons(settings_hwnd, &state.icons);
        apply_window_appearance(settings_hwnd, &state.settings);
        apply_settings_window_theme_snapshot(&settings_window, &state.current_theme);
        keep_dialog_above_owner(settings_hwnd, state.hwnd, &state.settings);
        center_window_on_screen(settings_hwnd);
    }
    crate::SETTINGS_WIN.with(|handle| *handle.borrow_mut() = Some(settings_window));
}

pub(crate) fn refresh_open_settings_window(state: &Rc<RefCell<AppState>>) {
    crate::SETTINGS_WIN.with(|handle| {
        let guard = handle.borrow();
        let Some(window) = guard.as_ref() else {
            return;
        };
        let state = state.borrow();
        populate_settings_window(window, &state.settings);
        populate_settings_window_runtime_fields(window, &state);
        apply_settings_window_theme_snapshot(window, &state.current_theme);
        if let Some(dialog_hwnd) = crate::get_hwnd(window.window()) {
            keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
        }
    });
}

pub(crate) fn open_create_tag_dialog(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
) {
    let already_open = crate::TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            if let Some(dialog_hwnd) = crate::get_hwnd(existing.window()) {
                let state = state.borrow();
                apply_window_icons(dialog_hwnd, &state.icons);
                keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
            }
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let suggested_name = suggested_tag_name(&info.app_label());
    let suggested_color = state.borrow().settings.tag_color_hex(&suggested_name);

    let dialog = match TagDialogWindow::new() {
        Ok(dialog) => dialog,
        Err(error) => {
            tracing::error!(%error, app_id = %info.app_id, "failed to create tag dialog");
            return;
        }
    };
    crate::populate_tr_global(&dialog);

    dialog.set_app_label(SharedString::from(info.app_label()));
    dialog.set_tag_name(SharedString::from(suggested_name));
    dialog.set_color_index(tag_color_index(&suggested_color));
    {
        let state = state.borrow();
        apply_tag_dialog_theme_snapshot(&dialog, &state.current_theme);
    }

    dialog.on_create({
        let state = state.clone();
        let weak = weak.clone();
        let app_id = info.app_id.clone();
        let display_name = info.app_label();
        move || {
            crate::TAG_DIALOG_WIN.with(|dialog_cell| {
                let guard = dialog_cell.borrow();
                let Some(dialog) = guard.as_ref() else {
                    return;
                };
                let tag_name = dialog.get_tag_name().to_string();
                let color_hex = tag_color_hex(dialog.get_color_index());
                drop(guard);

                apply_tag_creation(&state, &weak, &app_id, &display_name, &tag_name, &color_hex);
                close_tag_dialog_window();
            });
        }
    });

    dialog.on_closed(close_tag_dialog_window);

    dialog.window().on_close_requested(|| {
        close_tag_dialog_window();
        CloseRequestResponse::HideWindow
    });

    if let Err(error) = dialog.show() {
        tracing::error!(%error, app_id = %info.app_id, "failed to show tag dialog");
        return;
    }

    if let Some(dialog_hwnd) = crate::get_hwnd(dialog.window()) {
        let state = state.borrow();
        apply_window_icons(dialog_hwnd, &state.icons);
        apply_window_appearance(dialog_hwnd, &state.settings);
        apply_tag_dialog_theme_snapshot(&dialog, &state.current_theme);
        keep_dialog_above_owner(dialog_hwnd, state.hwnd, &state.settings);
    }

    crate::TAG_DIALOG_WIN.with(|dialog_cell| *dialog_cell.borrow_mut() = Some(dialog));
}

fn apply_runtime_settings_window_changes(window: &SettingsWindow, settings: &mut AppSettings) {
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

fn populate_settings_window_runtime_fields(window: &SettingsWindow, state: &AppState) {
    let runtime = collect_runtime_ui_options(state);
    window.set_theme_options(build_string_model(theme_catalog::theme_labels()));
    window.set_current_profile_label(SharedString::from(current_profile_label(
        state.profile_name.as_deref(),
    )));
    window.set_profile_name(SharedString::from(
        state.profile_name.clone().unwrap_or_default(),
    ));
    window.set_known_profiles_label(SharedString::from(known_profiles_label()));

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

fn collect_runtime_ui_options(state: &AppState) -> RuntimeUiOptions {
    let windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();

    RuntimeUiOptions {
        monitors: crate::collect_available_monitors(&windows),
        tags: state.settings.known_tags(),
        apps: crate::collect_available_apps(&windows),
        hidden_apps: state.settings.hidden_app_entries(),
    }
}

fn current_profile_label(profile_name: Option<&str>) -> String {
    profile_name.unwrap_or("default").to_owned()
}

fn known_profiles_label() -> String {
    use panopticon::i18n;
    match AppSettings::list_profiles() {
        Ok(profiles) if profiles.is_empty() => i18n::t("settings.saved_profiles").to_owned(),
        Ok(profiles) => i18n::t_fmt("settings.saved_profiles_fmt", &profiles.join(", ")),
        Err(error) => {
            tracing::warn!(%error, "failed to list saved profiles");
            i18n::t("settings.saved_profiles").to_owned()
        }
    }
}

fn build_string_model(values: Vec<String>) -> ModelRc<SharedString> {
    let values: Vec<SharedString> = values.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(values))
}

fn selected_model_value(model: &ModelRc<SharedString>, index: i32) -> Option<String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| model.row_data(index))
        .map(|value| value.to_string())
}

fn app_option_label(app: &AppSelectionEntry) -> String {
    format!("{}{}{}", app.label, crate::OPTION_SEPARATOR, app.app_id)
}

fn hidden_app_option_label(app: &HiddenAppEntry) -> String {
    format!("{}{}{}", app.label, crate::OPTION_SEPARATOR, app.app_id)
}

fn parse_option_value(value: &str) -> Option<String> {
    value
        .rsplit_once(crate::OPTION_SEPARATOR)
        .map(|(_, raw)| raw.trim().to_owned())
        .filter(|raw| !raw.is_empty())
}

fn save_settings_as_profile(settings: &AppSettings, profile_name: &str) -> bool {
    match settings.save(Some(profile_name)) {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, profile = profile_name, "failed to save profile");
            false
        }
    }
}

fn launch_additional_instance(profile_name: Option<&str>) -> bool {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            tracing::error!(%error, "failed to resolve executable path for new instance");
            return false;
        }
    };

    let mut command = Command::new(executable);
    if let Some(profile_name) = profile_name {
        command.arg("--profile").arg(profile_name);
    }

    match command.spawn() {
        Ok(_) => true,
        Err(error) => {
            tracing::error!(%error, profile = ?profile_name, "failed to launch extra instance");
            false
        }
    }
}

fn suggested_tag_name(label: &str) -> String {
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

fn apply_tag_creation(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    app_id: &str,
    display_name: &str,
    tag_name: &str,
    color_hex: &str,
) {
    crate::update_settings(state, |settings| {
        let _ = settings.assign_tag_with_color(app_id, display_name, tag_name, color_hex);
    });
    let _ = crate::refresh_windows(state);
    crate::refresh_ui(state, weak);
}

fn close_tag_dialog_window() {
    let taken = crate::TAG_DIALOG_WIN.with(|dialog| dialog.borrow_mut().take());
    if let Some(dialog) = taken {
        dialog.hide().ok();
    }
}

fn tag_color_index(color_hex: &str) -> i32 {
    match color_hex.to_ascii_uppercase().as_str() {
        "5CA9FF" => 1,
        "3CCF91" => 2,
        "FF6B8A" => 3,
        "9B7BFF" => 4,
        "F4B740" => 5,
        _ => 0,
    }
}

fn tag_color_hex(index: i32) -> String {
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
