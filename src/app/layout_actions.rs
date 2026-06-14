//! Layout switching, resize dragging, and per-layout ratio customization.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::layout::{
    apply_separator_drag, apply_separator_drag_grouped, default_ratios, LayoutType, ScrollDirection,
};
use slint::ComponentHandle;

use super::model_sync::{recompute_and_update_ui, sync_model_to_slint};
use super::runtime_support::refresh_ui;
use crate::{AppState, MainWindow};

pub(crate) fn cycle_layout(state: &Rc<RefCell<AppState>>) {
    let mut state = state.borrow_mut();
    if state.settings.locked_layout {
        return;
    }
    state.window_collection.current_layout = state.window_collection.current_layout.next();
    let next_layout = state.window_collection.current_layout;
    state.window_collection.drag_separator = None;
    let _ = state.settings.update_persisted(|settings| {
        settings.initial_layout = next_layout;
    });
    let _ = state.settings.save(state.workspace_name.as_deref());
}

pub(crate) fn set_layout(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    layout: LayoutType,
) {
    {
        let mut state_ref = state.borrow_mut();
        if state_ref.settings.locked_layout {
            return;
        }
        if state_ref.window_collection.current_layout == layout {
            return;
        }
        state_ref.window_collection.current_layout = layout;
        let _ = state_ref.settings.update_persisted(|settings| {
            settings.initial_layout = layout;
        });
        state_ref.window_collection.drag_separator = None;
        let _ = state_ref.settings.save(state_ref.workspace_name.as_deref());
    }
    refresh_ui(state, weak);
}

pub(crate) fn reset_layout_custom(state: &Rc<RefCell<AppState>>) {
    let mut state = state.borrow_mut();
    let layout = state.window_collection.current_layout;
    let _ = state.settings.update_persisted(|settings| {
        settings.clear_layout_custom(layout);
    });
    let _ = state.settings.save(state.workspace_name.as_deref());
}

pub(crate) fn handle_resize_drag_start(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    separator_index: usize,
    x: f64,
    y: f64,
) {
    let mut state = state.borrow_mut();
    if state.settings.lock_cell_resize || state.settings.locked_layout {
        state.window_collection.drag_separator = None;
        return;
    }
    let Some(separator) = state
        .window_collection
        .separators
        .get(separator_index)
        .copied()
    else {
        return;
    };

    let phys = weak.upgrade().map(|window| window.window().size());
    let scale = weak
        .upgrade()
        .map_or(1.0, |window| window.window().scale_factor());
    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };
    let logical_w = phys.map_or(1280, |size| (size.width as f32 / scale).round() as i32);
    let logical_h =
        phys.map_or(720, |size| (size.height as f32 / scale).round() as i32) - toolbar_h;

    let docked = state.settings.dock_edge.is_some();
    let scroll_dir = state
        .window_collection
        .current_layout
        .scroll_direction_for(docked);
    let axis_extent = if separator.horizontal {
        match scroll_dir {
            ScrollDirection::Vertical => {
                f64::from(state.window_collection.content_extent.max(logical_h))
            }
            _ => f64::from(logical_h),
        }
    } else {
        match scroll_dir {
            ScrollDirection::Horizontal => {
                f64::from(state.window_collection.content_extent.max(logical_w))
            }
            _ => f64::from(logical_w),
        }
    };

    let initial_offset = if separator.horizontal { y } else { x };

    state.window_collection.drag_separator = Some(crate::DragState {
        separator_index,
        horizontal: separator.horizontal,
        ratio_index: separator.ratio_index,
        axis_extent,
        last_pointer_offset: initial_offset,
    });
}

