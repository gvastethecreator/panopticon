//! Pure helpers for decoding Win32 input payloads.

/// Logical pixels applied per wheel tick when panning the viewport.
pub const WHEEL_SCROLL_PIXELS_PER_TICK: f32 = 48.0;

/// Convert a Win32 wheel delta into logical scroll pixels.
#[must_use]
pub fn scroll_pixels_from_wheel_delta(delta: i16) -> f32 {
    f32::from(delta) / 120.0 * WHEEL_SCROLL_PIXELS_PER_TICK
}

/// Decode signed x/y coordinates packed into a Win32 `LPARAM`.
#[must_use]
pub fn decode_mouse_lparam(lparam: isize) -> (i32, i32) {
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    (x, y)
}
