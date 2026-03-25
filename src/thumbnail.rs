//! RAII wrapper around a DWM thumbnail handle (`HTHUMBNAIL`).
//!
//! When a [`Thumbnail`] is dropped, the underlying DWM thumbnail is
//! automatically unregistered via [`DwmUnregisterThumbnail`].  This
//! prevents resource leaks when windows are closed or layouts change.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY,
    DWM_TNP_VISIBLE,
};

/// A registered DWM thumbnail with automatic cleanup on drop.
pub struct Thumbnail {
    handle: isize,
}

impl Thumbnail {
    /// Register a DWM thumbnail linking `source` window into `destination`
    /// window.
    ///
    /// # Errors
    ///
    /// Returns the underlying Windows error if `DwmRegisterThumbnail` fails
    /// (e.g. the source window is invalid or DWM is disabled).
    pub fn register(destination: HWND, source: HWND) -> windows::core::Result<Self> {
        // SAFETY: both HWNDs are valid handles obtained from the window
        // manager; the returned HTHUMBNAIL is owned by this struct.
        let handle = unsafe { DwmRegisterThumbnail(destination, source)? };
        Ok(Self { handle })
    }

    /// Update the thumbnail's destination rectangle and visibility.
    ///
    /// # Errors
    ///
    /// Returns an error if `DwmUpdateThumbnailProperties` fails (e.g. the
    /// source window has been destroyed).
    pub fn update(&self, dest_rect: RECT, visible: bool) -> windows::core::Result<()> {
        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_VISIBLE | DWM_TNP_RECTDESTINATION | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: dest_rect,
            fVisible: visible.into(),
            fSourceClientAreaOnly: true.into(),
            ..Default::default()
        };
        // SAFETY: `self.handle` is a live HTHUMBNAIL obtained from
        // `DwmRegisterThumbnail`; the properties struct is well-formed.
        unsafe {
            DwmUpdateThumbnailProperties(self.handle, &raw const props)?;
        }
        Ok(())
    }

    /// Return the raw `HTHUMBNAIL` value (for queries such as
    /// `DwmQueryThumbnailSourceSize`).
    #[must_use]
    pub fn handle(&self) -> isize {
        self.handle
    }
}

impl Drop for Thumbnail {
    fn drop(&mut self) {
        if self.handle != 0 {
            // SAFETY: dropping a live HTHUMBNAIL; after this call the handle
            // is invalid and must not be used again.
            unsafe {
                let _ = DwmUnregisterThumbnail(self.handle);
            }
        }
    }
}
