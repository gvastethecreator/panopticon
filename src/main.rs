#![windows_subsystem = "windows"]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::wildcard_imports
)]

//! Binary entry point for Panopticon — Slint UI with DWM thumbnail overlays.

mod app;
mod state;

// Re-export all public state types and thread-locals so that `crate::AppState`,
// `crate::UI_STATE`, etc. continue to resolve without changing every consumer.
pub(crate) use state::*;

pub(crate) use app::layout_actions::cycle_layout;
pub(crate) use app::model_sync::{
    advance_animation, recompute_and_update_ui, sync_model_to_slint, sync_settings_to_ui,
};
pub(crate) use app::native_runtime::get_hwnd;
pub(crate) use app::window_sync::refresh_windows;

use app::dock::reposition_appbar;
use app::dwm::{release_all_thumbnails, release_thumbnail, update_dwm_thumbnails};
use app::theme_ui::{advance_theme_animation, apply_main_window_theme_snapshot};
use app::tray::{AppIcons, INSTANCE_ACCENT_PALETTE};
use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use slint::{CloseRequestResponse, ComponentHandle, SharedString, Timer, TimerMode};

use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT, RECT};

use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

slint::include_modules!();

// ───────────────────────── Constants ─────────────────────────

/// Callback message posted by the shell when the app-bar needs repositioning.
pub(crate) const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

