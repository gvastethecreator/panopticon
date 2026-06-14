//! Layout recomputation, Slint model synchronization, and thumbnail animation.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::layout::{LayoutCustomization, LayoutType, ScrollDirection};
use slint::ComponentHandle;
use slint::Model;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;

use crate::app::window_collection::WindowCollection;
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

/// Compute the layout for the current window collection and update the
/// scroll direction / content extent accordingly.
fn compute_layout_and_scroll(
    window_collection: &mut WindowCollection,
    layout: LayoutType,
    custom: Option<&LayoutCustomization>,
    docked: bool,
    content_area: RECT,
) -> (Vec<RECT>, ScrollDirection) {
    let (rects, separators, cols, rows) = super::layout_pipeline::compute_layout_rects(
        layout,
        content_area,
        &window_collection.windows,
        docked,
        custom,
    );
    tracing::trace!(
        window_count = window_collection.windows.len(),
        "recompute checkpoint: layout computed"
    );
    window_collection.separators = separators;
    window_collection.docked_wrap_dims = if docked { Some((cols, rows)) } else { None };

    let scroll_dir = layout.scroll_direction_for(docked);
    window_collection.content_extent = match scroll_dir {
        ScrollDirection::Horizontal => rects.iter().map(|rect| rect.right).max().unwrap_or(0),
        ScrollDirection::Vertical => rects.iter().map(|rect| rect.bottom).max().unwrap_or(0),
        ScrollDirection::None => 0,
    };

    (rects, scroll_dir)
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
        super::presentation::sync_theme_target(&mut state);
        super::presentation::sync_settings_to_ui(win, &state.settings);
        super::presentation::sync_background_image(&mut state, win);
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

    let layout = state.window_collection.current_layout;
    let custom = state.settings.layout_custom(layout).cloned();
    let docked = state.settings.dock_edge.is_some();
    let (rects, scroll_dir) = compute_layout_and_scroll(
        &mut state.window_collection,
        layout,
        custom.as_ref(),
        docked,
        content_area,
    );

    let can_animate = state.settings.animate_transitions
        && !state.shell.hwnd.0.is_null()
        && unsafe {
            // SAFETY: read-only visibility query for the application's own top-level window.
            IsWindowVisible(state.shell.hwnd).as_bool()
        }
        && state.window_collection.drag_separator.is_none()
        && state
            .window_collection
            .windows
            .iter()
            .any(|managed_window| {
                super::layout_pipeline::rect_has_area(managed_window.display_rect)
            });

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

    let (clamped_x, clamped_y) = super::viewport_manager::clamp_offsets(
        scroll_dir,
        state.window_collection.content_extent,
        logical_w,
        content_area.bottom,
        win.get_viewport_x(),
        win.get_viewport_y(),
    );
    let scroll_presentation = super::presentation::ScrollPresentation::new(
        scroll_dir,
        state.window_collection.content_extent,
        clamped_x,
        clamped_y,
    );
    super::presentation::apply_scroll_presentation(win, &scroll_presentation);
    tracing::trace!("recompute checkpoint: scroll properties applied");
    tracing::trace!("recompute checkpoint: viewport clamped");

    super::presentation::sync_theme_target(&mut state);
    tracing::trace!("recompute checkpoint: theme synced");
    super::presentation::sync_settings_to_ui(win, &state.settings);
    tracing::trace!("recompute checkpoint: settings synced");
    super::presentation::sync_background_image(&mut state, win);
    tracing::trace!("recompute checkpoint: background synced");

    drop(state);
    tracing::trace!("recompute reached pre-model-sync checkpoint");
    sync_model_to_slint(app_state, win);
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

    super::presentation::sync_main_window_metadata(win, &state);

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
    if !unsafe {
        // SAFETY: read-only visibility query for the application's own top-level window.
        IsWindowVisible(state.shell.hwnd).as_bool()
    } {
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
