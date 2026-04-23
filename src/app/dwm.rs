//! DWM thumbnail registration, update, and geometry helpers.

use std::ffi::c_void;
use std::mem;
use std::time::Instant;

use slint::ComponentHandle;
use windows::Win32::Foundation::{HWND, RECT, SIZE};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DwmQueryThumbnailSourceSize, DWMWA_CLOAKED,
};
use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindow, IsWindowVisible};

use panopticon::constants::THUMBNAIL_ACCENT_HEIGHT;
use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::thumbnail::Thumbnail;

use crate::{
    ManagedWindow, HIDDEN_THUMBNAIL_RECT, THUMBNAIL_CONTENT_PADDING, THUMBNAIL_INFO_STRIP_HEIGHT,
};

// ───────────────────────── Thumbnail lifecycle ─────────────────────────

/// Register a DWM thumbnail for a managed window if one does not already exist.
/// Returns `true` if a new thumbnail was registered.
pub(crate) fn ensure_thumbnail(owner: HWND, mw: &mut ManagedWindow) -> bool {
    if mw.thumbnail.is_some() {
        return false;
    }
    if let Ok(thumb) = Thumbnail::register(owner, mw.info.hwnd) {
        mw.source_size = query_source_size(thumb.handle());
        mw.thumbnail = Some(thumb);
        true
    } else {
        false
    }
}

pub(crate) fn release_thumbnail(mw: &mut ManagedWindow) {
    mw.thumbnail = None;
    mw.last_thumb_update = None;
    mw.last_thumb_dest = None;
    mw.last_thumb_visible = false;
}

pub(crate) fn release_all_thumbnails(state: &std::rc::Rc<std::cell::RefCell<crate::AppState>>) {
    if let Ok(mut s) = state.try_borrow_mut() {
        for mw in &mut s.windows {
            release_thumbnail(mw);
        }
    }
}

pub(crate) fn query_source_size(handle: isize) -> SIZE {
    // SAFETY: handle is a valid DWM thumbnail handle obtained via registration.
    let mut size = unsafe { DwmQueryThumbnailSourceSize(handle).unwrap_or_default() };
    if size.cx == 0 {
        size.cx = 800;
    }
    if size.cy == 0 {
        size.cy = 600;
    }
    size
}

// ───────────────────────── DWM thumbnail sync ─────────────────────────

/// Synchronise all DWM thumbnail positions, visibility, and refresh timing.
pub(crate) fn update_dwm_thumbnails(
    state: &std::rc::Rc<std::cell::RefCell<crate::AppState>>,
    win: &crate::MainWindow,
) {
    let Ok(mut s) = state.try_borrow_mut() else {
        return;
    };
    // SAFETY: checking visibility is a read-only Win32 query on our live window.
    if s.hwnd.0.is_null() || !unsafe { IsWindowVisible(s.hwnd).as_bool() } {
        return;
    }

    let scale = win.window().scale_factor();
    let phys = win.window().size();
    let viewport_x = win.get_viewport_x();
    let viewport_y = win.get_viewport_y();
    let now = Instant::now();

    let dest_hwnd = s.hwnd;
    let (settings, windows) = {
        let state = &mut *s;
        (&state.settings, &mut state.windows)
    };
    let toolbar_phys_h = if settings.show_toolbar {
        (TOOLBAR_HEIGHT as f32 * scale).round() as i32
    } else {
        0
    };
    let content_phys_h = (phys.height as i32 - toolbar_phys_h).max(1);
    let show_icons = settings.show_app_icons;
    let show_info = settings.show_window_info;

    for mw in windows.iter_mut() {
        let preserve = settings.preserve_aspect_ratio_for(&mw.info.app_id);
        let refresh_mode = settings.thumbnail_refresh_mode_for(&mw.info.app_id);
        let interval_ms = settings.thumbnail_refresh_interval_ms_for(&mw.info.app_id);
        let render_scale_pct = settings.thumbnail_render_scale_pct;
        // SAFETY: Win32 queries on window handles discovered through enumeration.
        let is_minimized = unsafe { IsIconic(mw.info.hwnd).as_bool() };
        let is_source_valid = unsafe { IsWindow(Some(mw.info.hwnd)).as_bool() };
        let is_source_visible = unsafe { IsWindowVisible(mw.info.hwnd).as_bool() };
        let is_cloaked = is_window_cloaked(mw.info.hwnd);
        if !is_source_valid || (!is_source_visible && !is_minimized) || is_cloaked {
            release_thumbnail(mw);
            continue;
        }
        if show_icons {
            crate::app::icon::populate_cached_icon(mw);
        }
        let overlay_top_h = if show_info || is_minimized || (show_icons && mw.cached_icon.is_some())
        {
            THUMBNAIL_INFO_STRIP_HEIGHT
        } else {
            0
        };

        let should_refresh_bitmap = !is_minimized
            && match refresh_mode {
                panopticon::settings::ThumbnailRefreshMode::Frozen => mw.thumbnail.is_none(),
                panopticon::settings::ThumbnailRefreshMode::Interval => mw
                    .last_thumb_update
                    .is_none_or(|t| now.duration_since(t).as_millis() >= u128::from(interval_ms)),
                panopticon::settings::ThumbnailRefreshMode::Realtime => true,
            };

        let registered_thumbnail = ensure_thumbnail(dest_hwnd, mw);
        if let Some(thumb) = mw.thumbnail.as_ref() {
            let raw_dest = compute_dwm_rect(
                &mw.display_rect,
                mw.source_size,
                preserve,
                overlay_top_h,
                viewport_x,
                viewport_y,
                scale,
                render_scale_pct,
            );
            let (dest, visible) =
                sanitize_thumbnail_rect(raw_dest, phys.width as i32, content_phys_h);
            let props_changed = registered_thumbnail
                || mw.last_thumb_dest != Some(dest)
                || mw.last_thumb_visible != visible;
            let should_push_update = props_changed || should_refresh_bitmap;

            if should_push_update {
                if let Err(error) = thumb.update(dest, visible) {
                    tracing::warn!(
                        %error,
                        title = %mw.info.title,
                        visible,
                        minimized = is_minimized,
                        cloaked = is_cloaked,
                        dest = ?dest,
                        "thumbnail update failed — dropping"
                    );
                    release_thumbnail(mw);
                } else {
                    mw.last_thumb_dest = Some(dest);
                    mw.last_thumb_visible = visible;
                    if should_refresh_bitmap {
                        mw.last_thumb_update = Some(now);
                    }
                }
            }
        }
    }
}

