//! Helpers for synchronizing the Slint settings window with persisted settings.

use std::cell::RefCell;
use std::path::Path;

use panopticon::i18n;
use panopticon::layout::LayoutType;
use panopticon::settings::{
    AppSettings, BackgroundImageFit, DockEdge, RefreshPerformanceMode, ShortcutBindings,
    ThemeColorOverrides, ToolbarPosition, WindowGrouping, MIN_DOCK_COLUMN_THICKNESS,
    MIN_DOCK_ROW_THICKNESS, MIN_FIXED_WINDOW_HEIGHT, MIN_FIXED_WINDOW_WIDTH,
};
use panopticon::theme;
use slint::SharedString;
use slint::{ModelRc, VecModel};

use crate::{SettingsWindow, ThemePreviewData};

const MAX_THEME_PREVIEW_CARDS: usize = 48;

thread_local! {
    static THEME_PREVIEW_CACHE: RefCell<Option<Vec<ThemePreviewData>>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "settings draft mirrors boolean controls from the Slint settings window"
)]
pub(crate) struct SettingsWindowDraft {
    language_index: i32,
    always_on_top: bool,
    center_secondary_windows: bool,
    animate_transitions: bool,
    minimize_to_tray: bool,
    close_to_tray: bool,
    preserve_aspect_ratio: bool,
    hide_on_select: bool,
    show_toolbar: bool,
    toolbar_position_index: i32,
    show_window_info: bool,
    start_in_tray: bool,
    run_at_startup: bool,
    locked_layout: bool,
    lock_cell_resize: bool,
    show_app_icons: bool,
    thumbnail_render_scale_value: i32,
    theme_index: i32,
    theme_color_overrides: ThemeColorOverrides,
    background_color_hex: String,
    background_image_path: String,
    background_image_fit_index: i32,
    background_image_opacity_value: i32,
    fixed_width_value: i32,
    fixed_height_value: i32,
    dock_column_thickness_value: i32,
    dock_row_thickness_value: i32,
    dock_edge_index: i32,
    group_windows_index: i32,
    refresh_performance_mode_index: i32,
    refresh_index: i32,
    shortcuts: ShortcutBindings,
    layout_index: i32,
}

