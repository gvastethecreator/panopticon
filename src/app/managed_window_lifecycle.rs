//! Managed-window preview lifecycle helpers.
//!
//! Keeps constructor/reset logic for tracked windows in one place so
//! reconciliation, icon caching, and DWM code share the same defaults.

use panopticon::window_enum::WindowInfo;
use windows::Win32::Foundation::{RECT, SIZE};

use crate::{ManagedWindow, ManagedWindowPreview};

impl ManagedWindowPreview {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            thumbnail: None,
            source_size: SIZE { cx: 800, cy: 600 },
            last_thumb_update: None,
            last_thumb_dest: None,
            last_thumb_visible: false,
            cached_icon: None,
        }
    }

    pub(crate) fn release_thumbnail(&mut self) {
        self.thumbnail = None;
        self.last_thumb_update = None;
        self.last_thumb_dest = None;
        self.last_thumb_visible = false;
    }

    pub(crate) fn invalidate_cached_icon(&mut self) {
        self.cached_icon = None;
    }
}

impl ManagedWindow {
    #[must_use]
    pub(crate) fn new(info: WindowInfo) -> Self {
        Self {
            info,
            target_rect: RECT::default(),
            display_rect: RECT::default(),
            animation_from_rect: RECT::default(),
            preview: ManagedWindowPreview::new(),
        }
    }

    pub(crate) fn release_thumbnail_preview(&mut self) {
        self.preview.release_thumbnail();
    }

    pub(crate) fn invalidate_cached_icon(&mut self) {
        self.preview.invalidate_cached_icon();
    }
}
