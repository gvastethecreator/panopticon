use anyhow::{anyhow, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyIcon, DrawIconEx, GetClassLongPtrW, LoadIconW, SendMessageW,
    DI_NORMAL, GCLP_HICON, GCLP_HICONSM, HICON, ICON_BIG, ICON_SMALL, ICON_SMALL2, IDI_APPLICATION,
    IMAGE_FLAGS, WM_GETICON, WM_SETICON,
};

use crate::app::menu_utils::encode_wide;

/// Application icon handles used by the Win32 window class and the tray icon.
pub struct AppIcons {
    /// Large icon for the main window.
    pub large: HICON,
    /// Small icon for the taskbar / tray.
    pub small: HICON,
    owns_handles: bool,
}

impl AppIcons {
    /// Load application icons from the embedded `assets/icon.ico` resource.
    ///
    /// # Errors
    ///
    /// Returns an error if the ICO data cannot be parsed or a live [`HICON`]
    /// cannot be created.
    pub fn new() -> Result<Self> {
        Ok(Self {
            large: load_icon_from_ico(48)?,
            small: load_icon_from_ico(16)?,
            owns_handles: true,
        })
    }

    /// Create icons with a custom accent colour for differentiated instances.
    ///
    /// # Errors
    ///
    /// Returns an error if the generated icon resource cannot be converted.
    pub fn with_accent(r: u8, g: u8, b: u8) -> Result<Self> {
        Ok(Self {
            large: create_colored_icon(48, [r, g, b])?,
            small: create_colored_icon(16, [r, g, b])?,
            owns_handles: true,
        })
    }

    /// Fallback to the system application icon when custom icon generation
    /// fails.
    #[must_use]
    pub fn fallback_system() -> Self {
        // SAFETY: shared stock icon managed by the OS; must not be destroyed.
        let icon = unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() };
        Self {
            large: icon,
            small: icon,
            owns_handles: false,
        }
    }
}

/// Predefined accent colours for instance differentiation.
pub const INSTANCE_ACCENT_PALETTE: &[[u8; 3]] = &[
    [0xD2, 0x9A, 0x5C], // Amber (default)
    [0x5C, 0xA9, 0xFF], // Sky
    [0x3C, 0xCF, 0x91], // Mint
    [0xFF, 0x6B, 0x8A], // Rose
    [0x9B, 0x7B, 0xFF], // Violet
    [0xF4, 0xB7, 0x40], // Sun
    [0xFF, 0x8C, 0x42], // Tangerine
    [0x42, 0xD4, 0xD4], // Teal
];

impl Drop for AppIcons {
    fn drop(&mut self) {
        if self.owns_handles {
            if !self.large.0.is_null() {
                // SAFETY: owned icon created by `CreateIconFromResourceEx`.
                unsafe {
                    let _ = DestroyIcon(self.large);
                }
            }
            if !self.small.0.is_null() && self.small != self.large {
                // SAFETY: owned icon created by `CreateIconFromResourceEx`.
                unsafe {
                    let _ = DestroyIcon(self.small);
                }
            }
        }
    }
}

/// Apply the Panopticon application icons to a live top-level window.
pub fn apply_window_icons(hwnd: HWND, icons: &AppIcons) {
    if hwnd.0.is_null() {
        return;
    }

    // SAFETY: `hwnd` is a live top-level window handle owned by the UI thread.
    // `WM_SETICON` borrows the provided icon handles; ownership stays in
    // `AppIcons`, which outlives the windows that use them.
    unsafe {
        if !icons.large.0.is_null() {
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_BIG as usize)),
                Some(LPARAM(icons.large.0 as isize)),
            );
        }
        if !icons.small.0.is_null() {
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_SMALL as usize)),
                Some(LPARAM(icons.small.0 as isize)),
            );
        }
    }
}

/// Draw a window icon inside `rect`, centered and scaled.
#[allow(dead_code)]
pub fn draw_window_icon(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    hwnd: HWND,
    rect: RECT,
    size: i32,
) {
    if let Some(icon) = resolve_window_icon_sized(hwnd, size >= 32) {
        let x = rect.left + ((rect.right - rect.left - size) / 2);
        let y = rect.top + ((rect.bottom - rect.top - size) / 2);

        // SAFETY: `hdc` is valid for the current paint pass; `icon` is a live
        // window-owned icon handle borrowed from the source window / class.
        unsafe {
            let _ = DrawIconEx(hdc, x, y, icon, size, size, 0, None, DI_NORMAL);
        }
    }
}

