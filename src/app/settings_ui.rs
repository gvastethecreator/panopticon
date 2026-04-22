//! Helpers for synchronizing the Slint settings window with persisted settings.

use std::path::Path;

use panopticon::i18n;
use panopticon::layout::LayoutType;
use panopticon::settings::{AppSettings, BackgroundImageFit, DockEdge, WindowGrouping};
use panopticon::theme;
use slint::SharedString;
use slint::{ModelRc, VecModel};

use crate::{SettingsWindow, ThemePreviewData};

pub fn populate_settings_window(window: &SettingsWindow, settings: &AppSettings) {
    window.set_language_index(locale_to_index(settings.language));
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
    window.set_lock_cell_resize_setting(settings.lock_cell_resize);
    window.set_show_app_icons_setting(settings.show_app_icons);
    window.set_theme_index(theme::theme_index(settings.theme_id.as_deref()));
    window.set_bg_color_hex(SharedString::from(&settings.background_color_hex));
    let (bg_red, bg_green, bg_blue) = rgb_components_from_hex(&settings.background_color_hex);
    window.set_bg_red_value(i32::from(bg_red));
    window.set_bg_green_value(i32::from(bg_green));
    window.set_bg_blue_value(i32::from(bg_blue));
    let next_bg_image_path = settings.background_image_path.as_deref().unwrap_or("");
    let previous_bg_image_path = window.get_bg_image_path().to_string();
    window.set_bg_image_path(SharedString::from(next_bg_image_path));
    window.set_bg_image_fit_index(background_fit_to_index(settings.background_image_fit));
    window.set_bg_image_opacity_value(i32::from(settings.background_image_opacity_pct));
    window.set_fixed_width_value(settings.fixed_width.unwrap_or(0) as i32);
    window.set_fixed_height_value(settings.fixed_height.unwrap_or(0) as i32);
    window.set_refresh_index(refresh_to_index(settings.refresh_interval_ms));
    window.set_layout_index(layout_to_index(settings.initial_layout));
    window.set_dock_edge_index(dock_edge_to_index(settings.dock_edge));
    window.set_group_windows_index(grouping_to_index(settings.group_windows_by));
    window.set_shortcut_layout_grid(SharedString::from(&settings.shortcuts.layout_grid));
    window.set_shortcut_layout_mosaic(SharedString::from(&settings.shortcuts.layout_mosaic));
    window.set_shortcut_layout_bento(SharedString::from(&settings.shortcuts.layout_bento));
    window.set_shortcut_layout_fibonacci(SharedString::from(&settings.shortcuts.layout_fibonacci));
    window.set_shortcut_layout_columns(SharedString::from(&settings.shortcuts.layout_columns));
    window.set_shortcut_layout_row(SharedString::from(&settings.shortcuts.layout_row));
    window.set_shortcut_layout_column(SharedString::from(&settings.shortcuts.layout_column));
    window.set_shortcut_reset_layout(SharedString::from(&settings.shortcuts.reset_layout));
    window.set_shortcut_cycle_layout(SharedString::from(&settings.shortcuts.cycle_layout));
    window.set_shortcut_cycle_theme(SharedString::from(&settings.shortcuts.cycle_theme));
    window
        .set_shortcut_toggle_animations(SharedString::from(&settings.shortcuts.toggle_animations));
    window.set_shortcut_toggle_toolbar(SharedString::from(&settings.shortcuts.toggle_toolbar));
    window.set_shortcut_toggle_window_info(SharedString::from(
        &settings.shortcuts.toggle_window_info,
    ));
    window.set_shortcut_toggle_always_on_top(SharedString::from(
        &settings.shortcuts.toggle_always_on_top,
    ));
    window.set_shortcut_open_settings(SharedString::from(&settings.shortcuts.open_settings));
    window.set_shortcut_open_menu(SharedString::from(&settings.shortcuts.open_menu));
    window.set_shortcut_global_activate(SharedString::from(
        settings.shortcuts.global_activate.as_deref().unwrap_or(""),
    ));
    window.set_shortcut_refresh_now(SharedString::from(&settings.shortcuts.refresh_now));
    window.set_shortcut_exit_app(SharedString::from(&settings.shortcuts.exit_app));
    window.set_alt_toolbar_shortcut_enabled(settings.shortcuts.alt_toggles_toolbar);

    let resolved_theme =
        theme::resolve_ui_theme(settings.theme_id.as_deref(), &settings.background_color_hex);
    window.set_bg_preview_color(hex_to_color(&settings.background_color_hex));
    window.set_theme_preview_color(hex_to_color(&resolved_theme.accent_hex));
    if previous_bg_image_path != next_bg_image_path {
        window.set_bg_image_preview(load_image_preview(
            settings.background_image_path.as_deref(),
        ));
    }
    window.set_theme_preview_model(build_theme_preview_model());
}