#[allow(
    clippy::too_many_lines,
    reason = "one central place wires translation keys into the generated Slint global setters"
)]
pub(crate) fn populate_tr_global<Component>(window: &Component)
where
    Component: ComponentHandle,
    for<'a> Tr<'a>: slint::Global<'a, Component>,
{
    use panopticon::i18n;

    macro_rules! set_tr {
        ($tr:ident, $setter:ident, $key:literal) => {
            $tr.$setter(SharedString::from(i18n::t($key)));
        };
    }

    let tr = window.global::<Tr>();
    set_tr!(tr, set_app_name, "app.name");
    set_tr!(tr, set_main_window_title, "window.main_title");
    set_tr!(tr, set_settings_window_title, "window.settings_title");
    set_tr!(tr, set_tag_window_title, "window.tag_title");
    set_tr!(tr, set_about_window_title, "window.about_title");
    tr.set_minimized(SharedString::from(i18n::t("ui.minimized")));
    tr.set_last_seen(SharedString::from(i18n::t("ui.last_seen")));
    tr.set_visible_label(SharedString::from(i18n::t("ui.visible")));
    tr.set_hidden_label(SharedString::from(i18n::t("ui.hidden")));
    tr.set_always_on_top_label(SharedString::from(i18n::t("ui.always_on_top")));
    tr.set_normal_window_label(SharedString::from(i18n::t("ui.normal_window")));
    tr.set_toolbar_hint(SharedString::from(i18n::t("ui.toolbar_hint")));
    tr.set_anim_on(SharedString::from(i18n::t("ui.anim_on")));
    tr.set_anim_off(SharedString::from(i18n::t("ui.anim_off")));
    set_tr!(tr, set_layout_grid_label, "layout.grid");
    set_tr!(tr, set_layout_mosaic_label, "layout.mosaic");
    set_tr!(tr, set_layout_bento_label, "layout.bento");
    set_tr!(tr, set_layout_fibonacci_label, "layout.fibonacci");
    set_tr!(tr, set_layout_columns_label, "layout.columns");
    set_tr!(tr, set_layout_row_label, "layout.row");
    set_tr!(tr, set_layout_column_label, "layout.column");
    set_tr!(tr, set_group_none_label, "group.none");
    set_tr!(tr, set_group_application_label, "group.application");
    set_tr!(tr, set_group_monitor_label, "group.monitor");
    set_tr!(tr, set_group_title_label, "group.title");
    set_tr!(tr, set_group_class_label, "group.class");
    set_tr!(tr, set_dock_none_label, "dock.none");
    set_tr!(tr, set_dock_left_label, "dock.left");
    set_tr!(tr, set_dock_right_label, "dock.right");
    set_tr!(tr, set_dock_top_label, "dock.top");
    set_tr!(tr, set_dock_bottom_label, "dock.bottom");
    set_tr!(tr, set_locale_english_label, "locale.english");
    set_tr!(tr, set_locale_spanish_label, "locale.spanish");
    set_tr!(tr, set_fit_cover_label, "settings.fit.cover");
    set_tr!(tr, set_fit_contain_label, "settings.fit.contain");
    set_tr!(tr, set_fit_fill_label, "settings.fit.fill");
    set_tr!(tr, set_fit_preserve_label, "settings.fit.preserve");
    set_tr!(tr, set_all_monitors_label, "tray.all_monitors");
    set_tr!(tr, set_all_tags_label, "tray.all_tags");
    set_tr!(tr, set_all_apps_label, "tray.all_apps");
    set_tr!(tr, set_no_hidden_label, "settings.no_hidden");
    set_tr!(
        tr,
        set_no_saved_profiles_label,
        "settings.no_saved_profiles"
    );
    set_tr!(tr, set_default_profile_label, "settings.default_profile");
    tr.set_empty_message(SharedString::from(i18n::t("ui.empty_message")));
    tr.set_empty_helper(SharedString::from(i18n::t("ui.empty_helper")));
    set_tr!(tr, set_action_close, "action.close");
    set_tr!(tr, set_action_reset_defaults, "action.reset_defaults");
    set_tr!(tr, set_action_restore_selected, "action.restore_selected");
    set_tr!(tr, set_action_restore_all, "action.restore_all");
    set_tr!(tr, set_action_browse_image, "action.browse_image");
    set_tr!(tr, set_action_clear_image, "action.clear_image");
    set_tr!(tr, set_action_refresh_now, "action.refresh_now");
    set_tr!(tr, set_action_auto_apply, "action.auto_apply");
    set_tr!(tr, set_action_about, "action.about");
    set_tr!(tr, set_action_load_profile, "action.load_profile");
    set_tr!(tr, set_about_title, "about.title");
    set_tr!(tr, set_about_subtitle, "about.subtitle");
    set_tr!(tr, set_about_version_title, "about.version_title");
    set_tr!(tr, set_about_description_title, "about.description_title");
    set_tr!(tr, set_about_description_body, "about.description_body");
    set_tr!(tr, set_about_credits_title, "about.credits_title");
    set_tr!(tr, set_about_credits_body, "about.credits_body");
    tr.set_dock_mode_hint(SharedString::from(i18n::t("settings.dock_hint")));
    tr.set_filters_hint(SharedString::from(i18n::t("settings.filters_hint")));
    tr.set_current_profile_prefix(SharedString::from(i18n::t("settings.current_profile")));
    tr.set_profile_input_label(SharedString::from(i18n::t("settings.profile_label")));
    tr.set_save_profile_btn(SharedString::from(i18n::t("settings.save_profile")));
    tr.set_open_instance_btn(SharedString::from(i18n::t("settings.open_instance")));
    tr.set_no_hidden_hint(SharedString::from(i18n::t("settings.no_hidden_hint")));
    set_tr!(tr, set_settings_title, "settings.title");
    set_tr!(tr, set_settings_subtitle, "settings.subtitle");
    set_tr!(tr, set_settings_profile_badge, "settings.profile_badge");
    set_tr!(
        tr,
        set_settings_nav_behaviour_display_title,
        "settings.nav.behaviour_display.title"
    );
    set_tr!(
        tr,
        set_settings_nav_behaviour_display_subtitle,
        "settings.nav.behaviour_display.subtitle"
    );
    set_tr!(
        tr,
        set_settings_nav_filters_title,
        "settings.nav.filters.title"
    );
    set_tr!(
        tr,
        set_settings_nav_filters_subtitle,
        "settings.nav.filters.subtitle"
    );
    set_tr!(
        tr,
        set_settings_nav_theme_background_title,
        "settings.nav.theme_background.title"
    );
    set_tr!(
        tr,
        set_settings_nav_theme_background_subtitle,
        "settings.nav.theme_background.subtitle"
    );
    set_tr!(
        tr,
        set_settings_nav_profiles_title,
        "settings.nav.profiles.title"
    );
    set_tr!(
        tr,
        set_settings_nav_profiles_subtitle,
        "settings.nav.profiles.subtitle"
    );
    set_tr!(
        tr,
        set_settings_nav_shortcuts_title,
        "settings.nav.shortcuts.title"
    );
    set_tr!(
        tr,
        set_settings_nav_shortcuts_subtitle,
        "settings.nav.shortcuts.subtitle"
    );
    set_tr!(
        tr,
        set_settings_nav_advanced_title,
        "settings.nav.advanced.title"
    );
    set_tr!(
        tr,
        set_settings_nav_advanced_subtitle,
        "settings.nav.advanced.subtitle"
    );
    set_tr!(
        tr,
        set_settings_page_behaviour_display_title,
        "settings.page.behaviour_display.title"
    );
    set_tr!(
        tr,
        set_settings_page_behaviour_display_subtitle,
        "settings.page.behaviour_display.subtitle"
    );
    set_tr!(
        tr,
        set_settings_section_behaviour_title,
        "settings.section.behaviour.title"
    );
    set_tr!(
        tr,
        set_settings_section_behaviour_helper,
        "settings.section.behaviour.helper"
    );
    set_tr!(
        tr,
        set_settings_option_language_title,
        "settings.option.language.title"
    );
    set_tr!(
        tr,
        set_settings_option_language_description,
        "settings.option.language.description"
    );
    set_tr!(
        tr,
        set_settings_option_always_on_top_title,
        "settings.option.always_on_top.title"
    );
    set_tr!(
        tr,
        set_settings_option_always_on_top_description,
        "settings.option.always_on_top.description"
    );
    set_tr!(
        tr,
        set_settings_option_animate_transitions_title,
        "settings.option.animate_transitions.title"
    );
    set_tr!(
        tr,
        set_settings_option_animate_transitions_description,
        "settings.option.animate_transitions.description"
    );
    set_tr!(
        tr,
        set_settings_option_minimize_to_tray_title,
        "settings.option.minimize_to_tray.title"
    );
    set_tr!(
        tr,
        set_settings_option_minimize_to_tray_description,
        "settings.option.minimize_to_tray.description"
    );
    set_tr!(
        tr,
        set_settings_option_close_to_tray_title,
        "settings.option.close_to_tray.title"
    );
    set_tr!(
        tr,
        set_settings_option_close_to_tray_description,
        "settings.option.close_to_tray.description"
    );
    set_tr!(
        tr,
        set_settings_option_preserve_aspect_ratio_title,
        "settings.option.preserve_aspect_ratio.title"
    );
    set_tr!(
        tr,
        set_settings_option_preserve_aspect_ratio_description,
        "settings.option.preserve_aspect_ratio.description"
    );
    set_tr!(
        tr,
        set_settings_option_hide_on_select_title,
        "settings.option.hide_on_select.title"
    );
    set_tr!(
        tr,
        set_settings_option_hide_on_select_description,
        "settings.option.hide_on_select.description"
    );
    set_tr!(
        tr,
        set_settings_option_start_in_tray_title,
        "settings.option.start_in_tray.title"
    );
    set_tr!(
        tr,
        set_settings_option_start_in_tray_description,
        "settings.option.start_in_tray.description"
    );
    set_tr!(
        tr,
        set_settings_option_run_at_startup_title,
        "settings.option.run_at_startup.title"
    );
    set_tr!(
        tr,
        set_settings_option_run_at_startup_description,
        "settings.option.run_at_startup.description"
    );
    set_tr!(
        tr,
        set_settings_option_lock_layout_title,
        "settings.option.lock_layout.title"
    );
    set_tr!(
        tr,
        set_settings_option_lock_layout_description,
        "settings.option.lock_layout.description"
    );
    set_tr!(
        tr,
        set_settings_option_lock_cell_resize_title,
        "settings.option.lock_cell_resize.title"
    );
    set_tr!(
        tr,
        set_settings_option_lock_cell_resize_description,
        "settings.option.lock_cell_resize.description"
    );
    set_tr!(
        tr,
        set_settings_section_display_title,
        "settings.section.display.title"
    );
    set_tr!(
        tr,
        set_settings_section_display_helper,
        "settings.section.display.helper"
    );
    set_tr!(
        tr,
        set_settings_option_show_toolbar_title,
        "settings.option.show_toolbar.title"
    );
    set_tr!(
        tr,
        set_settings_option_show_toolbar_description,
        "settings.option.show_toolbar.description"
    );
    set_tr!(
        tr,
        set_settings_option_show_info_title,
        "settings.option.show_info.title"
    );
    set_tr!(
        tr,
        set_settings_option_show_info_description,
        "settings.option.show_info.description"
    );
    set_tr!(
        tr,
        set_settings_option_show_app_icons_title,
        "settings.option.show_app_icons.title"
    );
    set_tr!(
        tr,
        set_settings_option_show_app_icons_description,
        "settings.option.show_app_icons.description"
    );
    set_tr!(
        tr,
        set_settings_option_use_system_backdrop_title,
        "settings.option.use_system_backdrop.title"
    );
    set_tr!(
        tr,
        set_settings_option_use_system_backdrop_description,
        "settings.option.use_system_backdrop.description"
    );
    set_tr!(
        tr,
        set_settings_page_filters_title,
        "settings.page.filters.title"
    );
    set_tr!(
        tr,
        set_settings_page_filters_subtitle,
        "settings.page.filters.subtitle"
    );
    set_tr!(
        tr,
        set_settings_option_monitor_filter_title,
        "settings.option.monitor_filter.title"
    );
    set_tr!(
        tr,
        set_settings_option_monitor_filter_description,
        "settings.option.monitor_filter.description"
    );
    set_tr!(
        tr,
        set_settings_option_tag_filter_title,
        "settings.option.tag_filter.title"
    );
    set_tr!(
        tr,
        set_settings_option_tag_filter_description,
        "settings.option.tag_filter.description"
    );
    set_tr!(
        tr,
        set_settings_option_app_filter_title,
        "settings.option.app_filter.title"
    );
    set_tr!(
        tr,
        set_settings_option_app_filter_description,
        "settings.option.app_filter.description"
    );
    set_tr!(
        tr,
        set_settings_option_group_windows_title,
        "settings.option.group_windows.title"
    );
    set_tr!(
        tr,
        set_settings_option_group_windows_description,
        "settings.option.group_windows.description"
    );
    set_tr!(
        tr,
        set_settings_section_hidden_apps_title,
        "settings.section.hidden_apps.title"
    );
    set_tr!(
        tr,
        set_settings_section_hidden_apps_helper,
        "settings.section.hidden_apps.helper"
    );
    set_tr!(
        tr,
        set_settings_page_theme_background_title,
        "settings.page.theme_background.title"
    );
    set_tr!(
        tr,
        set_settings_page_theme_background_subtitle,
        "settings.page.theme_background.subtitle"
    );
    set_tr!(
        tr,
        set_settings_section_theme_grid_title,
        "settings.section.theme_grid.title"
    );
    set_tr!(
        tr,
        set_settings_section_theme_grid_helper,
        "settings.section.theme_grid.helper"
    );
    set_tr!(
        tr,
        set_settings_section_canvas_background_title,
        "settings.section.canvas_background.title"
    );
    set_tr!(
        tr,
        set_settings_section_canvas_background_helper,
        "settings.section.canvas_background.helper"
    );
    set_tr!(
        tr,
        set_settings_option_custom_canvas_colour_title,
        "settings.option.custom_canvas_colour.title"
    );
    set_tr!(
        tr,
        set_settings_option_custom_canvas_colour_description,
        "settings.option.custom_canvas_colour.description"
    );
    set_tr!(
        tr,
        set_settings_section_preview_title,
        "settings.section.preview.title"
    );
    set_tr!(
        tr,
        set_settings_section_preview_helper,
        "settings.section.preview.helper"
    );
    set_tr!(
        tr,
        set_settings_section_background_image_title,
        "settings.section.background_image.title"
    );
    set_tr!(
        tr,
        set_settings_section_background_image_helper,
        "settings.section.background_image.helper"
    );
    set_tr!(
        tr,
        set_settings_option_image_file_title,
        "settings.option.image_file.title"
    );
    set_tr!(
        tr,
        set_settings_option_image_file_description,
        "settings.option.image_file.description"
    );
    set_tr!(
        tr,
        set_settings_option_image_fit_title,
        "settings.option.image_fit.title"
    );
    set_tr!(
        tr,
        set_settings_option_image_fit_description,
        "settings.option.image_fit.description"
    );
    set_tr!(
        tr,
        set_settings_option_image_opacity_title,
        "settings.option.image_opacity.title"
    );
    set_tr!(
        tr,
        set_settings_option_image_opacity_description,
        "settings.option.image_opacity.description"
    );
    set_tr!(
        tr,
        set_settings_page_profiles_title,
        "settings.page.profiles.title"
    );
    set_tr!(
        tr,
        set_settings_page_profiles_subtitle,
        "settings.page.profiles.subtitle"
    );
    set_tr!(
        tr,
        set_settings_section_edit_profile_title,
        "settings.section.edit_profile.title"
    );
    set_tr!(
        tr,
        set_settings_section_edit_profile_helper,
        "settings.section.edit_profile.helper"
    );
    set_tr!(
        tr,
        set_settings_current_profile_card_title,
        "settings.current_profile_card.title"
    );
    set_tr!(
        tr,
        set_settings_option_profile_name_title,
        "settings.option.profile_name.title"
    );
    set_tr!(
        tr,
        set_settings_option_profile_name_description,
        "settings.option.profile_name.description"
    );
    set_tr!(
        tr,
        set_settings_section_saved_profiles_title,
        "settings.section.saved_profiles.title"
    );
    set_tr!(
        tr,
        set_settings_section_saved_profiles_helper,
        "settings.section.saved_profiles.helper"
    );
    set_tr!(
        tr,
        set_settings_section_load_profile_title,
        "settings.section.load_profile.title"
    );
    set_tr!(
        tr,
        set_settings_section_load_profile_helper,
        "settings.section.load_profile.helper"
    );
    set_tr!(
        tr,
        set_settings_option_available_profile_title,
        "settings.option.available_profile.title"
    );
    set_tr!(
        tr,
        set_settings_option_available_profile_description,
        "settings.option.available_profile.description"
    );
    set_tr!(tr, set_settings_tips_title, "settings.tips.title");
    set_tr!(tr, set_settings_tips_body, "settings.tips.body");
    set_tr!(
        tr,
        set_settings_page_shortcuts_title,
        "settings.page.shortcuts.title"
    );
    set_tr!(
        tr,
        set_settings_page_shortcuts_subtitle,
        "settings.page.shortcuts.subtitle"
    );
    set_tr!(
        tr,
        set_settings_section_layout_bindings_title,
        "settings.section.layout_bindings.title"
    );
    set_tr!(
        tr,
        set_settings_section_layout_bindings_helper,
        "settings.section.layout_bindings.helper"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_grid_title,
        "settings.shortcut.layout_grid.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_grid_description,
        "settings.shortcut.layout_grid.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_mosaic_title,
        "settings.shortcut.layout_mosaic.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_mosaic_description,
        "settings.shortcut.layout_mosaic.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_bento_title,
        "settings.shortcut.layout_bento.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_bento_description,
        "settings.shortcut.layout_bento.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_fibonacci_title,
        "settings.shortcut.layout_fibonacci.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_fibonacci_description,
        "settings.shortcut.layout_fibonacci.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_columns_title,
        "settings.shortcut.layout_columns.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_columns_description,
        "settings.shortcut.layout_columns.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_row_title,
        "settings.shortcut.layout_row.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_row_description,
        "settings.shortcut.layout_row.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_column_title,
        "settings.shortcut.layout_column.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_layout_column_description,
        "settings.shortcut.layout_column.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_reset_layout_title,
        "settings.shortcut.reset_layout.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_reset_layout_description,
        "settings.shortcut.reset_layout.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_cycle_layout_title,
        "settings.shortcut.cycle_layout.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_cycle_layout_description,
        "settings.shortcut.cycle_layout.description"
    );
    set_tr!(
        tr,
        set_settings_section_dashboard_actions_title,
        "settings.section.dashboard_actions.title"
    );
    set_tr!(
        tr,
        set_settings_section_dashboard_actions_helper,
        "settings.section.dashboard_actions.helper"
    );
    set_tr!(
        tr,
        set_settings_shortcut_cycle_theme_title,
        "settings.shortcut.cycle_theme.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_cycle_theme_description,
        "settings.shortcut.cycle_theme.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_animations_title,
        "settings.shortcut.toggle_animations.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_animations_description,
        "settings.shortcut.toggle_animations.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_toolbar_title,
        "settings.shortcut.toggle_toolbar.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_toolbar_description,
        "settings.shortcut.toggle_toolbar.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_window_info_title,
        "settings.shortcut.toggle_window_info.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_window_info_description,
        "settings.shortcut.toggle_window_info.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_always_on_top_title,
        "settings.shortcut.toggle_always_on_top.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_toggle_always_on_top_description,
        "settings.shortcut.toggle_always_on_top.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_open_settings_title,
        "settings.shortcut.open_settings.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_open_settings_description,
        "settings.shortcut.open_settings.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_open_menu_title,
        "settings.shortcut.open_menu.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_open_menu_description,
        "settings.shortcut.open_menu.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_global_activate_title,
        "settings.shortcut.global_activate.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_global_activate_description,
        "settings.shortcut.global_activate.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_refresh_now_title,
        "settings.shortcut.refresh_now.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_refresh_now_description,
        "settings.shortcut.refresh_now.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_exit_app_title,
        "settings.shortcut.exit_app.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_exit_app_description,
        "settings.shortcut.exit_app.description"
    );
    set_tr!(
        tr,
        set_settings_shortcut_alt_toolbar_title,
        "settings.shortcut.alt_toolbar.title"
    );
    set_tr!(
        tr,
        set_settings_shortcut_alt_toolbar_description,
        "settings.shortcut.alt_toolbar.description"
    );
    set_tr!(
        tr,
        set_settings_page_advanced_title,
        "settings.page.advanced.title"
    );
    set_tr!(
        tr,
        set_settings_page_advanced_subtitle,
        "settings.page.advanced.subtitle"
    );
    set_tr!(
        tr,
        set_settings_option_default_layout_title,
        "settings.option.default_layout.title"
    );
    set_tr!(
        tr,
        set_settings_option_default_layout_description,
        "settings.option.default_layout.description"
    );
    set_tr!(
        tr,
        set_settings_option_default_layout_docked_description,
        "settings.option.default_layout.docked_description"
    );
    set_tr!(
        tr,
        set_settings_option_refresh_interval_title,
        "settings.option.refresh_interval.title"
    );
    set_tr!(
        tr,
        set_settings_option_refresh_interval_description,
        "settings.option.refresh_interval.description"
    );
    set_tr!(
        tr,
        set_settings_section_manual_refresh_title,
        "settings.section.manual_refresh.title"
    );
    set_tr!(
        tr,
        set_settings_section_manual_refresh_helper,
        "settings.section.manual_refresh.helper"
    );
    set_tr!(
        tr,
        set_settings_section_dock_thickness_title,
        "settings.section.dock_thickness.title"
    );
    set_tr!(
        tr,
        set_settings_section_dock_thickness_helper,
        "settings.section.dock_thickness.helper"
    );
    set_tr!(
        tr,
        set_settings_section_floating_window_size_title,
        "settings.section.floating_window_size.title"
    );
    set_tr!(
        tr,
        set_settings_section_floating_window_size_helper,
        "settings.section.floating_window_size.helper"
    );
    set_tr!(
        tr,
        set_settings_option_thumbnail_render_scale_title,
        "settings.option.thumbnail_render_scale.title"
    );
    set_tr!(
        tr,
        set_settings_option_thumbnail_render_scale_description,
        "settings.option.thumbnail_render_scale.description"
    );
    set_tr!(tr, set_settings_width_label, "settings.label.width");
    set_tr!(tr, set_settings_height_label, "settings.label.height");
    set_tr!(
        tr,
        set_settings_option_dock_position_title,
        "settings.option.dock_position.title"
    );
    set_tr!(
        tr,
        set_settings_option_dock_position_description,
        "settings.option.dock_position.description"
    );
    tr.set_tag_title(SharedString::from(i18n::t("tag.title")));
    tr.set_tag_app_label(SharedString::from(i18n::t("tag.application")));
    tr.set_tag_name_label(SharedString::from(i18n::t("tag.name_label")));
    tr.set_tag_preset_colour(SharedString::from(i18n::t("tag.preset_colour")));
    tr.set_tag_create_assign(SharedString::from(i18n::t("tag.create_assign")));
    set_tr!(tr, set_tag_colour_amber_label, "tag.color.amber");
    set_tr!(tr, set_tag_colour_sky_label, "tag.color.sky");
    set_tr!(tr, set_tag_colour_mint_label, "tag.color.mint");
    set_tr!(tr, set_tag_colour_rose_label, "tag.color.rose");
    set_tr!(tr, set_tag_colour_violet_label, "tag.color.violet");
    set_tr!(tr, set_tag_colour_sun_label, "tag.color.sun");
}

