//! Helpers for synchronizing the Slint settings window with persisted settings.

use panopticon::layout::LayoutType;
use panopticon::settings::{AppSettings, DockEdge};
use panopticon::theme;
use slint::SharedString;

use crate::SettingsWindow;

pub fn populate_settings_window(window: &SettingsWindow, settings: &AppSettings) {
    window.set_always_on_top_setting(settings.always_on_top);
    window.set_animate_transitions_setting(settings.animate_transitions);
    window.set_minimize_to_tray_setting(settings.minimize_to_tray);
    window.set_close_to_tray_setting(settings.close_to_tray);
    window.set_preserve_aspect_ratio_setting(settings.preserve_aspect_ratio);
    window.set_hide_on_select_setting(settings.hide_on_select);
    window.set_hide_on_select_enabled(settings.dock_edge.is_none());
    window.set_show_toolbar_setting(settings.show_toolbar);
    window.set_show_info_setting(settings.show_window_info);
    window.set_use_system_backdrop_setting(settings.use_system_backdrop);
    window.set_start_in_tray_setting(settings.start_in_tray);
    window.set_locked_layout_setting(settings.locked_layout);
    window.set_show_app_icons_setting(settings.show_app_icons);
    window.set_theme_index(theme::theme_index(settings.theme_id.as_deref()));
    window.set_bg_color_hex(SharedString::from(&settings.background_color_hex));
    window.set_bg_image_path(SharedString::from(
        settings.background_image_path.as_deref().unwrap_or(""),
    ));
    window.set_fixed_width_value(settings.fixed_width.unwrap_or(0) as i32);
    window.set_fixed_height_value(settings.fixed_height.unwrap_or(0) as i32);
    window.set_refresh_index(refresh_to_index(settings.refresh_interval_ms));
    window.set_layout_index(layout_to_index(settings.initial_layout));
    window.set_dock_edge_index(dock_edge_to_index(settings.dock_edge));

    let resolved_theme =
        theme::resolve_ui_theme(settings.theme_id.as_deref(), &settings.background_color_hex);
    window.set_bg_preview_color(hex_to_color(&resolved_theme.bg_hex));
    window.set_theme_preview_color(hex_to_color(&resolved_theme.accent_hex));
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
    settings.start_in_tray = window.get_start_in_tray_setting();
    settings.locked_layout = window.get_locked_layout_setting();
    settings.show_app_icons = window.get_show_app_icons_setting();
    settings.theme_id = theme::theme_id_by_index(window.get_theme_index());
    settings.background_color_hex = window.get_bg_color_hex().to_string();
    let img_path = window.get_bg_image_path().to_string();
    settings.background_image_path = if img_path.is_empty() {
        None
    } else {
        Some(img_path)
    };

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

    settings.dock_edge = index_to_dock_edge(window.get_dock_edge_index());
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

fn dock_edge_to_index(edge: Option<DockEdge>) -> i32 {
    match edge {
        None => 0,
        Some(DockEdge::Left) => 1,
        Some(DockEdge::Right) => 2,
        Some(DockEdge::Top) => 3,
        Some(DockEdge::Bottom) => 4,
    }
}

fn index_to_dock_edge(index: i32) -> Option<DockEdge> {
    match index {
        1 => Some(DockEdge::Left),
        2 => Some(DockEdge::Right),
        3 => Some(DockEdge::Top),
        4 => Some(DockEdge::Bottom),
        _ => None,
    }
}

fn hex_to_color(hex: &str) -> slint::Color {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("D2"), 16).unwrap_or(0xD2);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("9A"), 16).unwrap_or(0x9A);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("5C"), 16).unwrap_or(0x5C);
    slint::Color::from_rgb_u8(red, green, blue)
}