#[expect(
    clippy::too_many_lines,
    reason = "the settings panel snapshot is applied in one pass to keep UI state updates consistent"
)]
pub fn populate_settings_window(window: &SettingsWindow, settings: &AppSettings) {
    window.set_language_index(locale_to_index(settings.language));
    window.set_always_on_top_setting(settings.always_on_top);
    window.set_center_secondary_windows_setting(settings.center_secondary_windows);
    window.set_animate_transitions_setting(settings.animate_transitions);
    window.set_minimize_to_tray_setting(settings.minimize_to_tray);
    window.set_close_to_tray_setting(settings.close_to_tray);
    window.set_preserve_aspect_ratio_setting(settings.preserve_aspect_ratio);
    window.set_hide_on_select_setting(settings.hide_on_select);
    window.set_hide_on_select_enabled(settings.dock_edge.is_none());
    window.set_default_layout_enabled(settings.dock_edge.is_none());
    window.set_show_toolbar_setting(settings.show_toolbar);
    window.set_toolbar_position_index(toolbar_position_to_index(settings.toolbar_position));
    window.set_show_info_setting(settings.show_window_info);
    window.set_start_in_tray_setting(settings.start_in_tray);
    window.set_run_at_startup_setting(settings.run_at_startup);
    window.set_locked_layout_setting(settings.locked_layout);
    window.set_lock_cell_resize_setting(settings.lock_cell_resize);
    window.set_show_app_icons_setting(settings.show_app_icons);
    window.set_thumbnail_render_scale_value(i32::from(settings.thumbnail_render_scale_pct));
    window.set_theme_index(theme::theme_index(settings.theme_id.as_deref()));
    window.set_bg_color_hex(SharedString::from(&settings.background_color_hex));
    let (bg_red, bg_green, bg_blue) =
        super::rgb_components_from_hex(&settings.background_color_hex);
    window.set_bg_red_value(i32::from(bg_red));
    window.set_bg_green_value(i32::from(bg_green));
    window.set_bg_blue_value(i32::from(bg_blue));
    let next_bg_image_path = settings.background_image_path.as_deref().unwrap_or("");
    let previous_bg_image_path = window.get_bg_image_path().to_string();
    window.set_bg_image_path(SharedString::from(next_bg_image_path));
    window.set_bg_image_fit_index(background_fit_to_index(settings.background_image_fit));
    window.set_bg_image_opacity_value(i32::from(settings.background_image_opacity_pct));
    window.set_fixed_width_value(settings.fixed_width.unwrap_or(MIN_FIXED_WINDOW_WIDTH) as i32);
    window.set_fixed_height_value(settings.fixed_height.unwrap_or(MIN_FIXED_WINDOW_HEIGHT) as i32);
    window.set_dock_column_thickness_value(
        settings
            .dock_column_thickness
            .unwrap_or(MIN_DOCK_COLUMN_THICKNESS) as i32,
    );
    window.set_dock_row_thickness_value(
        settings
            .dock_row_thickness
            .unwrap_or(MIN_DOCK_ROW_THICKNESS) as i32,
    );
    window.set_refresh_index(refresh_to_index(settings.refresh_interval_ms));
    window.set_refresh_performance_mode_index(refresh_mode_to_index(
        settings.refresh_performance_mode,
    ));
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
    window.set_shortcut_open_command_palette(SharedString::from(
        &settings.shortcuts.open_command_palette,
    ));
    window.set_shortcut_global_activate(SharedString::from(
        settings.shortcuts.global_activate.as_deref().unwrap_or(""),
    ));
    window.set_shortcut_refresh_now(SharedString::from(&settings.shortcuts.refresh_now));
    window.set_shortcut_exit_app(SharedString::from(&settings.shortcuts.exit_app));
    window.set_alt_toolbar_shortcut_enabled(settings.shortcuts.alt_toggles_toolbar);
    if let Some((summary, detail)) = settings.shortcuts.conflict_banner() {
        window.set_has_shortcut_conflicts(true);
        window.set_shortcut_conflict_summary(SharedString::from(summary));
        window.set_shortcut_conflict_detail(SharedString::from(detail));
    } else {
        window.set_has_shortcut_conflicts(false);
        window.set_shortcut_conflict_summary(SharedString::from(""));
        window.set_shortcut_conflict_detail(SharedString::from(""));
    }

    let resolved_theme = theme::resolve_ui_theme(
        settings.theme_id.as_deref(),
        &settings.background_color_hex,
        &settings.theme_color_overrides,
    );
    window.set_bg_preview_color(hex_to_color(&settings.background_color_hex));
    window.set_theme_preview_color(hex_to_color(&resolved_theme.accent_hex));
    populate_theme_colour_overrides(window, settings, &resolved_theme);
    if previous_bg_image_path != next_bg_image_path {
        window.set_bg_image_preview(load_image_preview(
            settings.background_image_path.as_deref(),
        ));
    }
    window.set_theme_preview_model(build_theme_preview_model());
}

pub fn apply_settings_window_changes(window: &SettingsWindow, settings: &mut AppSettings) {
    let draft = read_settings_window_draft(window);
    apply_settings_draft(settings, &draft);
}