// ───────────────────────── Entry Point ─────────────────────────

#[cfg(target_os = "windows")]
fn select_text_friendly_renderer() {
    let renderer_selection = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("skia".into())
        .select();

    match renderer_selection {
        Ok(()) => {
            tracing::info!(
                "selected Slint winit backend with Skia renderer for sharper Windows text"
            );
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "failed to select Slint Skia renderer; falling back to default backend selection"
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn select_text_friendly_renderer() {}

#[allow(clippy::too_many_lines)]
fn main() {
    let _ = panopticon::i18n::set_locale(panopticon::i18n::Locale::English);
    let startup_args = match parse_startup_args() {
        Ok(startup_args) => startup_args,
        Err(error) => StartupArgs::PrintAndExit {
            message: format!("{error}\n\n{}", cli_usage()),
            stderr: true,
        },
    };

    match startup_args {
        StartupArgs::Run { profile } => run_app(profile),
        StartupArgs::PrintAndExit { message, stderr } => {
            if stderr {
                eprintln!("{message}");
            } else {
                println!("{message}");
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run_app(profile: Option<String>) {
    let _log_guard = panopticon::logging::init().ok();
    select_text_friendly_renderer();

    tracing::info!(profile = ?profile, "Panopticon starting (Slint UI)");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::Relaxed);
    }

    let icons = match profile.as_deref() {
        Some(name) => {
            let idx = name.bytes().fold(0u32, |a, b| a.wrapping_add(u32::from(b))) as usize
                % INSTANCE_ACCENT_PALETTE.len();
            let [r, g, b] = INSTANCE_ACCENT_PALETTE[idx];
            AppIcons::with_accent(r, g, b).unwrap_or_else(|_| {
                AppIcons::new().unwrap_or_else(|error| {
                    tracing::error!(%error, "icon generation failed; falling back");
                    AppIcons::fallback_system()
                })
            })
        }
        None => AppIcons::new().unwrap_or_else(|error| {
            tracing::error!(%error, "icon generation failed; falling back");
            AppIcons::fallback_system()
        }),
    };
    let settings = AppSettings::load_or_default(profile.as_deref()).unwrap_or_else(|error| {
        tracing::error!(%error, "settings load failed; using defaults");
        AppSettings::default()
    });
    app::startup::sync_run_at_startup(settings.run_at_startup, profile.as_deref());
    panopticon::i18n::init(settings.language);
    app::secondary_windows::ensure_default_profiles_exist(&settings);

    let initial_theme = theme_catalog::resolve_ui_theme(
        settings.theme_id.as_deref(),
        &settings.background_color_hex,
    );

    let main_window = match MainWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create main window");
            return;
        }
    };
    populate_tr_global(&main_window);
    apply_main_window_theme_snapshot(&main_window, &initial_theme);

    // Apply initial property values from settings.
    sync_settings_to_ui(&main_window, &settings);

    let state = Rc::new(RefCell::new(AppState {
        hwnd: HWND::default(),
        windows: Vec::new(),
        current_layout: settings.effective_layout(),
        active_hwnd: None,
        tray_icon: None,
        icons,
        settings,
        animation_started_at: None,
        content_extent: 0,
        is_appbar: false,
        profile_name: profile,
        last_size: (0, 0),
        separators: Vec::new(),
        drag_separator: None,
        loaded_background_path: None,
        current_theme: initial_theme,
        theme_animation: None,
    }));

    // Show the window so the native HWND exists on next event-loop iteration.
    if let Err(error) = main_window.show() {
        tracing::error!(%error, "failed to show main window");
        return;
    }

    // Slint callbacks (don't need HWND — they use state internally).
    setup_callbacks(&main_window, &state);

    // Handle close button.
    main_window.window().on_close_requested({
        let state = state.clone();
        move || {
            let s = state.borrow();
            if s.settings.close_to_tray {
                drop(s);
                release_all_thumbnails(&state);
                CloseRequestResponse::HideWindow
            } else {
                drop(s);
                queue_exit_request();
                CloseRequestResponse::KeepWindowShown
            }
        }
    });

    // ── Deferred native-HWND initialisation (runs once the event loop is live) ──
    let init_timer = Timer::default();
    init_timer.start(TimerMode::SingleShot, Duration::from_millis(0), {
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            let Some(win) = weak.upgrade() else { return };
            let _ = app::native_runtime::try_initialize_native_runtime(&state, &win);
        }
    });

    // ── Timers ──────────────────────────────────────

    // Fast UI timer: size polling, animation, DWM thumbnail sync, action drain.
    let ui_timer = Timer::default();
    ui_timer.start(TimerMode::Repeated, Duration::from_millis(16), {
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            let Some(win) = weak.upgrade() else { return };
            if state.borrow().hwnd.0.is_null()
                && !app::native_runtime::try_initialize_native_runtime(&state, &win)
            {
                return;
            }

            // Drain pending actions without intermediate Vec allocation.
            PENDING_ACTIONS.with(|q| {
                let mut queue = q.borrow_mut();
                if !queue.is_empty() {
                    // Swap with a reusable buffer to avoid alloc on each tick.
                    let mut batch = std::mem::take(&mut *queue);
                    drop(queue);
                    for action in batch.drain(..) {
                        handle_pending_action(&state, &win, action);
                    }
                    // Return the allocation back for reuse.
                    let mut queue = q.borrow_mut();
                    if queue.is_empty() {
                        *queue = batch;
                    }
                }
            });

            // Check for window-size changes.
            let phys_size = win.window().size();
            let scale = win.window().scale_factor();
            let logical_w = (phys_size.width as f32 / scale).round() as i32;
            let logical_h = (phys_size.height as f32 / scale).round() as i32;
            let needs_relayout = {
                let s = state.borrow();
                logical_w != s.last_size.0 || logical_h != s.last_size.1
            };
            if needs_relayout {
                state.borrow_mut().last_size = (logical_w, logical_h);
                recompute_and_update_ui(&state, &win);
            }

            // Advance animations.
            advance_animation(&state, &win);

            // Smoothly interpolate theme changes.
            advance_theme_animation(&state, &win);

            // Re-sync DWM thumbnails (scroll changes, animation frames, etc.).
            update_dwm_thumbnails(&state, &win);
        }
    });

    // Slow refresh timer: window enumeration.
    let refresh_timer = Timer::default();
    refresh_timer.start(
        TimerMode::Repeated,
        Duration::from_millis((state.borrow().settings.refresh_interval_ms as u64).max(50)),
        {
            let state = state.clone();
            let weak = main_window.as_weak();
            move || {
                let visible = UI_STATE.with(|s| {
                    s.borrow().as_ref().is_some_and(|rc| {
                        rc.try_borrow()
                            .is_ok_and(|s| unsafe { IsWindowVisible(s.hwnd).as_bool() })
                    })
                });
                if !visible {
                    return;
                }
                if refresh_windows(&state) {
                    if let Some(win) = weak.upgrade() {
                        recompute_and_update_ui(&state, &win);
                    }
                }
            }
        },
    );

    // Scrollbar auto-hide timer: checks every 500 ms and hides after inactivity.
    let scrollbar_timer = Timer::default();
    scrollbar_timer.start(TimerMode::Repeated, Duration::from_millis(500), {
        let weak = main_window.as_weak();
        move || {
            app::window_subclass::hide_scrollbar_if_idle(&weak);
        }
    });

    tracing::info!("entering Slint event loop");
    if let Err(error) = slint::run_event_loop_until_quit() {
        tracing::error!(%error, "Slint event loop failed");
    }
    let hwnd = state.borrow().hwnd;
    if !hwnd.0.is_null() {
        app::window_subclass::teardown_subclass(hwnd);
    }
    tracing::info!("Panopticon exiting");
}

// ───────────────────────── Slint Callbacks ─────────────────────────

#[allow(clippy::too_many_lines)]
fn setup_callbacks(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) {
    main_window.on_thumbnail_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            app::thumbnail_interactions::handle_thumbnail_click(&state, &weak, index as usize);
        }
    });

    main_window.on_thumbnail_right_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            app::thumbnail_interactions::handle_thumbnail_right_click(
                &state,
                &weak,
                index as usize,
                x,
                y,
            );
        }
    });

    main_window.on_thumbnail_drag_ended({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |src_idx, drop_x, drop_y| {
            app::thumbnail_interactions::handle_thumbnail_drag_ended(
                &state,
                &weak,
                src_idx as usize,
                drop_x as f64,
                drop_y as f64,
            );
        }
    });

    main_window.on_thumbnail_close_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            app::thumbnail_interactions::handle_thumbnail_close(&state, &weak, index as usize);
        }
    });

    main_window.on_toolbar_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            cycle_layout(&state);
            if let Some(win) = weak.upgrade() {
                recompute_and_update_ui(&state, &win);
            }
        }
    });

    main_window.on_app_context_menu_requested({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |x, y| app::tray_actions::open_application_context_menu(&state, &weak, Some((x, y)))
    });

    main_window.on_resize_drag_started({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            app::layout_actions::handle_resize_drag_start(
                &state,
                &weak,
                index as usize,
                x as f64,
                y as f64,
            );
        }
    });

    main_window.on_resize_drag_moved({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            app::layout_actions::handle_resize_drag_move(
                &state,
                &weak,
                index as usize,
                x as f64,
                y as f64,
            );
        }
    });

    main_window.on_resize_drag_ended({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |_index| {
            app::layout_actions::handle_resize_drag_end(&state, &weak);
        }
    });

    main_window.on_key_pressed({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |key_text, shift_pressed| {
            app::keyboard_actions::handle_key(&state, &weak, &key_text, shift_pressed)
        }
    });
}