pub fn apply_settings_window_changes(
    window: &SettingsWindow,
    settings: &mut AppSettings,
) -> LayoutType {
    settings.language = index_to_locale(window.get_language_index());
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
    settings.lock_cell_resize = window.get_lock_cell_resize_setting();
    settings.show_app_icons = window.get_show_app_icons_setting();
    let next_theme_id = theme::theme_id_by_index(window.get_theme_index());
    let theme_changed = settings.theme_id != next_theme_id;
    settings.theme_id = next_theme_id;
    let requested_bg_hex = window.get_bg_color_hex().to_string();
    settings.background_color_hex = if theme_changed && settings.theme_id.is_some() {
        theme::theme_base_background_hex(settings.theme_id.as_deref(), &requested_bg_hex)
    } else {
        requested_bg_hex
    };
    let img_path = window.get_bg_image_path().to_string();
    settings.background_image_path = if img_path.is_empty() {
        None
    } else {
        Some(img_path)
    };
    settings.background_image_fit = index_to_background_fit(window.get_bg_image_fit_index());
    settings.background_image_opacity_pct = window.get_bg_image_opacity_value().clamp(0, 100) as u8;

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
    settings.group_windows_by = index_to_grouping(window.get_group_windows_index());
    settings.refresh_interval_ms = index_to_refresh(window.get_refresh_index());
    settings.shortcuts.layout_grid = window.get_shortcut_layout_grid().to_string();
    settings.shortcuts.layout_mosaic = window.get_shortcut_layout_mosaic().to_string();
    settings.shortcuts.layout_bento = window.get_shortcut_layout_bento().to_string();
    settings.shortcuts.layout_fibonacci = window.get_shortcut_layout_fibonacci().to_string();
    settings.shortcuts.layout_columns = window.get_shortcut_layout_columns().to_string();
    settings.shortcuts.layout_row = window.get_shortcut_layout_row().to_string();
    settings.shortcuts.layout_column = window.get_shortcut_layout_column().to_string();
    settings.shortcuts.reset_layout = window.get_shortcut_reset_layout().to_string();
    settings.shortcuts.cycle_layout = window.get_shortcut_cycle_layout().to_string();
    settings.shortcuts.cycle_theme = window.get_shortcut_cycle_theme().to_string();
    settings.shortcuts.toggle_animations = window.get_shortcut_toggle_animations().to_string();
    settings.shortcuts.toggle_toolbar = window.get_shortcut_toggle_toolbar().to_string();
    settings.shortcuts.toggle_window_info = window.get_shortcut_toggle_window_info().to_string();
    settings.shortcuts.toggle_always_on_top =
        window.get_shortcut_toggle_always_on_top().to_string();
    settings.shortcuts.open_settings = window.get_shortcut_open_settings().to_string();
    settings.shortcuts.open_menu = window.get_shortcut_open_menu().to_string();
    settings.shortcuts.global_activate = Some(window.get_shortcut_global_activate().to_string());
    settings.shortcuts.refresh_now = window.get_shortcut_refresh_now().to_string();
    settings.shortcuts.exit_app = window.get_shortcut_exit_app().to_string();
    settings.shortcuts.alt_toggles_toolbar = window.get_alt_toolbar_shortcut_enabled();
    let layout = index_to_layout(window.get_layout_index());
    settings.initial_layout = layout;
    layout
}

pub(crate) const fn background_fit_to_index(fit: BackgroundImageFit) -> i32 {
    match fit {
        BackgroundImageFit::Cover => 0,
        BackgroundImageFit::Contain => 1,
        BackgroundImageFit::Fill => 2,
        BackgroundImageFit::Preserve => 3,
    }
}