/// Resolve the best available icon for a source window.
#[must_use]
pub fn resolve_window_icon(hwnd: HWND) -> Option<HICON> {
    resolve_window_icon_sized(hwnd, false)
}

/// Resolve the best available icon for a source window, preferring either the
/// large or the small handle depending on the intended render size.
#[must_use]
pub fn resolve_window_icon_sized(hwnd: HWND, prefer_large: bool) -> Option<HICON> {
    // SAFETY: message send / class queries are read-only operations on a live
    // window handle. Returned icons are borrowed; callers must not destroy them.
    unsafe {
        let icon_order = if prefer_large {
            [ICON_BIG, ICON_SMALL2, ICON_SMALL]
        } else {
            [ICON_SMALL2, ICON_SMALL, ICON_BIG]
        };

        for icon_type in icon_order {
            let icon = SendMessageW(
                hwnd,
                WM_GETICON,
                Some(WPARAM(icon_type as usize)),
                Some(LPARAM(0)),
            );
            if icon.0 != 0 {
                return Some(HICON(icon.0 as *mut _));
            }
        }

        let class_order = if prefer_large {
            [GCLP_HICON, GCLP_HICONSM]
        } else {
            [GCLP_HICONSM, GCLP_HICON]
        };

        for class_index in class_order {
            let class_icon = GetClassLongPtrW(hwnd, class_index);
            if class_icon != 0 {
                return Some(HICON(class_icon as *mut _));
            }
        }
    }

    None
}

/// Extract the executable icon from a file path.
///
/// The returned handle is owned by the caller and must be destroyed with
/// [`DestroyIcon`] when no longer needed.
#[must_use]
pub fn resolve_window_icon_from_executable(path: &str, prefer_large: bool) -> Option<HICON> {
    use windows::Win32::UI::Shell::ExtractIconExW;

    let wide = encode_wide(path);
    let mut large = [HICON::default(); 1];
    let mut small = [HICON::default(); 1];

    // SAFETY: `wide` is a valid, nul-terminated UTF-16 path and both icon
    // buffers outlive the call.
    let extracted = unsafe {
        ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(large.as_mut_ptr()),
            Some(small.as_mut_ptr()),
            1,
        )
    };
    if extracted == 0 {
        return None;
    }

    let preferred = if prefer_large { large[0] } else { small[0] };
    let secondary = if prefer_large { small[0] } else { large[0] };

    if !preferred.0.is_null() {
        if !secondary.0.is_null() && secondary != preferred {
            // SAFETY: `secondary` is an extracted icon handle owned by us.
            unsafe {
                let _ = DestroyIcon(secondary);
            }
        }
        Some(preferred)
    } else if !secondary.0.is_null() {
        Some(secondary)
    } else {
        None
    }
}

/// Application icon, statically compiled into the binary.
static APP_ICON_ICO: &[u8] = include_bytes!("../../../assets/icon.ico");

/// Load an [`HICON`] from the embedded `assets/icon.ico`, selecting the entry
/// whose dimensions are closest to `size × size`.
fn load_icon_from_ico(size: u8) -> Result<HICON> {
    let ico = APP_ICON_ICO;
    if ico.len() < 6 {
        return Err(anyhow!("embedded ICO file is too small"));
    }

    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    let mut best_offset: Option<(usize, usize)> = None; // (data_offset, data_len)
    let mut best_diff: u16 = u16::MAX;

    for i in 0..count {
        let e = 6 + i * 16;
        if e + 16 > ico.len() {
            break;
        }
        // ICONDIRENTRY: width is byte 0 (0 encodes 256)
        let img_w = ico[e];
        let actual_w: u16 = if img_w == 0 { 256 } else { u16::from(img_w) };
        let diff = actual_w.abs_diff(u16::from(size));

        let img_bytes =
            u32::from_le_bytes(ico[e + 8..e + 12].try_into().unwrap_or_default()) as usize;
        let img_off =
            u32::from_le_bytes(ico[e + 12..e + 16].try_into().unwrap_or_default()) as usize;

        if diff < best_diff {
            best_diff = diff;
            best_offset = Some((img_off, img_bytes));
        }
    }

    let (img_off, img_bytes) =
        best_offset.ok_or_else(|| anyhow!("no entries found in embedded ICO"))?;
    let image_data = ico
        .get(img_off..img_off + img_bytes)
        .ok_or_else(|| anyhow!("ICO entry data is out of bounds"))?;

    // SAFETY: `image_data` points to a valid ICO image entry (BITMAPINFOHEADER
    // or PNG header) inside a static buffer; `CreateIconFromResourceEx` copies
    // the data so the pointer does not need to remain valid after the call.
    unsafe {
        CreateIconFromResourceEx(
            image_data,
            true,
            0x0003_0000,
            i32::from(size),
            i32::from(size),
            IMAGE_FLAGS(0),
        )
    }
    .map_err(|_| anyhow!("CreateIconFromResourceEx failed for size {size}"))
}