pub(crate) fn read_settings_window_draft(window: &SettingsWindow) -> SettingsWindowDraft {
    SettingsWindowDraft {
        language_index: window.get_language_index(),
        always_on_top: window.get_always_on_top_setting(),
        center_secondary_windows: window.get_center_secondary_windows_setting(),
        animate_transitions: window.get_animate_transitions_setting(),
        minimize_to_tray: window.get_minimize_to_tray_setting(),
        close_to_tray: window.get_close_to_tray_setting(),
        preserve_aspect_ratio: window.get_preserve_aspect_ratio_setting(),
        hide_on_select: window.get_hide_on_select_setting(),
        show_toolbar: window.get_show_toolbar_setting(),
        toolbar_position_index: window.get_toolbar_position_index(),
        show_window_info: window.get_show_info_setting(),
        start_in_tray: window.get_start_in_tray_setting(),
        run_at_startup: window.get_run_at_startup_setting(),
        locked_layout: window.get_locked_layout_setting(),
        lock_cell_resize: window.get_lock_cell_resize_setting(),
        show_app_icons: window.get_show_app_icons_setting(),
        thumbnail_render_scale_value: window.get_thumbnail_render_scale_value(),
        theme_index: window.get_theme_index(),
        theme_color_overrides: ThemeColorOverrides {
            accent_hex: optional_theme_override(&window.get_theme_accent_hex()),
            surface_hex: optional_theme_override(&window.get_theme_surface_hex()),
            card_hex: optional_theme_override(&window.get_theme_card_hex()),
            text_hex: optional_theme_override(&window.get_theme_text_hex()),
            muted_hex: optional_theme_override(&window.get_theme_muted_hex()),
            border_hex: optional_theme_override(&window.get_theme_border_hex()),
        },
        background_color_hex: window.get_bg_color_hex().to_string(),
        background_image_path: window.get_bg_image_path().to_string(),
        background_image_fit_index: window.get_bg_image_fit_index(),
        background_image_opacity_value: window.get_bg_image_opacity_value(),
        fixed_width_value: window.get_fixed_width_value(),
        fixed_height_value: window.get_fixed_height_value(),
        dock_column_thickness_value: window.get_dock_column_thickness_value(),
        dock_row_thickness_value: window.get_dock_row_thickness_value(),
        dock_edge_index: window.get_dock_edge_index(),
        group_windows_index: window.get_group_windows_index(),
        refresh_performance_mode_index: window.get_refresh_performance_mode_index(),
        refresh_index: window.get_refresh_index(),
        shortcuts: ShortcutBindings {
            layout_grid: window.get_shortcut_layout_grid().to_string(),
            layout_mosaic: window.get_shortcut_layout_mosaic().to_string(),
            layout_bento: window.get_shortcut_layout_bento().to_string(),
            layout_fibonacci: window.get_shortcut_layout_fibonacci().to_string(),
            layout_columns: window.get_shortcut_layout_columns().to_string(),
            layout_row: window.get_shortcut_layout_row().to_string(),
            layout_column: window.get_shortcut_layout_column().to_string(),
            reset_layout: window.get_shortcut_reset_layout().to_string(),
            cycle_layout: window.get_shortcut_cycle_layout().to_string(),
            cycle_theme: window.get_shortcut_cycle_theme().to_string(),
            toggle_animations: window.get_shortcut_toggle_animations().to_string(),
            toggle_toolbar: window.get_shortcut_toggle_toolbar().to_string(),
            toggle_window_info: window.get_shortcut_toggle_window_info().to_string(),
            toggle_always_on_top: window.get_shortcut_toggle_always_on_top().to_string(),
            open_settings: window.get_shortcut_open_settings().to_string(),
            open_menu: window.get_shortcut_open_menu().to_string(),
            open_command_palette: window.get_shortcut_open_command_palette().to_string(),
            global_activate: Some(window.get_shortcut_global_activate().to_string()),
            refresh_now: window.get_shortcut_refresh_now().to_string(),
            exit_app: window.get_shortcut_exit_app().to_string(),
            alt_toggles_toolbar: window.get_alt_toolbar_shortcut_enabled(),
        },
        layout_index: window.get_layout_index(),
    }
}

