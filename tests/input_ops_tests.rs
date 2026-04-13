//! Integration tests for Win32 input decoding helpers.

use panopticon::input_ops::{decode_mouse_lparam, scroll_pixels_from_wheel_delta};

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

fn mouse_lparam(x: i16, y: i16) -> isize {
    let low_word = i32::from(u16::from_ne_bytes(x.to_ne_bytes()));
    let packed = low_word | (i32::from(y) << 16);
    isize::try_from(packed).expect("packed LPARAM must fit in isize on Windows")
}

#[test]
fn scroll_pixels_from_wheel_delta_matches_windows_tick_scale() {
    assert_close(scroll_pixels_from_wheel_delta(120), 48.0);
    assert_close(scroll_pixels_from_wheel_delta(-120), -48.0);
    assert_close(scroll_pixels_from_wheel_delta(60), 24.0);
}

#[test]
fn decode_mouse_lparam_preserves_signed_coordinates() {
    let positive = mouse_lparam(45, 90);
    let negative = mouse_lparam(-12, -34);

    assert_eq!(decode_mouse_lparam(positive), (45, 90));
    assert_eq!(decode_mouse_lparam(negative), (-12, -34));
}
