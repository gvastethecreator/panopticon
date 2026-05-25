//! Main-window presentation helpers.
//!
//! Owns the UI-facing translation from runtime state to Slint properties,
//! leaving layout orchestration to `model_sync`.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use panopticon::i18n;
use panopticon::layout::ScrollDirection;
use panopticon::settings::{AppSettings, ToolbarPosition};
use panopticon::theme as theme_catalog;
use panopticon::window_ops::active_filter_summary;
use slint::Model;
use slint::SharedString;

use crate::{
    AppState, MainWindow, ABOUT_WIN, COMMAND_PALETTE_WIN, SETTINGS_WIN, TAG_DIALOG_WIN,
    THEME_TRANSITION_DURATION_MS,
};

use super::settings::ui::background_fit_to_index;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScrollPresentation {
    horizontal: bool,
    vertical: bool,
    content_extent: i32,
    viewport_x: f32,
    viewport_y: f32,
}

impl ScrollPresentation {
    #[must_use]
    pub(crate) fn new(
        scroll_direction: ScrollDirection,
        content_extent: i32,
        viewport_x: f32,
        viewport_y: f32,
    ) -> Self {
        Self {
            horizontal: scroll_direction == ScrollDirection::Horizontal,
            vertical: scroll_direction == ScrollDirection::Vertical,
            content_extent,
            viewport_x,
            viewport_y,
        }
    }
}

pub(crate) fn apply_scroll_presentation(win: &MainWindow, presentation: &ScrollPresentation) {
    win.set_scroll_horizontal(presentation.horizontal);
    win.set_scroll_vertical(presentation.vertical);
    win.set_content_width(presentation.content_extent as f32);
    win.set_content_height(presentation.content_extent as f32);
    win.set_viewport_x(presentation.viewport_x);
    win.set_viewport_y(presentation.viewport_y);
}

pub(crate) fn sync_settings_to_ui(win: &MainWindow, settings: &AppSettings) {
    win.set_show_toolbar(settings.show_toolbar);
    win.set_toolbar_on_top(matches!(settings.toolbar_position, ToolbarPosition::Top));
    win.set_show_window_info(settings.show_window_info);
    win.set_is_always_on_top(settings.always_on_top);
    win.set_animate_transitions(settings.animate_transitions);
    win.set_resize_locked(settings.locked_layout || settings.lock_cell_resize);
    win.set_canvas_background_color(canvas_background_color(settings));
    win.set_background_image_fit_index(background_fit_to_index(settings.background_image_fit));
    win.set_background_image_opacity(settings.background_image_opacity_pct as f32 / 100.0);
    win.set_empty_welcome_dismissed(settings.dismissed_empty_state_welcome);
    win.set_refresh_label(SharedString::from(settings.refresh_status_label()));
    win.set_filters_label(SharedString::from(
        active_filter_summary(settings).unwrap_or_default(),
    ));

    let empty_context = derive_empty_state_context(settings);
    win.set_empty_message(SharedString::from(empty_context.message));
    win.set_empty_helper(SharedString::from(empty_context.helper));
    win.set_empty_status_summary(SharedString::from(empty_context.status_summary));
    win.set_empty_can_clear_filters(empty_context.can_clear_filters);
    win.set_empty_can_show_hidden(empty_context.can_show_hidden);
}

pub(crate) fn sync_main_window_metadata(win: &MainWindow, state: &AppState) {
    win.set_layout_label(SharedString::from(i18n::t(
        state.window_collection.current_layout.translation_key(),
    )));
    win.set_window_count(state.window_collection.windows.len() as i32);
    win.set_hidden_count(state.settings.hidden_app_entries().len() as i32);
}

pub(crate) fn sync_theme_target(state: &mut AppState) {
    let desired = theme_catalog::resolve_ui_theme(
        state.settings.theme_id.as_deref(),
        &state.settings.background_color_hex,
        &state.settings.theme_color_overrides,
    );
    let already_targeting = state
        .theme
        .theme_animation
        .as_ref()
        .is_some_and(|animation| animation.to == desired);

    if already_targeting || state.theme.current_theme == desired {
        return;
    }

    state.theme.theme_animation = Some(crate::ThemeAnimation {
        from_rgb: theme_catalog::RgbThemeSnapshot::from_ui_theme(&state.theme.current_theme),
        to_rgb: theme_catalog::RgbThemeSnapshot::from_ui_theme(&desired),
        to: desired,
        started_at: Instant::now(),
    });
}

pub(crate) fn advance_theme_animation(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let mut state_ref = state.borrow_mut();
    let Some(animation) = state_ref.theme.theme_animation.as_ref() else {
        return;
    };

    let from_rgb = animation.from_rgb;
    let to_rgb = animation.to_rgb;
    let target_theme = animation.to.clone();
    let started_at = animation.started_at;

    let elapsed_ms = started_at.elapsed().as_millis() as u32;
    let progress = (elapsed_ms as f32 / THEME_TRANSITION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);
    let resolved = from_rgb.interpolate(&to_rgb, eased, &target_theme);
    state_ref.theme.current_theme = resolved;
    let palette_theme = target_theme.clone();
    if progress >= 1.0 {
        state_ref.theme.current_theme = target_theme;
        state_ref.theme.theme_animation = None;
    }
    let current = state_ref.theme.current_theme.clone();
    drop(state_ref);

    apply_theme_snapshot_everywhere(win, &current);
    // Keep palette dark/light mode pinned to the target theme while animating.
    // This avoids rapid scheme flips near luminance thresholds that can cause
    // occasional blank/flicker artifacts during theme transitions.
    apply_palette_color_scheme_everywhere(win, &palette_theme);
    refresh_thumbnail_accent_rows(state, win);
}

