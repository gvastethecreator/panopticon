//! Window collection and layout state.
//!
//! Groups the fields that describe the set of tracked windows, their
//! current layout, and any in-progress user interactions (separator drag,
//! active selection).

use panopticon::layout::{LayoutType, Separator};
use windows::Win32::Foundation::HWND;

use crate::{DragState, ManagedWindow};

/// The subset of [`AppState`] that deals with the window collection.
pub(crate) struct WindowCollection {
    pub(crate) windows: Vec<ManagedWindow>,
    pub(crate) current_layout: LayoutType,
    pub(crate) separators: Vec<Separator>,
    pub(crate) drag_separator: Option<DragState>,
    pub(crate) content_extent: i32,
    pub(crate) active_hwnd: Option<HWND>,
    /// Grid dimensions used when a docked `Row`/`Column` layout wraps its
    /// content.  `None` for all other layout modes.
    pub(crate) docked_wrap_dims: Option<(usize, usize)>,
}

impl WindowCollection {
    pub(crate) fn new(initial_layout: LayoutType) -> Self {
        Self {
            windows: Vec::new(),
            current_layout: initial_layout,
            separators: Vec::new(),
            drag_separator: None,
            content_extent: 0,
            active_hwnd: None,
            docked_wrap_dims: None,
        }
    }
}
