//! Layout recomputation, Slint model synchronization, and thumbnail animation.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::i18n;
use panopticon::layout::ScrollDirection;
use panopticon::settings::{AppSettings, ToolbarPosition};
use panopticon::window_ops::active_filter_summary;
use slint::ComponentHandle;
use slint::Model;
use slint::SharedString;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use super::settings_ui::background_fit_to_index;
use super::theme_ui::sync_theme_target;
use crate::{AppState, MainWindow};

thread_local! {
    static RECOMPUTE_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
    static MODEL_SYNC_IN_PROGRESS: Cell<bool> = const { Cell::new(false) };
}

struct RecomputeGuard;

impl RecomputeGuard {
    fn enter() -> Option<Self> {
        let already_running = RECOMPUTE_IN_PROGRESS.with(|flag| {
            if flag.get() {
                true
            } else {
                flag.set(true);
                false
            }
        });
        if already_running {
            None
        } else {
            Some(Self)
        }
    }
}

impl Drop for RecomputeGuard {
    fn drop(&mut self) {
        RECOMPUTE_IN_PROGRESS.with(|flag| flag.set(false));
    }
}

struct ModelSyncGuard;

impl ModelSyncGuard {
    fn enter() -> Option<Self> {
        let already_running = MODEL_SYNC_IN_PROGRESS.with(|flag| {
            if flag.get() {
                true
            } else {
                flag.set(true);
                false
            }
        });
        if already_running {
            None
        } else {
            Some(Self)
        }
    }
}

impl Drop for ModelSyncGuard {
    fn drop(&mut self) {
        MODEL_SYNC_IN_PROGRESS.with(|flag| flag.set(false));
    }
}