const fn locale_to_index(locale: i18n::Locale) -> i32 {
    match locale {
        i18n::Locale::English => 0,
        i18n::Locale::Spanish => 1,
    }
}

const fn index_to_locale(index: i32) -> i18n::Locale {
    match index {
        1 => i18n::Locale::Spanish,
        _ => i18n::Locale::English,
    }
}

fn index_to_background_fit(index: i32) -> BackgroundImageFit {
    match index {
        1 => BackgroundImageFit::Contain,
        2 => BackgroundImageFit::Fill,
        3 => BackgroundImageFit::Preserve,
        _ => BackgroundImageFit::Cover,
    }
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

fn grouping_to_index(grouping: WindowGrouping) -> i32 {
    match grouping {
        WindowGrouping::None => 0,
        WindowGrouping::Application => 1,
        WindowGrouping::Monitor => 2,
        WindowGrouping::WindowTitle => 3,
        WindowGrouping::ClassName => 4,
    }
}

fn index_to_grouping(index: i32) -> WindowGrouping {
    match index {
        1 => WindowGrouping::Application,
        2 => WindowGrouping::Monitor,
        3 => WindowGrouping::WindowTitle,
        4 => WindowGrouping::ClassName,
        _ => WindowGrouping::None,
    }
}

fn hex_to_color(hex: &str) -> slint::Color {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("D2"), 16).unwrap_or(0xD2);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("9A"), 16).unwrap_or(0x9A);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("5C"), 16).unwrap_or(0x5C);
    slint::Color::from_rgb_u8(red, green, blue)
}

fn rgb_components_from_hex(hex: &str) -> (u8, u8, u8) {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("18"), 16).unwrap_or(0x18);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("15"), 16).unwrap_or(0x15);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("13"), 16).unwrap_or(0x13);
    (red, green, blue)
}

fn build_theme_preview_model() -> ModelRc<ThemePreviewData> {
    let mut previews = Vec::with_capacity(theme::theme_presets().len() + 1);
    previews.push(ThemePreviewData {
        index: 0,
        label: SharedString::from(i18n::t("theme.classic_name")),
        subtitle: SharedString::from(i18n::t("theme.classic_subtitle")),
        bg: hex_to_color("181513"),
        surface: hex_to_color("221E1C"),
        accent: hex_to_color("D29A5C"),
        text: hex_to_color("E6E2DE"),
    });

    previews.extend(
        theme::theme_presets()
            .iter()
            .enumerate()
            .map(|(offset, preset)| ThemePreviewData {
                index: offset as i32 + 1,
                label: SharedString::from(preset.label.clone()),
                subtitle: SharedString::from(preset.id.clone()),
                bg: hex_to_color(&preset.ui.bg_hex),
                surface: hex_to_color(&preset.ui.surface_hex),
                accent: hex_to_color(&preset.ui.accent_hex),
                text: hex_to_color(&preset.ui.text_hex),
            }),
    );

    ModelRc::new(VecModel::from(previews))
}

fn load_image_preview(path: Option<&str>) -> slint::Image {
    path.filter(|value| !value.trim().is_empty())
        .and_then(|value| slint::Image::load_from_path(Path::new(value)).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{background_fit_to_index, index_to_background_fit};
    use panopticon::settings::BackgroundImageFit;

    #[test]
    fn background_fit_indices_roundtrip_all_supported_modes() {
        let cases = [
            (BackgroundImageFit::Cover, 0),
            (BackgroundImageFit::Contain, 1),
            (BackgroundImageFit::Fill, 2),
            (BackgroundImageFit::Preserve, 3),
        ];

        for (fit, index) in cases {
            assert_eq!(background_fit_to_index(fit), index);
            assert_eq!(index_to_background_fit(index), fit);
        }
    }

    #[test]
    fn unknown_background_fit_index_falls_back_to_cover() {
        assert_eq!(index_to_background_fit(-1), BackgroundImageFit::Cover);
        assert_eq!(index_to_background_fit(99), BackgroundImageFit::Cover);
    }
}
