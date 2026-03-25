//! Helpers for synchronizing the Slint settings window with persisted settings.

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;
use slint::SharedString;

use crate::SettingsWindow;

pub fn populate_settings_window(window: &SettingsWindow, settings: &AppSettings) {
    window.set_always_on_top_setting(settings.always_on_top);
    window.set_animate_transitions_setting(settings.animate_transitions);
    window.set_minimize_to_tray_setting(settings.minimize_to_tray);
    window.set_close_to_tray_setting(settings.close_to_tray);
    window.set_preserve_aspect_ratio_setting(settings.preserve_aspect_ratio);
    window.set_hide_on_select_setting(settings.hide_on_select);
    window.set_show_toolbar_setting(settings.show_toolbar);
    window.set_show_info_setting(settings.show_window_info);
    window.set_use_system_backdrop_setting(settings.use_system_backdrop);
    window.set_bg_color_hex(SharedString::from(&settings.background_color_hex));
    window.set_fixed_width_value(settings.fixed_width.unwrap_or(0) as i32);
    window.set_fixed_height_value(settings.fixed_height.unwrap_or(0) as i32);
    window.set_refresh_index(refresh_to_index(settings.refresh_interval_ms));
    window.set_layout_index(layout_to_index(settings.initial_layout));
}

pub fn apply_settings_window_changes(
    window: &SettingsWindow,
    settings: &mut AppSettings,
) -> LayoutType {
    settings.always_on_top = window.get_always_on_top_setting();
    settings.animate_transitions = window.get_animate_transitions_setting();
    settings.minimize_to_tray = window.get_minimize_to_tray_setting();
    settings.close_to_tray = window.get_close_to_tray_setting();
    settings.preserve_aspect_ratio = window.get_preserve_aspect_ratio_setting();
    settings.hide_on_select = window.get_hide_on_select_setting();
    settings.show_toolbar = window.get_show_toolbar_setting();
    settings.show_window_info = window.get_show_info_setting();
    settings.use_system_backdrop = window.get_use_system_backdrop_setting();
    settings.background_color_hex = window.get_bg_color_hex().to_string();

    let fixed_width = window.get_fixed_width_value();
    settings.fixed_width = if fixed_width > 0 {
        Some(fixed_width as u32)
    } else {
        None
    };

    let fixed_height = window.get_fixed_height_value();
    settings.fixed_height = if fixed_height > 0 {
        Some(fixed_height as u32)
    } else {
        None
    };

    settings.refresh_interval_ms = index_to_refresh(window.get_refresh_index());
    let layout = index_to_layout(window.get_layout_index());
    settings.initial_layout = layout;
    layout
}

fn layout_to_index(layout: LayoutType) -> i32 {
    match layout {
        LayoutType::Grid => 0,
        LayoutType::Mosaic => 1,
        LayoutType::Bento => 2,
        LayoutType::Fibonacci => 3,
        LayoutType::Columns => 4,
        LayoutType::Row => 5,
        LayoutType::Column => 6,
    }
}

fn index_to_layout(index: i32) -> LayoutType {
    match index {
        1 => LayoutType::Mosaic,
        2 => LayoutType::Bento,
        3 => LayoutType::Fibonacci,
        4 => LayoutType::Columns,
        5 => LayoutType::Row,
        6 => LayoutType::Column,
        _ => LayoutType::Grid,
    }
}

fn refresh_to_index(milliseconds: u32) -> i32 {
    match milliseconds {
        1_000 => 0,
        5_000 => 2,
        10_000 => 3,
        _ => 1,
    }
}

fn index_to_refresh(index: i32) -> u32 {
    match index {
        0 => 1_000,
        2 => 5_000,
        3 => 10_000,
        _ => 2_000,
    }
}
