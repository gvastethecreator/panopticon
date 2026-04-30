//! Layout animation engine.
//!
//! Owns the animation lifecycle: computes easing curves and updates
//! `ManagedWindow.display_rect` values.  Pure geometry with no Slint
//! or DWM dependencies.

use std::time::Instant;

use panopticon::constants::ANIMATION_DURATION_MS;
use windows::Win32::Foundation::RECT;

use crate::ManagedWindow;

/// Result of a single animation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationStatus {
    /// Animation is still running; display rects were updated.
    Running,
    /// Animation completed this tick; display rects are at their targets.
    Complete,
}

/// Tick the layout animation for a window collection.
///
/// Updates `display_rect` on every `ManagedWindow` based on the elapsed
/// time since `started_at`.  Returns `Complete` when the animation has
/// finished (all rects equal their `target_rect`).
pub(crate) fn tick(
    windows: &mut [ManagedWindow],
    started_at: Instant,
    now: Instant,
) -> AnimationStatus {
    let elapsed_ms = now.duration_since(started_at).as_millis() as u32;
    let progress = (elapsed_ms as f32 / ANIMATION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);

    for window in windows.iter_mut() {
        window.display_rect = lerp_rect(
            window.animation_from_rect,
            window.target_rect,
            eased,
        );
    }

    if progress >= 1.0 {
        for window in windows.iter_mut() {
            window.display_rect = window.target_rect;
        }
        AnimationStatus::Complete
    } else {
        AnimationStatus::Running
    }
}

/// Cancel an in-progress animation and snap all windows to their targets.
pub(crate) fn cancel(windows: &mut [ManagedWindow]) {
    for window in windows.iter_mut() {
        window.display_rect = window.target_rect;
    }
}

fn lerp_rect(from: RECT, to: RECT, t: f32) -> RECT {
    RECT {
        left: lerp_i32(from.left, to.left, t),
        top: lerp_i32(from.top, to.top, t),
        right: lerp_i32(from.right, to.right, t),
        bottom: lerp_i32(from.bottom, to.bottom, t),
    }
}

fn lerp_i32(from: i32, to: i32, t: f32) -> i32 {
    (from as f32 + (to - from) as f32 * t).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use windows::Win32::Foundation::{HWND, RECT, SIZE};
    use crate::ManagedWindow;
    use panopticon::window_enum::WindowInfo;

    fn dummy_window(display_rect: RECT, target_rect: RECT) -> ManagedWindow {
        ManagedWindow {
            info: WindowInfo {
                hwnd: HWND(std::ptr::null_mut::<c_void>()),
                title: String::new(),
                app_id: String::new(),
                process_name: String::new(),
                process_path: None,
                class_name: String::new(),
                monitor_name: String::new(),
            },
            thumbnail: None,
            target_rect,
            display_rect,
            animation_from_rect: display_rect,
            source_size: SIZE { cx: 800, cy: 600 },
            last_thumb_update: None,
            last_thumb_dest: None,
            last_thumb_visible: false,
            cached_icon: None,
        }
    }

    #[test]
    fn tick_completes_when_progress_reaches_one() {
        let target = RECT { left: 10, top: 10, right: 110, bottom: 110 };
        let mut windows = vec![
            dummy_window(RECT { left: 0, top: 0, right: 100, bottom: 100 }, target),
        ];

        let started = Instant::now();
        let ended = started + std::time::Duration::from_millis(ANIMATION_DURATION_MS as u64 + 10);
        let status = tick(&mut windows, started, ended);

        assert_eq!(status, AnimationStatus::Complete);
        assert_eq!(windows[0].display_rect, target);
    }

    #[test]
    fn tick_interpolates_partially() {
        let from = RECT { left: 0, top: 0, right: 100, bottom: 100 };
        let to = RECT { left: 100, top: 0, right: 200, bottom: 100 };
        let mut windows = vec![dummy_window(from, to)];
        windows[0].animation_from_rect = from;

        let started = Instant::now();
        // Simulate a tick at exactly half the animation duration.
        let mid = started + std::time::Duration::from_millis(ANIMATION_DURATION_MS as u64 / 2);
        let status = tick(&mut windows, started, mid);

        assert_eq!(status, AnimationStatus::Running);
        // Eased progress at 0.5 is not exactly 0.5, but the display_rect
        // should be strictly between from and to.
        assert!(windows[0].display_rect.left > from.left);
        assert!(windows[0].display_rect.left < to.left);
    }

    #[test]
    fn cancel_snaps_to_targets() {
        let target = RECT { left: 50, top: 50, right: 150, bottom: 150 };
        let mut windows = vec![
            dummy_window(RECT { left: 0, top: 0, right: 100, bottom: 100 }, target),
        ];

        cancel(&mut windows);

        assert_eq!(windows[0].display_rect, target);
    }
}