pub(crate) fn recompute_and_update_ui(app_state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Some(_guard) = RecomputeGuard::enter() else {
        tracing::debug!("skipping nested recompute_and_update_ui invocation");
        return;
    };

    tracing::trace!("recompute checkpoint: entered");

    let mut state = app_state.borrow_mut();
    if state.window_collection.windows.is_empty() {
        state.theme.animation_started_at = None;
        sync_theme_target(&mut state);
        sync_settings_to_ui(win, &state.settings);
        sync_background_image(&mut state, win);
        drop(state);
        sync_model_to_slint(app_state, win);
        return;
    }

    let phys = win.window().size();
    let scale = win.window().scale_factor();
    let logical_w = (phys.width as f32 / scale).round() as i32;
    let logical_h = (phys.height as f32 / scale).round() as i32;
    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    let content_area = RECT {
        left: 0,
        top: 0,
        right: logical_w,
        bottom: (logical_h - toolbar_h).max(1),
    };

    let custom = state.settings.layout_custom(state.window_collection.current_layout).cloned();
    let (rects, separators) = super::layout_pipeline::compute_layout_rects(
        state.window_collection.current_layout,
        content_area,
        &state.window_collection.windows,
        custom.as_ref(),
    );
    tracing::trace!(
        window_count = state.window_collection.windows.len(),
        "recompute checkpoint: layout computed"
    );
    state.window_collection.separators = separators;

    let scroll_dir = state.window_collection.current_layout.scroll_direction();
    state.window_collection.content_extent = match scroll_dir {
        ScrollDirection::Horizontal => rects.iter().map(|rect| rect.right).max().unwrap_or(0),
        ScrollDirection::Vertical => rects.iter().map(|rect| rect.bottom).max().unwrap_or(0),
        ScrollDirection::None => 0,
    };

    let can_animate = state.settings.animate_transitions
        && !state.shell.hwnd.0.is_null()
        && unsafe { IsWindowVisible(state.shell.hwnd).as_bool() }
        && state.window_collection.drag_separator.is_none()
        && state
            .window_collection
            .windows
            .iter()
            .any(|managed_window| super::layout_pipeline::rect_has_area(managed_window.display_rect));

    let animation_needed = super::layout_pipeline::apply_layout_rects(
        &mut state.window_collection.windows,
        &rects,
        can_animate,
    );

    if animation_needed {
        state.theme.animation_started_at = Some(Instant::now());
    } else {
        state.theme.animation_started_at = None;
    }

    let scroll_h = scroll_dir == ScrollDirection::Horizontal;
    let scroll_v = scroll_dir == ScrollDirection::Vertical;
    win.set_scroll_horizontal(scroll_h);
    win.set_scroll_vertical(scroll_v);
    win.set_content_width(state.window_collection.content_extent as f32);
    win.set_content_height(state.window_collection.content_extent as f32);
    tracing::trace!("recompute checkpoint: scroll properties applied");
    let (clamped_x, clamped_y) = super::viewport_manager::clamp_offsets(
        scroll_dir,
        state.window_collection.content_extent,
        logical_w,
        content_area.bottom,
        win.get_viewport_x(),
        win.get_viewport_y(),
    );
    win.set_viewport_x(clamped_x);
    win.set_viewport_y(clamped_y);
    tracing::trace!("recompute checkpoint: viewport clamped");

    sync_theme_target(&mut state);
    tracing::trace!("recompute checkpoint: theme synced");
    sync_settings_to_ui(win, &state.settings);
    tracing::trace!("recompute checkpoint: settings synced");
    sync_background_image(&mut state, win);
    tracing::trace!("recompute checkpoint: background synced");

    drop(state);
    tracing::trace!("recompute reached pre-model-sync checkpoint");
    sync_model_to_slint(app_state, win);
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

#[allow(clippy::too_many_lines)]
pub(crate) fn sync_model_to_slint(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Some(_guard) = ModelSyncGuard::enter() else {
        tracing::debug!("skipping nested sync_model_to_slint invocation");
        return;
    };

    tracing::trace!("model sync checkpoint: entered");

    let mut state = state.borrow_mut();

    super::thumbnail_model_builder::sync_model_to_slint(&mut state, win);
    tracing::trace!("model sync checkpoint: thumbnail and handle models synced");

    win.set_layout_label(SharedString::from(i18n::t(
        state.window_collection.current_layout.translation_key(),
    )));
    win.set_window_count(state.window_collection.windows.len() as i32);
    win.set_hidden_count(state.settings.hidden_app_entries().len() as i32);

    tracing::trace!("model sync checkpoint: finished");
}

pub(crate) fn advance_animation(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let state_rc = state.clone();
    let Ok(mut state) = state.try_borrow_mut() else {
        return;
    };
    let Some(started_at) = state.theme.animation_started_at else {
        return;
    };
    if !unsafe { IsWindowVisible(state.shell.hwnd).as_bool() } {
        state.theme.animation_started_at = None;
        return;
    }

    let status = super::animation_engine::tick(
        &mut state.window_collection.windows,
        started_at,
        std::time::Instant::now(),
    );

    if status == super::animation_engine::AnimationStatus::Complete {
        state.theme.animation_started_at = None;
    }

    let window_count = state.window_collection.windows.len();
    let model = win.get_thumbnails();
    if model.row_count() == window_count {
        super::thumbnail_model_builder::update_animation_geometry(
            &state.window_collection.windows,
            win,
        );
    } else {
        drop(state);
        if let Ok(mut state) = state_rc.try_borrow_mut() {
            super::thumbnail_model_builder::sync_model_to_slint(&mut state, win);
        }
    }
}

fn sync_background_image(state: &mut AppState, win: &MainWindow) {
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
                state.settings.background_image_path = None;
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

fn canvas_background_color(settings: &AppSettings) -> slint::Color {
    let (red, green, blue) = rgb_components_from_hex(&settings.background_color_hex);
    slint::Color::from_argb_u8(255, red, green, blue)
}

fn rgb_components_from_hex(hex: &str) -> (u8, u8, u8) {
    let sanitized = hex.trim().trim_start_matches('#');
    let red = u8::from_str_radix(sanitized.get(0..2).unwrap_or("18"), 16).unwrap_or(0x18);
    let green = u8::from_str_radix(sanitized.get(2..4).unwrap_or("15"), 16).unwrap_or(0x15);
    let blue = u8::from_str_radix(sanitized.get(4..6).unwrap_or("13"), 16).unwrap_or(0x13);
    (red, green, blue)
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

    #[test]
    fn rgb_components_from_hex_parses_valid_hex() {
        let (r, g, b) = rgb_components_from_hex("#AABBCC");
        assert_eq!(r, 0xAA);
        assert_eq!(g, 0xBB);
        assert_eq!(b, 0xCC);
    }

    #[test]
    fn rgb_components_from_hex_defaults_on_short_input() {
        let (r, g, b) = rgb_components_from_hex("#12");
        assert_eq!(r, 0x12);
        assert_eq!(g, 0x15);
        assert_eq!(b, 0x13);
    }
}
