//! Layout recomputation, Slint model synchronization, and thumbnail animation.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use panopticon::constants::{ANIMATION_DURATION_MS, TOOLBAR_HEIGHT};
use panopticon::i18n;
use panopticon::layout::{compute_layout_custom, AspectHint, ScrollDirection};
use panopticon::settings::{AppSettings, ToolbarPosition};
use panopticon::window_ops::{active_filter_summary, truncate_title};
use slint::ComponentHandle;
use slint::{Model, ModelRc, SharedString, VecModel};
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};

use super::icon::populate_cached_icon;
use super::settings_ui::background_fit_to_index;
use super::theme_ui::{sync_theme_target, thumbnail_accent_color};
use crate::{AppState, MainWindow, ResizeHandleData, ThumbnailData};

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

    tracing::info!("recompute checkpoint: entered");

    let mut state = app_state.borrow_mut();
    if state.windows.is_empty() {
        state.animation_started_at = None;
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

    let aspects: Vec<AspectHint> = state
        .windows
        .iter()
        .map(|managed_window| AspectHint {
            width: f64::from(managed_window.source_size.cx),
            height: f64::from(managed_window.source_size.cy),
        })
        .collect();

    let custom = state.settings.layout_custom(state.current_layout).cloned();
    let result = compute_layout_custom(
        state.current_layout,
        content_area,
        state.windows.len(),
        &aspects,
        custom.as_ref(),
    );
    tracing::info!(
        window_count = state.windows.len(),
        "recompute checkpoint: layout computed"
    );
    let rects = result.rects;
    state.separators = result.separators;

    let scroll_dir = state.current_layout.scroll_direction();
    state.content_extent = match scroll_dir {
        ScrollDirection::Horizontal => rects.iter().map(|rect| rect.right).max().unwrap_or(0),
        ScrollDirection::Vertical => rects.iter().map(|rect| rect.bottom).max().unwrap_or(0),
        ScrollDirection::None => 0,
    };

    let can_animate = state.settings.animate_transitions
        && !state.hwnd.0.is_null()
        && unsafe { IsWindowVisible(state.hwnd).as_bool() }
        && state.drag_separator.is_none()
        && state
            .windows
            .iter()
            .any(|managed_window| rect_has_area(managed_window.display_rect));
    let mut animation_needed = false;

    for (index, managed_window) in state.windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(index) {
            let prev = if rect_has_area(managed_window.display_rect) {
                managed_window.display_rect
            } else {
                rect
            };
            managed_window.animation_from_rect = prev;
            managed_window.target_rect = rect;
            if can_animate && prev != rect {
                animation_needed = true;
            } else {
                managed_window.display_rect = rect;
            }
        }
    }

    if animation_needed {
        state.animation_started_at = Some(Instant::now());
    } else {
        state.animation_started_at = None;
        for managed_window in &mut state.windows {
            managed_window.display_rect = managed_window.target_rect;
        }
    }

    let scroll_h = scroll_dir == ScrollDirection::Horizontal;
    let scroll_v = scroll_dir == ScrollDirection::Vertical;
    win.set_scroll_horizontal(scroll_h);
    win.set_scroll_vertical(scroll_v);
    win.set_content_width(state.content_extent as f32);
    win.set_content_height(state.content_extent as f32);
    tracing::info!("recompute checkpoint: scroll properties applied");
    clamp_viewport_offsets(
        win,
        scroll_dir,
        state.content_extent,
        logical_w,
        content_area.bottom,
    );
    tracing::info!("recompute checkpoint: viewport clamped");

    sync_theme_target(&mut state);
    tracing::info!("recompute checkpoint: theme synced");
    sync_settings_to_ui(win, &state.settings);
    tracing::info!("recompute checkpoint: settings synced");
    sync_background_image(&mut state, win);
    tracing::info!("recompute checkpoint: background synced");

    drop(state);
    tracing::info!("recompute reached pre-model-sync checkpoint");
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
    win.set_refresh_label(SharedString::from(settings.refresh_status_label()));
    win.set_filters_label(SharedString::from(
        active_filter_summary(settings).unwrap_or_default(),
    ));
}