// ───────────────────────── Geometry helpers ─────────────────────────

pub(crate) fn sanitize_thumbnail_rect(
    dest: RECT,
    client_width: i32,
    client_height: i32,
) -> (RECT, bool) {
    if client_width <= 0 || client_height <= 0 || dest.right <= dest.left || dest.bottom <= dest.top
    {
        return (HIDDEN_THUMBNAIL_RECT, false);
    }

    if dest.right <= 0 || dest.bottom <= 0 || dest.left >= client_width || dest.top >= client_height
    {
        return (HIDDEN_THUMBNAIL_RECT, false);
    }

    let clipped = RECT {
        left: dest.left.clamp(0, client_width.saturating_sub(1)),
        top: dest.top.clamp(0, client_height.saturating_sub(1)),
        right: dest.right.clamp(1, client_width),
        bottom: dest.bottom.clamp(1, client_height),
    };

    if clipped.right <= clipped.left || clipped.bottom <= clipped.top {
        (HIDDEN_THUMBNAIL_RECT, false)
    } else {
        (clipped, true)
    }
}

pub(crate) fn is_window_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    // SAFETY: querying a DWM attribute on a live top-level HWND is read-only.
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            std::ptr::from_mut(&mut cloaked).cast::<c_void>(),
            mem::size_of_val(&cloaked) as u32,
        )
        .is_ok_and(|()| cloaked != 0)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::many_single_char_names)]
pub(crate) fn compute_dwm_rect(
    card_rect: &RECT,
    source_size: SIZE,
    preserve_aspect: bool,
    overlay_top_h: i32,
    viewport_x: f32,
    viewport_y: f32,
    scale: f32,
    render_scale_pct: u8,
) -> RECT {
    let inset = THUMBNAIL_CONTENT_PADDING as f32;
    let l = card_rect.left as f32 + inset;
    let t = card_rect.top as f32 + THUMBNAIL_ACCENT_HEIGHT as f32 + overlay_top_h as f32 + inset;
    let r = card_rect.right as f32 - inset;
    let b = card_rect.bottom as f32 - inset;

    let (fl, ft, fr, fb) = if preserve_aspect && source_size.cx > 0 && source_size.cy > 0 {
        let aw = r - l;
        let ah = b - t;
        let wr = aw / source_size.cx as f32;
        let hr = ah / source_size.cy as f32;
        let s = wr.min(hr);
        let rw = source_size.cx as f32 * s;
        let rh = source_size.cy as f32 * s;
        (
            l + (aw - rw) / 2.0,
            t + (ah - rh) / 2.0,
            l + (aw - rw) / 2.0 + rw,
            t + (ah - rh) / 2.0 + rh,
        )
    } else {
        (l, t, r, b)
    };

    let render_scale = (f32::from(render_scale_pct) / 100.0).clamp(0.25, 1.0);
    // DWM thumbnails do not expose a separate internal render-resolution knob,
    // so we approximate lower scales by nudging the destination rect inward
    // while keeping the visual footprint close to the card size. That makes the
    // preview look softer / a bit more pixelated without shrinking it as much
    // as the previous implementation did.
    let visual_scale = (0.88 + render_scale * 0.12).clamp(0.91, 1.0);
    let scaled_width = (fr - fl) * visual_scale;
    let scaled_height = (fb - ft) * visual_scale;
    let scaled_left = fl + ((fr - fl) - scaled_width) / 2.0;
    let scaled_top = ft + ((fb - ft) - scaled_height) / 2.0;
    let scaled_right = scaled_left + scaled_width;
    let scaled_bottom = scaled_top + scaled_height;

    RECT {
        left: ((scaled_left + viewport_x) * scale).round() as i32,
        top: ((scaled_top + viewport_y) * scale).round() as i32,
        right: ((scaled_right + viewport_x) * scale).round() as i32,
        bottom: ((scaled_bottom + viewport_y) * scale).round() as i32,
    }
}