pub(crate) fn apply_settings_draft(settings: &mut AppSettings, draft: &SettingsWindowDraft) {
    settings.language = index_to_locale(draft.language_index);
    settings.always_on_top = draft.always_on_top;
    settings.center_secondary_windows = draft.center_secondary_windows;
    settings.animate_transitions = draft.animate_transitions;
    settings.minimize_to_tray = draft.minimize_to_tray;
    settings.close_to_tray = draft.close_to_tray;
    settings.preserve_aspect_ratio = draft.preserve_aspect_ratio;
    settings.hide_on_select = draft.hide_on_select;
    settings.show_toolbar = draft.show_toolbar;
    settings.toolbar_position = index_to_toolbar_position(draft.toolbar_position_index);
    settings.show_window_info = draft.show_window_info;
    settings.start_in_tray = draft.start_in_tray;
    settings.run_at_startup = draft.run_at_startup;
    settings.locked_layout = draft.locked_layout;
    settings.lock_cell_resize = draft.lock_cell_resize;
    settings.show_app_icons = draft.show_app_icons;
    settings.thumbnail_render_scale_pct = draft.thumbnail_render_scale_value.clamp(25, 100) as u8;
    settings.theme_color_overrides = draft.theme_color_overrides.clone();
    let next_theme_id = theme::theme_id_by_index(draft.theme_index);
    let theme_changed = settings.theme_id != next_theme_id;
    settings.theme_id = next_theme_id;
    settings.background_color_hex = if theme_changed && settings.theme_id.is_some() {
        theme::theme_base_background_hex(settings.theme_id.as_deref(), &draft.background_color_hex)
    } else {
        draft.background_color_hex.clone()
    };
    settings.background_image_path = if draft.background_image_path.is_empty() {
        None
    } else {
        Some(draft.background_image_path.clone())
    };
    settings.background_image_fit = index_to_background_fit(draft.background_image_fit_index);
    settings.background_image_opacity_pct =
        draft.background_image_opacity_value.clamp(0, 100) as u8;

    settings.fixed_width =
        optional_dimension_with_min(draft.fixed_width_value, MIN_FIXED_WINDOW_WIDTH);
    settings.fixed_height =
        optional_dimension_with_min(draft.fixed_height_value, MIN_FIXED_WINDOW_HEIGHT);
    settings.dock_column_thickness =
        optional_dimension_with_min(draft.dock_column_thickness_value, MIN_DOCK_COLUMN_THICKNESS);
    settings.dock_row_thickness =
        optional_dimension_with_min(draft.dock_row_thickness_value, MIN_DOCK_ROW_THICKNESS);

    settings.dock_edge = index_to_dock_edge(draft.dock_edge_index);
    settings.group_windows_by = index_to_grouping(draft.group_windows_index);
    settings.refresh_performance_mode = index_to_refresh_mode(draft.refresh_performance_mode_index);
    settings.refresh_interval_ms = settings
        .refresh_performance_mode
        .fixed_interval_ms()
        .unwrap_or_else(|| index_to_refresh(draft.refresh_index));
    settings.shortcuts = draft.shortcuts.clone();
    settings.initial_layout = index_to_layout(draft.layout_index);
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

const fn toolbar_position_to_index(position: ToolbarPosition) -> i32 {
    match position {
        ToolbarPosition::Top => 0,
        ToolbarPosition::Bottom => 1,
    }
}

const fn index_to_toolbar_position(index: i32) -> ToolbarPosition {
    match index {
        0 => ToolbarPosition::Top,
        _ => ToolbarPosition::Bottom,
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

const fn refresh_mode_to_index(mode: RefreshPerformanceMode) -> i32 {
    match mode {
        RefreshPerformanceMode::Realtime => 0,
        RefreshPerformanceMode::Balanced => 1,
        RefreshPerformanceMode::BatterySaver => 2,
        RefreshPerformanceMode::Manual => 3,
    }
}

const fn index_to_refresh_mode(index: i32) -> RefreshPerformanceMode {
    match index {
        0 => RefreshPerformanceMode::Realtime,
        2 => RefreshPerformanceMode::BatterySaver,
        3 => RefreshPerformanceMode::Manual,
        _ => RefreshPerformanceMode::Balanced,
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

fn optional_dimension_with_min(value: i32, minimum: u32) -> Option<u32> {
    let value = u32::try_from(value).ok()?;
    (value > 0).then_some(value.max(minimum))
}

fn hex_to_color(hex: &str) -> slint::Color {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("D2"), 16).unwrap_or(0xD2);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("9A"), 16).unwrap_or(0x9A);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("5C"), 16).unwrap_or(0x5C);
    slint::Color::from_rgb_u8(red, green, blue)
}

fn build_theme_preview_model() -> ModelRc<ThemePreviewData> {
    THEME_PREVIEW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let previews = cache.get_or_insert_with(build_theme_preview_rows).clone();
        ModelRc::new(VecModel::from(previews))
    })
}

fn build_theme_preview_rows() -> Vec<ThemePreviewData> {
    let preset_preview_count = theme::theme_presets()
        .len()
        .min(MAX_THEME_PREVIEW_CARDS.saturating_sub(1));
    let mut previews = Vec::with_capacity(preset_preview_count + 1);
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
            .take(preset_preview_count)
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

    previews
}

