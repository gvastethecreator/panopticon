//! Pure layout computation pipeline.
//!
//! Bridges the already-pure [`compute_layout_custom`] engine with the
//! runtime's [`ManagedWindow`] collection.  Everything in this module is
//! deterministic and testable without Slint or DWM.

use panopticon::layout::{compute_layout_custom, AspectHint, LayoutCustomization, LayoutType, Separator};
use windows::Win32::Foundation::RECT;

use crate::ManagedWindow;

/// Compute layout rectangles and separators for a window collection.
///
/// This is a thin, pure wrapper around [`compute_layout_custom`] that
/// extracts aspect hints from the current window set.
pub(crate) fn compute_layout_rects(
    layout: LayoutType,
    content_area: RECT,
    windows: &[ManagedWindow],
    custom: Option<&LayoutCustomization>,
) -> (Vec<RECT>, Vec<Separator>) {
    let aspects: Vec<AspectHint> = windows
        .iter()
        .map(|w| AspectHint {
            width: f64::from(w.source_size.cx),
            height: f64::from(w.source_size.cy),
        })
        .collect();
    let result = compute_layout_custom(layout, content_area, windows.len(), &aspects, custom);
    (result.rects, result.separators)
}

/// Apply computed layout rectangles to the window collection.
///
/// Updates each [`ManagedWindow`]'s `target_rect`, `animation_from_rect`,
/// and `display_rect`.  Returns `true` when at least one window moved
/// far enough to warrant an animated transition.
pub(crate) fn apply_layout_rects(
    windows: &mut [ManagedWindow],
    rects: &[RECT],
    can_animate: bool,
) -> bool {
    let mut animation_needed = false;

    for (index, window) in windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(index) {
            let prev = if rect_has_area(window.display_rect) {
                window.display_rect
            } else {
                rect
            };
            window.animation_from_rect = prev;
            window.target_rect = rect;
            if can_animate && prev != rect {
                animation_needed = true;
            } else {
                window.display_rect = rect;
            }
        }
    }

    if !animation_needed {
        for window in windows.iter_mut() {
            window.display_rect = window.target_rect;
        }
    }

    animation_needed
}

pub(crate) fn rect_has_area(rect: RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use windows::Win32::Foundation::{HWND, RECT, SIZE};
    use crate::ManagedWindow;
    use panopticon::window_enum::WindowInfo;

    fn dummy_window(display_rect: RECT) -> ManagedWindow {
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
            target_rect: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            display_rect,
            animation_from_rect: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            source_size: SIZE { cx: 800, cy: 600 },
            last_thumb_update: None,
            last_thumb_dest: None,
            last_thumb_visible: false,
            cached_icon: None,
        }
    }

    #[test]
    fn apply_layout_rects_updates_target_and_display_when_no_animation() {
        let mut windows = vec![
            dummy_window(RECT { left: 0, top: 0, right: 100, bottom: 100 }),
        ];
        let rects = vec![RECT { left: 10, top: 10, right: 110, bottom: 110 }];

        let needed = apply_layout_rects(&mut windows, &rects, false);

        assert!(!needed);
        assert_eq!(windows[0].target_rect, rects[0]);
        assert_eq!(windows[0].display_rect, rects[0]);
    }

    #[test]
    fn apply_layout_rects_flags_animation_when_rect_changes_and_allowed() {
        let mut windows = vec![
            dummy_window(RECT { left: 0, top: 0, right: 100, bottom: 100 }),
        ];
        let rects = vec![RECT { left: 10, top: 10, right: 110, bottom: 110 }];

        let needed = apply_layout_rects(&mut windows, &rects, true);

        assert!(needed);
        assert_eq!(windows[0].target_rect, rects[0]);
        assert_eq!(windows[0].display_rect, RECT { left: 0, top: 0, right: 100, bottom: 100 });
    }

    #[test]
    fn apply_layout_rects_syncs_display_to_target_when_animation_not_needed() {
        let mut windows = vec![
            dummy_window(RECT { left: 5, top: 5, right: 105, bottom: 105 }),
        ];
        let rects = vec![RECT { left: 5, top: 5, right: 105, bottom: 105 }];

        let needed = apply_layout_rects(&mut windows, &rects, true);

        assert!(!needed);
        assert_eq!(windows[0].display_rect, rects[0]);
    }
}
