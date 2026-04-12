//! Integration tests for Win32 input decoding helpers.

use panopticon::input_ops::{decode_mouse_lparam, scroll_pixels_from_wheel_delta};

#[test]
fn scroll_pixels_from_wheel_delta_matches_windows_tick_scale() {
    assert_eq!(scroll_pixels_from_wheel_delta(120), 48.0);
    assert_eq!(scroll_pixels_from_wheel_delta(-120), -48.0);
    assert_eq!(scroll_pixels_from_wheel_delta(60), 24.0);
}

#[test]
fn decode_mouse_lparam_preserves_signed_coordinates() {
    let positive = ((45_i32 as u32) | ((90_i32 as u32) << 16)) as isize;
    let negative = ((-12_i16 as u16 as u32) | ((-34_i16 as u16 as u32) << 16)) as isize;

    assert_eq!(decode_mouse_lparam(positive), (45, 90));
    assert_eq!(decode_mouse_lparam(negative), (-12, -34));
}
