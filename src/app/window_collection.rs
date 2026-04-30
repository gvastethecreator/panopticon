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
        }
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.windows.len()
    }

    #[inline]
    pub(crate) fn reset_drag(&mut self) {
        self.drag_separator = None;
    }

    #[inline]
    pub(crate) fn set_active(&mut self, hwnd: HWND) {
        self.active_hwnd = Some(hwnd);
    }
}