// ───────────────────────── Resize Drag ─────────────────────────

fn handle_pending_action(state: &Rc<RefCell<AppState>>, win: &MainWindow, action: PendingAction) {
    let weak = win.as_weak();
    match action {
        PendingAction::Tray(ta) => app::tray_actions::handle_tray_action(state, &weak, ta),
        PendingAction::ActivateMainWindow => app::tray_actions::activate_main_window(state, &weak),
        PendingAction::Reposition => {
            if let Ok(mut s) = state.try_borrow_mut() {
                if s.is_appbar {
                    reposition_appbar(&mut s);
                }
            }
        }
        PendingAction::HideToTray => {
            release_all_thumbnails(state);
            win.hide().ok();
        }
        PendingAction::Refresh => {
            if refresh_windows(state) {
                recompute_and_update_ui(state, win);
            }
        }
        PendingAction::Exit => {
            app::native_runtime::request_exit(state);
        }
    }
}

// ───────────────────────── Layout / State helpers ─────────────────────────

pub(crate) fn update_settings(
    state: &Rc<RefCell<AppState>>,
    mutate: impl FnOnce(&mut AppSettings),
) {
    let (hwnd, settings_snapshot, profile_name) = {
        let mut s = state.borrow_mut();
        mutate(&mut s.settings);
        s.settings = s.settings.normalized();
        let _ = s.settings.save(s.profile_name.as_deref());
        (s.hwnd, s.settings.clone(), s.profile_name.clone())
    };
    app::startup::sync_run_at_startup(settings_snapshot.run_at_startup, profile_name.as_deref());
    app::global_hotkey::sync_activate_hotkey(hwnd, &settings_snapshot);
}

