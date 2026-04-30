//! Build Slint thumbnail and resize-handle models from runtime state.
//!
//! This module owns the mapping from `ManagedWindow`s and `Separator`s to
//! the `ThumbnailData` and `ResizeHandleData` structs consumed by Slint.

use panopticon::layout::Separator;
use panopticon::window_ops::truncate_title;
use slint::{Model, ModelRc, SharedString, VecModel};
use windows::Win32::UI::WindowsAndMessaging::IsIconic;

use crate::{AppState, MainWindow, ManagedWindow, ResizeHandleData, ThumbnailData};
use super::icon::populate_cached_icon;
use super::theme_ui::thumbnail_accent_color;

const HANDLE_THICKNESS: f32 = 14.0;

/// Build the complete thumbnail model from current state.
pub(crate) fn build_thumbnails(
    state: &mut AppState,
    _win: &MainWindow,
) -> Vec<ThumbnailData> {
    let show_footer = state.settings.show_window_info;
    let show_icons = state.settings.show_app_icons;

    if show_icons {
        for managed_window in &mut state.window_collection.windows {
            populate_cached_icon(managed_window);
        }
    } else {
        for managed_window in &mut state.window_collection.windows {
            managed_window.cached_icon = None;
        }
    }

    state
        .window_collection
        .windows
        .iter()
        .map(|managed_window| build_thumbnail_data(managed_window, state, show_footer, show_icons))
        .collect()
}

/// Build a single `ThumbnailData` from a `ManagedWindow`.
pub(crate) fn build_thumbnail_data(
    managed_window: &ManagedWindow,
    state: &AppState,
    show_footer: bool,
    show_icons: bool,
) -> ThumbnailData {
    let accent = thumbnail_accent_color(
        &state.settings,
        &state.theme.current_theme,
        &managed_window.info.app_id,
    );
    let is_minimized = unsafe { IsIconic(managed_window.info.hwnd).as_bool() };
    let pinned_slot = state
        .settings
        .pinned_position_for(&managed_window.info.app_id)
        .and_then(|slot| i32::try_from(slot).ok())
        .unwrap_or(-1);
    ThumbnailData {
        x: managed_window.display_rect.left as f32,
        y: managed_window.display_rect.top as f32,
        width: (managed_window.display_rect.right - managed_window.display_rect.left) as f32,
        height: (managed_window.display_rect.bottom - managed_window.display_rect.top) as f32,
        title: SharedString::from(truncate_title(&managed_window.info.title)),
        app_label: SharedString::from(managed_window.info.app_label()),
        is_active: state.window_collection.active_hwnd == Some(managed_window.info.hwnd),
        accent_color: accent,
        show_footer,
        is_minimized,
        icon: managed_window.cached_icon.clone().unwrap_or_default(),
        show_icon: show_icons,
        pinned_slot,
    }
}

/// Build resize handles from the current separator set.
pub(crate) fn build_resize_handles(
    separators: &[Separator],
    _drag_separator: Option<&crate::DragState>,
    resize_locked: bool,
) -> Vec<ResizeHandleData> {
    if resize_locked {
        return Vec::new();
    }

    separators
        .iter()
        .enumerate()
        .map(|(idx, separator)| {
            if separator.horizontal {
                ResizeHandleData {
                    x: separator.extent_start as f32,
                    y: separator.position as f32 - HANDLE_THICKNESS / 2.0,
                    width: (separator.extent_end - separator.extent_start) as f32,
                    height: HANDLE_THICKNESS,
                    horizontal: true,
                    index: idx as i32,
                }
            } else {
                ResizeHandleData {
                    x: separator.position as f32 - HANDLE_THICKNESS / 2.0,
                    y: separator.extent_start as f32,
                    width: HANDLE_THICKNESS,
                    height: (separator.extent_end - separator.extent_start) as f32,
                    horizontal: false,
                    index: idx as i32,
                }
            }
        })
        .collect()
}

/// Synchronise the Slint model with the current thumbnail and handle data.
///
/// Compares row counts and uses the fast path (row update) when possible,
/// falling back to full model replacement when the count changed.
pub(crate) fn sync_model_to_slint(
    state: &mut AppState,
    win: &MainWindow,
) {
    let thumbnails = build_thumbnails(state, win);
    let resize_locked = state.settings.locked_layout || state.settings.lock_cell_resize;
    let handles = build_resize_handles(
        &state.window_collection.separators,
        state.window_collection.drag_separator.as_ref(),
        resize_locked,
    );
    let active_drag = state
        .window_collection
        .drag_separator
        .as_ref()
        .map_or(-1, |drag| drag.separator_index as i32);

    // Sync thumbnails model.
    let thumbnails_model = win.get_thumbnails();
    if thumbnails_model.row_count() == thumbnails.len() {
        for (index, item) in thumbnails.into_iter().enumerate() {
            thumbnails_model.set_row_data(index, item);
        }
    } else {
        win.set_thumbnails(ModelRc::new(VecModel::from(thumbnails)));
    }

    // Sync resize handles model.
    let handles_model = win.get_resize_handles();
    if handles_model.row_count() == handles.len() {
        for (index, handle_data) in handles.into_iter().enumerate() {
            handles_model.set_row_data(index, handle_data);
        }
    } else {
        win.set_resize_handles(ModelRc::new(VecModel::from(handles)));
    }

    win.set_active_drag_index(active_drag);
}

/// Fast-path animation model update: only geometry fields that change.
///
/// Assumes the model row count already matches the window count.
pub(crate) fn update_animation_geometry(
    windows: &[ManagedWindow],
    win: &MainWindow,
) {
    let model = win.get_thumbnails();
    for (index, managed_window) in windows.iter().enumerate() {
        if let Some(mut item) = model.row_data(index) {
            let new_x = managed_window.display_rect.left as f32;
            let new_y = managed_window.display_rect.top as f32;
            let new_w =
                (managed_window.display_rect.right - managed_window.display_rect.left) as f32;
            let new_h =
                (managed_window.display_rect.bottom - managed_window.display_rect.top) as f32;
            #[allow(clippy::float_cmp)]
            if item.x != new_x || item.y != new_y || item.width != new_w || item.height != new_h {
                item.x = new_x;
                item.y = new_y;
                item.width = new_w;
                item.height = new_h;
                model.set_row_data(index, item);
            }
        }
    }
}