pub(crate) fn handle_resize_drag_move(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    separator_index: usize,
    x: f64,
    y: f64,
) {
    let (horizontal, ratio_index, axis_extent, last_pointer_offset) = {
        let state = state.borrow();
        if state.settings.lock_cell_resize || state.settings.locked_layout {
            return;
        }
        let Some(drag) = state.window_collection.drag_separator.as_ref() else {
            return;
        };
        if drag.separator_index != separator_index || drag.axis_extent <= 0.0 {
            return;
        }
        (
            drag.horizontal,
            drag.ratio_index,
            drag.axis_extent,
            drag.last_pointer_offset,
        )
    };

    let pointer_offset = if horizontal { y } else { x };
    let delta_frac = (pointer_offset - last_pointer_offset) / axis_extent;

    let mut state_ref = state.borrow_mut();
    let layout = state_ref.window_collection.current_layout;
    let window_count = state_ref.window_collection.windows.len();
    let docked = state_ref.settings.dock_edge.is_some();
    let wrap_dims = state_ref.window_collection.docked_wrap_dims;

    let min_frac = 0.03;
    let _ = state_ref.settings.update_persisted(|settings| {
        ensure_custom_ratios(settings, layout, window_count, docked, wrap_dims);

        if let Some(custom) = settings.layout_customizations.get_mut(layout.storage_key()) {
            let ratios = if horizontal {
                &mut custom.row_ratios
            } else {
                &mut custom.col_ratios
            };
            if ratio_index + 1 < ratios.len() {
                match layout {
                    // Docked wrap modes use per-adjacent resizing like Grid.
                    LayoutType::Row | LayoutType::Column if docked => {
                        apply_separator_drag(ratios, ratio_index, delta_frac, min_frac);
                    }
                    LayoutType::Columns | LayoutType::Row | LayoutType::Column => {
                        apply_separator_drag_grouped(ratios, ratio_index, delta_frac, min_frac);
                    }
                    _ => apply_separator_drag(ratios, ratio_index, delta_frac, min_frac),
                }
            }
        }
    });

    if let Some(drag) = state_ref.window_collection.drag_separator.as_mut() {
        if drag.separator_index == separator_index {
            drag.last_pointer_offset = pointer_offset;
        }
    }
    drop(state_ref);

    if let Some(window) = weak.upgrade() {
        recompute_and_update_ui(state, &window);
    }
}

pub(crate) fn handle_resize_drag_end(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
) {
    let mut state_ref = state.borrow_mut();
    state_ref.window_collection.drag_separator = None;
    let _ = state_ref.settings.save(state_ref.workspace_name.as_deref());
    drop(state_ref);

    if let Some(window) = weak.upgrade() {
        sync_model_to_slint(state, &window);
    }
}

fn ensure_custom_ratios(
    settings: &mut panopticon::settings::AppSettings,
    layout: LayoutType,
    count: usize,
    docked: bool,
    wrap_dims: Option<(usize, usize)>,
) {
    if count == 0 {
        return;
    }

    let entry = settings
        .layout_customizations
        .entry(layout.storage_key().to_owned())
        .or_default();

    match layout {
        LayoutType::Grid => {
            let cols = (count as f64).sqrt().ceil() as usize;
            let rows = count.div_ceil(cols);
            if entry.col_ratios.len() != cols {
                entry.col_ratios = default_ratios(cols);
            }
            if entry.row_ratios.len() != rows {
                entry.row_ratios = default_ratios(rows);
            }
        }
        LayoutType::Mosaic => {
            let cols = (count as f64).sqrt().ceil() as usize;
            let rows_count = count.div_ceil(cols);
            if entry.row_ratios.len() != rows_count {
                entry.row_ratios = default_ratios(rows_count);
            }
        }
        LayoutType::Bento => {
            if entry.col_ratios.is_empty() {
                entry.col_ratios = vec![0.6];
            }
            let side_count = count.saturating_sub(1);
            if side_count > 0 && entry.row_ratios.len() != side_count {
                entry.row_ratios = default_ratios(side_count);
            }
        }
        LayoutType::Columns => {
            let num_cols = ((count as f64).sqrt().ceil() as usize).clamp(2, 5);
            if entry.col_ratios.len() != num_cols {
                entry.col_ratios = default_ratios(num_cols);
            }
        }
        LayoutType::Row if docked => {
            // Docked Row wraps into rows; only row heights are resizable.
            let rows = wrap_dims.map_or(1, |(_, r)| r);
            if entry.row_ratios.len() != rows {
                entry.row_ratios = default_ratios(rows);
            }
        }
        LayoutType::Row => {
            if entry.col_ratios.len() != count {
                entry.col_ratios = default_ratios(count);
            }
        }
        LayoutType::Column if docked => {
            // Docked Column wraps into columns; only column widths are resizable.
            let cols = wrap_dims.map_or(1, |(c, _)| c);
            if entry.col_ratios.len() != cols {
                entry.col_ratios = default_ratios(cols);
            }
        }
        LayoutType::Column => {
            if entry.row_ratios.len() != count {
                entry.row_ratios = default_ratios(count);
            }
        }
        LayoutType::Fibonacci => {}
    }
}