pub(crate) fn sync_model_to_slint(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Some(_guard) = ModelSyncGuard::enter() else {
        tracing::debug!("skipping nested sync_model_to_slint invocation");
        return;
    };

    tracing::info!("model sync checkpoint: entered");

    let mut state = state.borrow_mut();
    let show_footer = state.settings.show_window_info;
    let show_icons = state.settings.show_app_icons;
    let resize_locked = state.settings.locked_layout || state.settings.lock_cell_resize;

    if show_icons {
        for managed_window in &mut state.windows {
            populate_cached_icon(managed_window);
        }
    } else {
        for managed_window in &mut state.windows {
            managed_window.cached_icon = None;
        }
    }
    tracing::info!(
        show_icons,
        window_count = state.windows.len(),
        "model sync checkpoint: icons ready"
    );

    let data: Vec<ThumbnailData> = state
        .windows
        .iter()
        .map(|managed_window| build_thumbnail_data(managed_window, &state, show_footer, show_icons))
        .collect();
    tracing::info!(
        thumbnail_rows = data.len(),
        "model sync checkpoint: thumbnail data built"
    );

    let handle_thickness: f32 = 14.0;
    let handles: Vec<ResizeHandleData> = if resize_locked {
        Vec::new()
    } else {
        state
            .separators
            .iter()
            .enumerate()
            .map(|(idx, separator)| {
                if separator.horizontal {
                    ResizeHandleData {
                        x: separator.extent_start as f32,
                        y: separator.position as f32 - handle_thickness / 2.0,
                        width: (separator.extent_end - separator.extent_start) as f32,
                        height: handle_thickness,
                        horizontal: true,
                        index: idx as i32,
                    }
                } else {
                    ResizeHandleData {
                        x: separator.position as f32 - handle_thickness / 2.0,
                        y: separator.extent_start as f32,
                        width: handle_thickness,
                        height: (separator.extent_end - separator.extent_start) as f32,
                        horizontal: false,
                        index: idx as i32,
                    }
                }
            })
            .collect()
    };

    let dragging = state.drag_separator.is_some();
    let active_drag = state
        .drag_separator
        .as_ref()
        .map_or(-1, |drag| drag.separator_index as i32);
    tracing::info!(
        handle_rows = handles.len(),
        dragging,
        "model sync checkpoint: resize handles built"
    );

    win.set_layout_label(SharedString::from(i18n::t(
        state.current_layout.translation_key(),
    )));
    win.set_window_count(state.windows.len() as i32);
    win.set_hidden_count(state.settings.hidden_app_entries().len() as i32);

    drop(state);
    tracing::info!("model sync checkpoint: window labels applied");
    win.set_thumbnails(ModelRc::new(VecModel::from(data)));
    tracing::info!("model sync checkpoint: thumbnails model set");

    if dragging {
        let model = win.get_resize_handles();
        let existing = model.row_count();
        for (idx, handle_data) in handles.into_iter().enumerate() {
            if idx < existing {
                model.set_row_data(idx, handle_data);
            }
        }
    } else {
        win.set_resize_handles(ModelRc::new(VecModel::from(handles)));
    }
    tracing::info!("model sync checkpoint: resize handles set");
    win.set_active_drag_index(active_drag);
    tracing::info!("model sync checkpoint: finished");
}

pub(crate) fn advance_animation(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Ok(mut state) = state.try_borrow_mut() else {
        return;
    };
    let Some(started_at) = state.animation_started_at else {
        return;
    };
    if !unsafe { IsWindowVisible(state.hwnd).as_bool() } {
        state.animation_started_at = None;
        return;
    }

    let elapsed_ms = started_at.elapsed().as_millis() as u32;
    let progress = (elapsed_ms as f32 / ANIMATION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);

    for managed_window in &mut state.windows {
        managed_window.display_rect = lerp_rect(
            managed_window.animation_from_rect,
            managed_window.target_rect,
            eased,
        );
    }

    if progress >= 1.0 {
        state.animation_started_at = None;
        for managed_window in &mut state.windows {
            managed_window.display_rect = managed_window.target_rect;
        }
    }

    let show_footer = state.settings.show_window_info;
    let show_icons = state.settings.show_app_icons;
    let model = win.get_thumbnails();
    let row_count = model.row_count();
    let window_count = state.windows.len();

    if row_count == window_count {
        // Fast path: update only geometry fields that change during animation.
        for (index, managed_window) in state.windows.iter().enumerate() {
            if let Some(mut item) = model.row_data(index) {
                let new_x = managed_window.display_rect.left as f32;
                let new_y = managed_window.display_rect.top as f32;
                let new_w =
                    (managed_window.display_rect.right - managed_window.display_rect.left) as f32;
                let new_h =
                    (managed_window.display_rect.bottom - managed_window.display_rect.top) as f32;
                // These are integer-derived pixel coordinates cast to f32,
                // so exact bit equality is correct here.
                #[allow(clippy::float_cmp)]
                if item.x != new_x || item.y != new_y || item.width != new_w || item.height != new_h
                {
                    item.x = new_x;
                    item.y = new_y;
                    item.width = new_w;
                    item.height = new_h;
                    model.set_row_data(index, item);
                }
            }
        }
    } else {
        let data: Vec<ThumbnailData> = state
            .windows
            .iter()
            .map(|managed_window| {
                build_thumbnail_data(managed_window, &state, show_footer, show_icons)
            })
            .collect();
        drop(state);
        win.set_thumbnails(ModelRc::new(VecModel::from(data)));
    }
}

