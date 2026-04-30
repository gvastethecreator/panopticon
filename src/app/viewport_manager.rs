//! Viewport offset clamping.
//!
//! Pure logic that determines the valid scroll range for a given layout
//! and clamps viewport offsets so content stays visible.

use panopticon::layout::ScrollDirection;

/// Clamp viewport offsets for the given scroll direction and content extent.
///
/// Returns `(viewport_x, viewport_y)` after clamping.
pub(crate) fn clamp_offsets(
    scroll_dir: ScrollDirection,
    content_extent: i32,
    visible_width: i32,
    visible_height: i32,
    current_x: f32,
    current_y: f32,
) -> (f32, f32) {
    match scroll_dir {
        ScrollDirection::Horizontal => {
            let max_scroll = (content_extent - visible_width).max(0) as f32;
            (current_x.clamp(-max_scroll, 0.0), 0.0)
        }
        ScrollDirection::Vertical => {
            let max_scroll = (content_extent - visible_height).max(0) as f32;
            (0.0, current_y.clamp(-max_scroll, 0.0))
        }
        ScrollDirection::None => (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_clamps_to_negative_range() {
        let (x, y) = clamp_offsets(ScrollDirection::Horizontal, 500, 200, 100, -50.0, 10.0);
        assert_eq!(x, -50.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn horizontal_clamps_beyond_max() {
        let (x, y) = clamp_offsets(ScrollDirection::Horizontal, 500, 200, 100, -400.0, 0.0);
        assert_eq!(x, -300.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn vertical_clamps_beyond_max() {
        let (x, y) = clamp_offsets(ScrollDirection::Vertical, 800, 400, 200, 0.0, -700.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, -600.0);
    }

    #[test]
    fn none_always_zero() {
        let (x, y) = clamp_offsets(ScrollDirection::None, 1000, 100, 100, -50.0, -50.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }
}