pub(crate) fn refresh_ui(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
        advance_theme_animation(state, &win);
    }
    app::secondary_windows::refresh_open_settings_window(state);
}

/// Schedule an immediate refresh + a deferred one (300 ms) so that
/// closed/killed windows disappear promptly even if the process takes
/// a moment to terminate.
pub(crate) fn schedule_deferred_refresh(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
) {
    let _ = refresh_windows(state);
    refresh_ui(state, weak);

    let state2 = state.clone();
    let weak2 = weak.clone();
    let timer = Timer::default();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(300),
        move || {
            if refresh_windows(&state2) {
                if let Some(win) = weak2.upgrade() {
                    recompute_and_update_ui(&state2, &win);
                }
            }
        },
    );
    // Intentional: the Slint event loop owns the timer until it fires;
    // dropping it here would cancel the callback. `forget` transfers
    // ownership to the event loop (no real leak for SingleShot timers).
    std::mem::forget(timer);
}

// ───────────────────────── Utility ─────────────────────────

fn parse_startup_args() -> Result<StartupArgs, String> {
    parse_startup_args_from(std::env::args())
}

fn parse_startup_args_from(
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<StartupArgs, String> {
    let mut profile = None;
    let mut args = args.into_iter().map(Into::into);
    let _ = args.next();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" | "/?" => {
                return Ok(StartupArgs::PrintAndExit {
                    message: cli_usage(),
                    stderr: false,
                });
            }
            "--version" | "-V" => {
                return Ok(StartupArgs::PrintAndExit {
                    message: cli_version(),
                    stderr: false,
                });
            }
            "--profile" => {
                let Some(raw_profile) = args.next() else {
                    return Err(panopticon::i18n::t("cli.missing_profile_value").to_owned());
                };
                profile = Some(parse_profile_name(&raw_profile)?);
            }
            _ => {
                if let Some(raw_profile) = arg.strip_prefix("--profile=") {
                    profile = Some(parse_profile_name(raw_profile)?);
                } else {
                    return Err(panopticon::i18n::t_fmt("cli.unknown_argument", &arg));
                }
            }
        }
    }

    Ok(StartupArgs::Run { profile })
}