fn sync_background_image(state: &mut AppState, win: &MainWindow) {
    let desired = state.settings.background_image_path.clone();
    if state.loaded_background_path == desired {
        return;
    }

    if let Some(path) = desired.as_deref() {
        match slint::Image::load_from_path(Path::new(path)) {
            Ok(image) => {
                win.set_background_image(image);
                state.loaded_background_path = desired;
            }
            Err(error) => {
                tracing::warn!(%error, path, "failed to load background image");
                win.set_background_image(slint::Image::default());
                state.settings.background_image_path = None;
                state.loaded_background_path = None;

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
        state.loaded_background_path = None;
    }
}

fn clamp_viewport_offsets(
    win: &MainWindow,
    scroll_dir: ScrollDirection,
    content_extent: i32,
    visible_width: i32,
    visible_height: i32,
) {
    match scroll_dir {
        ScrollDirection::Horizontal => {
            let max_scroll = (content_extent - visible_width).max(0) as f32;
            win.set_viewport_x(win.get_viewport_x().clamp(-max_scroll, 0.0));
            win.set_viewport_y(0.0);
        }
        ScrollDirection::Vertical => {
            let max_scroll = (content_extent - visible_height).max(0) as f32;
            win.set_viewport_y(win.get_viewport_y().clamp(-max_scroll, 0.0));
            win.set_viewport_x(0.0);
        }
        ScrollDirection::None => {
            win.set_viewport_x(0.0);
            win.set_viewport_y(0.0);
        }
    }
}

fn build_thumbnail_data(
    managed_window: &crate::ManagedWindow,
    state: &AppState,
    show_footer: bool,
    show_icons: bool,
) -> ThumbnailData {
    let accent = thumbnail_accent_color(
        &state.settings,
        &state.current_theme,
        &managed_window.info.app_id,
    );
    let is_minimized = unsafe { IsIconic(managed_window.info.hwnd).as_bool() };
    ThumbnailData {
        x: managed_window.display_rect.left as f32,
        y: managed_window.display_rect.top as f32,
        width: (managed_window.display_rect.right - managed_window.display_rect.left) as f32,
        height: (managed_window.display_rect.bottom - managed_window.display_rect.top) as f32,
        title: SharedString::from(truncate_title(&managed_window.info.title)),
        app_label: SharedString::from(managed_window.info.app_label()),
        is_active: state.active_hwnd == Some(managed_window.info.hwnd),
        accent_color: accent,
        show_footer,
        is_minimized,
        icon: managed_window.cached_icon.clone().unwrap_or_default(),
        show_icon: show_icons,
    }
}

fn rect_has_area(rect: RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn lerp_rect(from: RECT, to: RECT, t: f32) -> RECT {
    RECT {
        left: lerp_i32(from.left, to.left, t),
        top: lerp_i32(from.top, to.top, t),
        right: lerp_i32(from.right, to.right, t),
        bottom: lerp_i32(from.bottom, to.bottom, t),
    }
}

fn lerp_i32(from: i32, to: i32, t: f32) -> i32 {
    (from as f32 + (to - from) as f32 * t).round() as i32
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
    fn rect_has_area_rejects_degenerate_rects() {
        assert!(!rect_has_area(RECT {
            left: 10,
            top: 10,
            right: 10,
            bottom: 20,
        }));
        assert!(!rect_has_area(RECT {
            left: 10,
            top: 10,
            right: 20,
            bottom: 10,
        }));
        assert!(rect_has_area(RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        }));
    }

    #[test]
    fn lerp_i32_interpolates_and_rounds() {
        assert_eq!(lerp_i32(0, 10, 0.0), 0);
        assert_eq!(lerp_i32(0, 10, 0.5), 5);
        assert_eq!(lerp_i32(0, 10, 1.0), 10);
        assert_eq!(lerp_i32(10, 0, 0.25), 8);
    }

    #[test]
    fn lerp_rect_interpolates_each_edge() {
        let from = RECT {
            left: 0,
            top: 10,
            right: 100,
            bottom: 110,
        };
        let to = RECT {
            left: 20,
            top: 30,
            right: 140,
            bottom: 170,
        };

        let mid = lerp_rect(from, to, 0.5);

        assert_eq!(mid.left, 10);
        assert_eq!(mid.top, 20);
        assert_eq!(mid.right, 120);
        assert_eq!(mid.bottom, 140);
    }
}
