//! Window icon extraction and HICON-to-Slint rendering helpers.

use std::ffi::c_void;
use std::mem;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL, HICON};

use panopticon::window_enum::WindowInfo;

use crate::app::tray::{
    resolve_window_icon, resolve_window_icon_from_executable, resolve_window_icon_sized,
};
use crate::ManagedWindow;

// ───────────────────────── Cached icon population ─────────────────────────

/// Extract and cache the application icon for a managed window.
pub(crate) fn populate_cached_icon(mw: &mut ManagedWindow) {
    if mw.cached_icon.is_some() {
        return;
    }
    mw.cached_icon = hicon_to_slint_image(&mw.info);
}

// ───────────────────────── HICON → Slint Image ─────────────────────────

/// Convert a window's HICON to a high-resolution Slint RGBA image.
fn hicon_to_slint_image(info: &WindowInfo) -> Option<slint::Image> {
    let (icon, owns_icon) = resolve_preview_icon(info)?;
    let image = render_hicon_to_slint_image(icon);
    if owns_icon {
        // SAFETY: fallback icons extracted from executables are owned by this
        // function and must be destroyed after rendering.
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }
    image
}

fn resolve_preview_icon(info: &WindowInfo) -> Option<(HICON, bool)> {
    info.process_path
        .as_deref()
        .and_then(|path| resolve_window_icon_from_executable(path, true))
        .map(|icon| (icon, true))
        .or_else(|| resolve_window_icon_sized(info.hwnd, true).map(|icon| (icon, false)))
        .or_else(|| {
            info.process_path
                .as_deref()
                .and_then(|path| resolve_window_icon_from_executable(path, false))
                .map(|icon| (icon, true))
        })
        .or_else(|| resolve_window_icon(info.hwnd).map(|icon| (icon, false)))
}

fn render_hicon_to_slint_image(icon: HICON) -> Option<slint::Image> {
    let size: i32 = 256;
    // SAFETY: GDI drawing operations on a temporary memory DC; all resources
    // are released before returning.
    unsafe {
        let screen_dc = GetDC(HWND::default());
        if screen_dc.0.is_null() {
            return None;
        }
        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.0.is_null() {
            let _ = ReleaseDC(HWND::default(), screen_dc);
            return None;
        }

        let mut bmi: BITMAPINFO = mem::zeroed();
        bmi.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = size;
        bmi.bmiHeader.biHeight = -size; // top-down DIB
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB.0;

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let Ok(dib) = CreateDIBSection(
            mem_dc,
            &raw const bmi,
            DIB_RGB_COLORS,
            &raw mut bits_ptr,
            None,
            0,
        ) else {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(HWND::default(), screen_dc);
            return None;
        };
        if bits_ptr.is_null() {
            let _ = DeleteObject(HGDIOBJ(dib.0.cast()));
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(HWND::default(), screen_dc);
            return None;
        }

        let old = SelectObject(mem_dc, HGDIOBJ(dib.0.cast()));
        let _ = DrawIconEx(mem_dc, 0, 0, icon, size, size, 0, None, DI_NORMAL);
        SelectObject(mem_dc, old);

        let pixel_count = (size * size) as usize;
        let src = std::slice::from_raw_parts(bits_ptr.cast::<u8>(), pixel_count * 4);
        let mut rgba = vec![0u8; pixel_count * 4];

        let mut has_alpha = false;
        for (i, chunk) in src.chunks_exact(4).enumerate() {
            let o = i * 4;
            rgba[o] = chunk[2];
            rgba[o + 1] = chunk[1];
            rgba[o + 2] = chunk[0];
            rgba[o + 3] = chunk[3];
            if chunk[3] != 0 {
                has_alpha = true;
            }
        }

        // Icons without an alpha channel: set all non-black pixels to opaque.
        if !has_alpha {
            for chunk in rgba.chunks_exact_mut(4) {
                if chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0 {
                    chunk[3] = 255;
                }
            }
        }

        let rgba = normalize_icon_canvas(&rgba, size as usize, 4);

        let _ = DeleteObject(HGDIOBJ(dib.0.cast()));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(HWND::default(), screen_dc);

        let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            &rgba,
            u32::try_from(size).unwrap_or(32),
            u32::try_from(size).unwrap_or(32),
        );
        Some(slint::Image::from_rgba8(buffer))
    }
}

// ───────────────────────── Normalize + bilinear ─────────────────────────

fn normalize_icon_canvas(source: &[u8], size: usize, padding: usize) -> Vec<u8> {
    let mut min_x = size;
    let mut min_y = size;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;

    for y in 0..size {
        for x in 0..size {
            let alpha = source[(y * size + x) * 4 + 3];
            if alpha > 8 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    if !found {
        return source.to_vec();
    }

    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;
    let target_side = size.saturating_sub(padding * 2).max(1);
    let scale = target_side as f32 / crop_w.max(crop_h) as f32;
    let dest_w = ((crop_w as f32 * scale).round() as usize).max(1);
    let dest_h = ((crop_h as f32 * scale).round() as usize).max(1);
    let offset_x = (size.saturating_sub(dest_w)) / 2;
    let offset_y = (size.saturating_sub(dest_h)) / 2;
    let mut normalized = vec![0u8; source.len()];

    for dy in 0..dest_h {
        for dx in 0..dest_w {
            let sx = min_x as f32 + ((dx as f32 + 0.5) / scale) - 0.5;
            let sy = min_y as f32 + ((dy as f32 + 0.5) / scale) - 0.5;
            let sample = bilinear_sample_rgba(source, size, sx, sy);
            let dst_index = ((offset_y + dy) * size + (offset_x + dx)) * 4;
            normalized[dst_index..dst_index + 4].copy_from_slice(&sample);
        }
    }

    normalized
}

pub(crate) fn bilinear_sample_rgba(source: &[u8], size: usize, x: f32, y: f32) -> [u8; 4] {
    let max = (size.saturating_sub(1)) as f32;
    let x = x.clamp(0.0, max);
    let y = y.clamp(0.0, max);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(size.saturating_sub(1));
    let y1 = (y0 + 1).min(size.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let sample = |sx: usize, sy: usize| {
        let index = (sy * size + sx) * 4;
        &source[index..index + 4]
    };

    let weights = [
        ((1.0 - tx) * (1.0 - ty), sample(x0, y0)),
        (tx * (1.0 - ty), sample(x1, y0)),
        ((1.0 - tx) * ty, sample(x0, y1)),
        (tx * ty, sample(x1, y1)),
    ];

    let mut accum_r = 0.0;
    let mut accum_g = 0.0;
    let mut accum_b = 0.0;
    let mut accum_a = 0.0;

    for (weight, pixel) in weights {
        let alpha = f32::from(pixel[3]) / 255.0;
        let weighted_alpha = weight * alpha;
        accum_r += weight * f32::from(pixel[0]) * alpha;
        accum_g += weight * f32::from(pixel[1]) * alpha;
        accum_b += weight * f32::from(pixel[2]) * alpha;
        accum_a += weighted_alpha;
    }

    if accum_a <= f32::EPSILON {
        return [0, 0, 0, 0];
    }

    [
        (accum_r / accum_a).round().clamp(0.0, 255.0) as u8,
        (accum_g / accum_a).round().clamp(0.0, 255.0) as u8,
        (accum_b / accum_a).round().clamp(0.0, 255.0) as u8,
        (accum_a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}