fn populate_theme_colour_overrides(
    window: &SettingsWindow,
    settings: &AppSettings,
    resolved_theme: &theme::UiTheme,
) {
    set_theme_override_field(
        window,
        settings.theme_color_overrides.accent_hex.as_deref(),
        &resolved_theme.accent_hex,
        SettingsWindow::set_theme_accent_hex,
        SettingsWindow::set_theme_accent_placeholder,
        SettingsWindow::set_theme_accent_preview_color,
    );
    set_theme_override_field(
        window,
        settings.theme_color_overrides.surface_hex.as_deref(),
        &resolved_theme.surface_hex,
        SettingsWindow::set_theme_surface_hex,
        SettingsWindow::set_theme_surface_placeholder,
        SettingsWindow::set_theme_surface_preview_color,
    );
    set_theme_override_field(
        window,
        settings.theme_color_overrides.card_hex.as_deref(),
        &resolved_theme.card_bg_hex,
        SettingsWindow::set_theme_card_hex,
        SettingsWindow::set_theme_card_placeholder,
        SettingsWindow::set_theme_card_preview_color,
    );
    set_theme_override_field(
        window,
        settings.theme_color_overrides.text_hex.as_deref(),
        &resolved_theme.text_hex,
        SettingsWindow::set_theme_text_hex,
        SettingsWindow::set_theme_text_placeholder,
        SettingsWindow::set_theme_text_preview_color,
    );
    set_theme_override_field(
        window,
        settings.theme_color_overrides.muted_hex.as_deref(),
        &resolved_theme.muted_hex,
        SettingsWindow::set_theme_muted_hex,
        SettingsWindow::set_theme_muted_placeholder,
        SettingsWindow::set_theme_muted_preview_color,
    );
    set_theme_override_field(
        window,
        settings.theme_color_overrides.border_hex.as_deref(),
        &resolved_theme.border_hex,
        SettingsWindow::set_theme_border_hex,
        SettingsWindow::set_theme_border_placeholder,
        SettingsWindow::set_theme_border_preview_color,
    );
}

fn set_theme_override_field(
    window: &SettingsWindow,
    override_hex: Option<&str>,
    resolved_hex: &str,
    set_value: fn(&SettingsWindow, SharedString),
    set_placeholder: fn(&SettingsWindow, SharedString),
    set_preview: fn(&SettingsWindow, slint::Color),
) {
    set_value(window, SharedString::from(override_hex.unwrap_or_default()));
    set_placeholder(window, SharedString::from(resolved_hex));
    set_preview(window, hex_to_color(override_hex.unwrap_or(resolved_hex)));
}

fn optional_theme_override(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.is_empty()
        || trimmed.len() != 6
        || !trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return None;
    }

    Some(trimmed.to_ascii_uppercase())
}