pub(crate) fn sync_background_image(state: &mut AppState, win: &MainWindow) {
    let desired = state.settings.background_image_path.clone();
    if state.theme.loaded_background_path == desired {
        return;
    }

    if let Some(path) = desired.as_deref() {
        match slint::Image::load_from_path(Path::new(path)) {
            Ok(image) => {
                win.set_background_image(image);
                state.theme.loaded_background_path = desired;
            }
            Err(error) => {
                tracing::warn!(%error, path, "failed to load background image");
                win.set_background_image(slint::Image::default());
                let _ = state.settings.update_persisted(|settings| {
                    settings.background_image_path = None;
                });
                state.theme.loaded_background_path = None;

                if let Err(save_error) = state.settings.save(state.workspace_name.as_deref()) {
                    tracing::warn!(
                        %save_error,
                        path,
                        "failed to persist cleared background image path"
                    );
                }
            }
        }
    } else {
        win.set_background_image(slint::Image::default());
        state.theme.loaded_background_path = None;
    }
}

struct EmptyStateContext {
    message: String,
    helper: String,
    status_summary: String,
    can_clear_filters: bool,
    can_show_hidden: bool,
}

fn derive_empty_state_context(settings: &AppSettings) -> EmptyStateContext {
    let has_filters = settings.active_monitor_filter.is_some()
        || settings.active_tag_filter.is_some()
        || settings.active_app_filter.is_some();
    let hidden_count = settings.hidden_app_entries().len();
    let can_show_hidden = hidden_count > 0;
    let filter_summary = active_filter_summary(settings).unwrap_or_default();

    let status_summary = match (has_filters, hidden_count) {
        (true, 0) if !filter_summary.is_empty() => format!("Active filters: {filter_summary}"),
        (true, 0) => "Active filters are restricting visible windows.".to_owned(),
        (false, count) if count > 0 => {
            if count == 1 {
                "1 app is currently hidden.".to_owned()
            } else {
                format!("{count} apps are currently hidden.")
            }
        }
        (true, count) => {
            let hidden_label = if count == 1 {
                "1 hidden app".to_owned()
            } else {
                format!("{count} hidden apps")
            };
            if filter_summary.is_empty() {
                format!("Active filters + {hidden_label}")
            } else {
                format!("{filter_summary} · {hidden_label}")
            }
        }
        _ => String::new(),
    };

    if has_filters {
        EmptyStateContext {
            message: "No windows match your current filters".to_owned(),
            helper: "Try clearing filters or refreshing to repopulate visible windows.".to_owned(),
            status_summary,
            can_clear_filters: true,
            can_show_hidden,
        }
    } else if can_show_hidden {
        EmptyStateContext {
            message: "All tracked windows are hidden".to_owned(),
            helper: "Restore hidden apps to bring them back into the layout.".to_owned(),
            status_summary,
            can_clear_filters: false,
            can_show_hidden: true,
        }
    } else {
        EmptyStateContext {
            message: i18n::t("ui.empty_message").to_owned(),
            helper: i18n::t("ui.empty_helper").to_owned(),
            status_summary,
            can_clear_filters: false,
            can_show_hidden: false,
        }
    }
}

fn canvas_background_color(settings: &AppSettings) -> slint::Color {
    let (red, green, blue) =
        super::settings::rgb_components_from_hex(&settings.background_color_hex);
    slint::Color::from_argb_u8(255, red, green, blue)
}

fn apply_theme_snapshot_everywhere(win: &MainWindow, resolved: &theme_catalog::UiTheme) {
    super::theme_ui::apply_main_window_theme_snapshot(win, resolved);
    SETTINGS_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_settings_window_theme_snapshot(window, resolved);
        }
    });
    TAG_DIALOG_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_tag_dialog_theme_snapshot(window, resolved);
        }
    });
    ABOUT_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_about_window_theme_snapshot(window, resolved);
        }
    });
    COMMAND_PALETTE_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_command_palette_window_theme_snapshot(window, resolved);
        }
    });
}

fn apply_palette_color_scheme_everywhere(win: &MainWindow, resolved: &theme_catalog::UiTheme) {
    super::theme_ui::apply_main_window_palette_color_scheme(win, resolved);
    SETTINGS_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_settings_window_palette_color_scheme(window, resolved);
        }
    });
    TAG_DIALOG_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_tag_dialog_palette_color_scheme(window, resolved);
        }
    });
    ABOUT_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_about_window_palette_color_scheme(window, resolved);
        }
    });
    COMMAND_PALETTE_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            super::theme_ui::apply_command_palette_window_palette_color_scheme(window, resolved);
        }
    });
}

fn refresh_thumbnail_accent_rows(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let state_ref = state.borrow();
    let model = win.get_thumbnails();
    if model.row_count() != state_ref.window_collection.windows.len() {
        return;
    }

    for (index, managed_window) in state_ref.window_collection.windows.iter().enumerate() {
        if let Some(mut item) = model.row_data(index) {
            item.accent_color = super::theme_ui::thumbnail_accent_color(
                &state_ref.settings,
                &state_ref.theme.current_theme,
                &managed_window.info.app_id,
            );
            model.set_row_data(index, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_background_color_parses_hex() {
        let color = canvas_background_color(&AppSettings {
            background_color_hex: "#ff0000".to_owned(),
            ..Default::default()
        });
        assert_eq!(color.red(), 255);
        assert_eq!(color.green(), 0);
        assert_eq!(color.blue(), 0);
    }
}