fn create_colored_icon(size: u8, accent_rgb: [u8; 3]) -> Result<HICON> {
    let bytes = build_colored_icon_resource(size, accent_rgb);
    let image_data = &bytes[22..];

    // SAFETY: `bytes` contains a valid in-memory ICO resource with a single
    // 32-bit BGRA image; the buffer outlives the call.
    let icon = unsafe {
        CreateIconFromResourceEx(
            image_data,
            true,
            0x0003_0000,
            i32::from(size),
            i32::from(size),
            IMAGE_FLAGS(0),
        )
    };

    if icon.is_err() {
        Err(anyhow!("failed to create coloured icon handle"))
    } else {
        Ok(icon?)
    }
}

fn build_colored_icon_resource(size: u8, accent_rgb: [u8; 3]) -> Vec<u8> {
    let size_usize = usize::from(size);
    let mask_stride = size_usize.div_ceil(32) * 4;
    let image_size = 40 + (size_usize * size_usize * 4) + (mask_stride * size_usize);
    let image_offset = 6 + 16;

    let mut bytes = Vec::with_capacity(image_offset + image_size);

    // ICONDIR
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());

    // ICONDIRENTRY
    bytes.push(size);
    bytes.push(size);
    bytes.push(0);
    bytes.push(0);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&(image_size as u32).to_le_bytes());
    bytes.extend_from_slice(&(image_offset as u32).to_le_bytes());

    // BITMAPINFOHEADER
    bytes.extend_from_slice(&40u32.to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size)).to_le_bytes());
    bytes.extend_from_slice(&(i32::from(size) * 2).to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&((size_usize * size_usize * 4) as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());

    // Lighten accent for the ring
    let ring_rgb = [
        accent_rgb[0].saturating_add(0x10),
        accent_rgb[1].saturating_add(0x06),
        accent_rgb[2].saturating_add(0x0B),
    ];

    // XOR bitmap (BGRA, bottom-up)
    for y in (0..size_usize).rev() {
        for x in 0..size_usize {
            let pixel = icon_pixel_colored(x as f32, y as f32, size as f32, accent_rgb, ring_rgb);
            bytes.extend_from_slice(&pixel);
        }
    }

    // AND mask
    bytes.resize(image_offset + image_size, 0);

    bytes
}

fn icon_pixel_colored(
    x: f32,
    y: f32,
    size: f32,
    accent_rgb: [u8; 3],
    ring_rgb: [u8; 3],
) -> [u8; 4] {
    let center = (size - 1.0) / 2.0;
    let dx = x - center;
    let dy = y - center;
    let distance = (dx * dx + dy * dy).sqrt();

    let outer = size * 0.47;
    let ring = size * 0.41;
    let eye_x = dx / (size * 0.36);
    let eye_y = dy / (size * 0.22);
    let eye = eye_x * eye_x + eye_y * eye_y;
    let iris = distance <= size * 0.14;
    let pupil = distance <= size * 0.07;
    let highlight = (x - size * 0.62).powi(2) + (y - size * 0.36).powi(2) <= (size * 0.05).powi(2);

    let transparent = [0, 0, 0, 0];
    let dark = [0x19, 0x1A, 0x20, 0xFF];
    let slate = [0x2D, 0x31, 0x3B, 0xFF];
    // BGRA order
    let accent = [accent_rgb[2], accent_rgb[1], accent_rgb[0], 0xFF];
    let accent_ring_color = [ring_rgb[2], ring_rgb[1], ring_rgb[0], 0xFF];
    let near_white = [0xF4, 0xF6, 0xFA, 0xFF];
    let pupil_color = [0x08, 0x0A, 0x0E, 0xFF];

    if distance > outer {
        transparent
    } else if distance >= ring {
        accent_ring_color
    } else if highlight {
        near_white
    } else if pupil {
        pupil_color
    } else if iris {
        accent
    } else if eye <= 1.0 {
        slate
    } else {
        dark
    }
}