fn load_image_preview(path: Option<&str>) -> slint::Image {
    path.filter(|value| !value.trim().is_empty())
        .and_then(|value| slint::Image::load_from_path(Path::new(value)).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_settings_draft, background_fit_to_index, build_theme_preview_model,
        index_to_background_fit, SettingsWindowDraft, MAX_THEME_PREVIEW_CARDS,
    };
    use panopticon::settings::{
        AppSettings, BackgroundImageFit, RefreshPerformanceMode, ShortcutBindings,
        ThemeColorOverrides, MIN_DOCK_COLUMN_THICKNESS, MIN_DOCK_ROW_THICKNESS,
        MIN_FIXED_WINDOW_HEIGHT, MIN_FIXED_WINDOW_WIDTH,
    };
    use slint::Model;

    fn base_draft() -> SettingsWindowDraft {
        SettingsWindowDraft {
            language_index: 0,
            always_on_top: false,
            center_secondary_windows: true,
            animate_transitions: true,
            minimize_to_tray: true,
            close_to_tray: true,
            preserve_aspect_ratio: true,
            hide_on_select: true,
            show_toolbar: true,
            toolbar_position_index: 0,
            show_window_info: true,
            start_in_tray: false,
            run_at_startup: false,
            locked_layout: false,
            lock_cell_resize: false,
            show_app_icons: true,
            thumbnail_render_scale_value: 100,
            theme_index: 0,
            theme_color_overrides: ThemeColorOverrides::default(),
            background_color_hex: "181513".to_owned(),
            background_image_path: String::new(),
            background_image_fit_index: 0,
            background_image_opacity_value: 25,
            fixed_width_value: MIN_FIXED_WINDOW_WIDTH as i32,
            fixed_height_value: MIN_FIXED_WINDOW_HEIGHT as i32,
            dock_column_thickness_value: MIN_DOCK_COLUMN_THICKNESS as i32,
            dock_row_thickness_value: MIN_DOCK_ROW_THICKNESS as i32,
            dock_edge_index: 0,
            group_windows_index: 0,
            refresh_performance_mode_index: 3,
            refresh_index: 1,
            shortcuts: ShortcutBindings::default(),
            layout_index: 0,
        }
    }

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

    #[test]
    fn theme_preview_model_is_capped_for_settings_rendering() {
        assert_eq!(
            build_theme_preview_model().row_count(),
            MAX_THEME_PREVIEW_CARDS
        );
    }

    #[test]
    fn draft_applies_clamped_numeric_fields() {
        let mut settings = AppSettings::default();
        let mut draft = base_draft();
        draft.thumbnail_render_scale_value = 5;
        draft.background_image_opacity_value = 250;
        draft.fixed_width_value = 10;
        draft.fixed_height_value = 10;
        draft.dock_column_thickness_value = 10;
        draft.dock_row_thickness_value = 10;

        apply_settings_draft(&mut settings, &draft);

        assert_eq!(settings.thumbnail_render_scale_pct, 25);
        assert_eq!(settings.background_image_opacity_pct, 100);
        assert_eq!(settings.fixed_width, Some(MIN_FIXED_WINDOW_WIDTH));
        assert_eq!(settings.fixed_height, Some(MIN_FIXED_WINDOW_HEIGHT));
        assert_eq!(
            settings.dock_column_thickness,
            Some(MIN_DOCK_COLUMN_THICKNESS)
        );
        assert_eq!(settings.dock_row_thickness, Some(MIN_DOCK_ROW_THICKNESS));
    }

    #[test]
    fn draft_fixed_refresh_modes_override_manual_interval_index() {
        let mut settings = AppSettings::default();
        let mut draft = base_draft();
        draft.refresh_performance_mode_index = 0;
        draft.refresh_index = 3;

        apply_settings_draft(&mut settings, &draft);

        assert_eq!(
            settings.refresh_performance_mode,
            RefreshPerformanceMode::Realtime
        );
        assert_eq!(settings.refresh_interval_ms, 1_000);
    }

    #[test]
    fn draft_invalid_theme_overrides_are_ignored() {
        let mut settings = AppSettings::default();
        let mut draft = base_draft();
        draft.theme_color_overrides = ThemeColorOverrides {
            accent_hex: Some("not-a-colour".to_owned()),
            surface_hex: Some("#12345".to_owned()),
            card_hex: Some("123456".to_owned()),
            text_hex: None,
            muted_hex: None,
            border_hex: None,
        };

        // Simulates the Slint adapter, where invalid values are normalised
        // before the draft reaches pure settings application.
        draft.theme_color_overrides.accent_hex = super::optional_theme_override(
            draft
                .theme_color_overrides
                .accent_hex
                .as_deref()
                .unwrap_or_default(),
        );
        draft.theme_color_overrides.surface_hex = super::optional_theme_override(
            draft
                .theme_color_overrides
                .surface_hex
                .as_deref()
                .unwrap_or_default(),
        );
        draft.theme_color_overrides.card_hex = super::optional_theme_override(
            draft
                .theme_color_overrides
                .card_hex
                .as_deref()
                .unwrap_or_default(),
        );

        apply_settings_draft(&mut settings, &draft);

        assert_eq!(settings.theme_color_overrides.accent_hex, None);
        assert_eq!(settings.theme_color_overrides.surface_hex, None);
        assert_eq!(
            settings.theme_color_overrides.card_hex,
            Some("123456".to_owned())
        );
    }
}