fn parse_profile_name(raw_profile: &str) -> Result<String, String> {
    match panopticon::settings::validate_profile_name_input(raw_profile) {
        panopticon::settings::ProfileNameValidation::Valid(profile_name) => Ok(profile_name),
        panopticon::settings::ProfileNameValidation::Empty => {
            Err(panopticon::i18n::t("settings.profile_empty_name").to_owned())
        }
        panopticon::settings::ProfileNameValidation::Invalid(reason) => Err(reason),
    }
}

fn cli_usage() -> String {
    format!(
        "{} {}\n\n{}\n  panopticon [--profile <name>]\n  panopticon [--profile=<name>]\n  panopticon --help\n  panopticon --version\n\n{}\n  --profile <name>   {}\n  --help, -h, /?     {}\n  --version, -V      {}",
        panopticon::i18n::t("app.name"),
        env!("CARGO_PKG_VERSION"),
        panopticon::i18n::t("cli.usage_heading"),
        panopticon::i18n::t("cli.options_heading"),
        panopticon::i18n::t("cli.profile_option_help"),
        panopticon::i18n::t("cli.help_option_help"),
        panopticon::i18n::t("cli.help_option_version"),
    )
}

fn cli_version() -> String {
    format!(
        "{} {}",
        panopticon::i18n::t("app.name"),
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn logical_to_screen_point(hwnd: HWND, logical_x: f32, logical_y: f32) -> POINT {
    let mut window_rect = RECT::default();
    // SAFETY: hwnd is our live window; window_rect is stack-allocated and valid.
    unsafe {
        let _ = GetWindowRect(hwnd, &raw mut window_rect);
    }

    POINT {
        x: window_rect.left + logical_x.round() as i32,
        y: window_rect.top + logical_y.round() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dwm::sanitize_thumbnail_rect;
    use crate::app::icon::bilinear_sample_rgba;
    use panopticon::window_enum::WindowInfo;
    use panopticon::window_ops::apply_pinned_positions;
    use std::ffi::c_void;

    #[test]
    fn sanitize_thumbnail_rect_clips_to_client_bounds() {
        let (rect, visible) = sanitize_thumbnail_rect(
            RECT {
                left: -12,
                top: 10,
                right: 180,
                bottom: 140,
            },
            120,
            90,
        );

        assert!(visible);
        assert_eq!(rect.left, 0);
        assert_eq!(rect.top, 10);
        assert_eq!(rect.right, 120);
        assert_eq!(rect.bottom, 90);
    }

    #[test]
    fn sanitize_thumbnail_rect_hides_rects_outside_client() {
        let (rect, visible) = sanitize_thumbnail_rect(
            RECT {
                left: 300,
                top: 50,
                right: 360,
                bottom: 110,
            },
            200,
            120,
        );

        assert!(!visible);
        assert_eq!(rect, HIDDEN_THUMBNAIL_RECT);
    }

    #[test]
    fn bilinear_sample_rgba_preserves_transparent_edges() {
        let size = 4usize;
        let mut source = vec![0u8; size * size * 4];
        let center = (size + 1) * 4;
        source[center..center + 4].copy_from_slice(&[255, 128, 64, 255]);

        let sample = bilinear_sample_rgba(&source, size, 1.0, 1.0);

        assert_eq!(sample, [255, 128, 64, 255]);
        let transparent = bilinear_sample_rgba(&source, size, 0.0, 0.0);
        assert_eq!(transparent[3], 0);
    }

    #[test]
    fn apply_pinned_positions_keeps_pinned_app_in_reserved_slot() {
        let mut settings = AppSettings::default();
        let _ = settings.toggle_app_pinned_position("app:b", "B", 1);

        let mut windows = vec![
            WindowInfo {
                hwnd: HWND(std::ptr::dangling_mut::<c_void>()),
                title: "Alpha".to_owned(),
                app_id: "app:a".to_owned(),
                process_name: "A".to_owned(),
                process_path: None,
                class_name: "A".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
            WindowInfo {
                hwnd: HWND(2usize as *mut c_void),
                title: "Bravo".to_owned(),
                app_id: "app:b".to_owned(),
                process_name: "B".to_owned(),
                process_path: None,
                class_name: "B".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
            WindowInfo {
                hwnd: HWND(3usize as *mut c_void),
                title: "Charlie".to_owned(),
                app_id: "app:c".to_owned(),
                process_name: "C".to_owned(),
                process_path: None,
                class_name: "C".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
        ];

        windows.swap(0, 1);
        apply_pinned_positions(&mut windows, &settings);

        assert_eq!(windows[1].app_id, "app:b");
    }

    #[test]
    fn parse_startup_args_supports_profile_value_forms() {
        assert_eq!(
            parse_startup_args_from(["panopticon", "--profile", "work"]),
            Ok(StartupArgs::Run {
                profile: Some("work".to_owned()),
            })
        );

        assert_eq!(
            parse_startup_args_from(["panopticon", "--profile=focus"]),
            Ok(StartupArgs::Run {
                profile: Some("focus".to_owned()),
            })
        );
    }

    #[test]
    fn parse_startup_args_supports_help_and_version_flags() {
        let help = parse_startup_args_from(["panopticon", "--help"]);
        assert!(matches!(
            help,
            Ok(StartupArgs::PrintAndExit { stderr: false, .. })
        ));
        assert!(matches!(
            help,
            Ok(StartupArgs::PrintAndExit { ref message, .. }) if message.contains("Usage:")
        ));

        let version = parse_startup_args_from(["panopticon", "--version"]);
        assert!(matches!(
            version,
            Ok(StartupArgs::PrintAndExit { stderr: false, .. })
        ));
        assert!(matches!(
            version,
            Ok(StartupArgs::PrintAndExit { ref message, .. }) if message.contains(env!("CARGO_PKG_VERSION"))
        ));
    }

    #[test]
    fn parse_startup_args_rejects_unknown_or_invalid_arguments() {
        let missing_value = parse_startup_args_from(["panopticon", "--profile"]);
        assert!(matches!(missing_value, Err(ref error) if error.contains("Missing value")));

        let invalid_profile = parse_startup_args_from(["panopticon", "--profile", "???"]);
        assert!(matches!(
            invalid_profile,
            Err(ref error) if error.contains("invalid")
        ));

        let unknown = parse_startup_args_from(["panopticon", "--wat"]);
        assert!(matches!(unknown, Err(ref error) if error.contains("Unknown argument")));
    }
}
