//! Integration tests for the Panopticon layout engine.

use std::collections::HashSet;

use panopticon::layout::{compute_layout, AspectHint, LayoutType};
use windows::Win32::Foundation::RECT;

// ── Helpers ──────────────────────────────────────────────────

fn area(w: i32, h: i32) -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    }
}

fn uniform_aspects(count: usize) -> Vec<AspectHint> {
    vec![
        AspectHint {
            width: 1920.0,
            height: 1080.0,
        };
        count
    ]
}

const ALL_LAYOUTS: [LayoutType; 7] = [
    LayoutType::Grid,
    LayoutType::Mosaic,
    LayoutType::Bento,
    LayoutType::Fibonacci,
    LayoutType::Columns,
    LayoutType::Row,
    LayoutType::Column,
];

/// Layouts where all rects are guaranteed to stay within the provided area.
const BOUNDED_LAYOUTS: [LayoutType; 5] = [
    LayoutType::Grid,
    LayoutType::Mosaic,
    LayoutType::Bento,
    LayoutType::Fibonacci,
    LayoutType::Columns,
];

// ── Zero windows ─────────────────────────────────────────────

#[test]
fn all_layouts_return_empty_for_zero_windows() {
    for layout in ALL_LAYOUTS {
        let rects = compute_layout(layout, area(1280, 720), 0, &[]);
        assert!(
            rects.is_empty(),
            "{layout:?} should return empty vec for 0 windows"
        );
    }
}

// ── Single window ────────────────────────────────────────────

#[test]
fn all_layouts_return_one_rect_for_single_window() {
    let aspects = uniform_aspects(1);
    for layout in ALL_LAYOUTS {
        let rects = compute_layout(layout, area(1280, 720), 1, &aspects);
        assert_eq!(rects.len(), 1, "{layout:?} should return exactly 1 rect");
    }
}

// ── Correct count ────────────────────────────────────────────

#[test]
fn all_layouts_return_correct_count() {
    for count in [2, 3, 5, 8, 13, 20] {
        let aspects = uniform_aspects(count);
        for layout in ALL_LAYOUTS {
            let rects = compute_layout(layout, area(1280, 720), count, &aspects);
            assert_eq!(
                rects.len(),
                count,
                "{layout:?} with {count} windows should return {count} rects"
            );
        }
    }
}

// ── Rects within bounds ──────────────────────────────────────

#[test]
fn rects_stay_within_area() {
    let a = area(1920, 1080);
    let aspects = uniform_aspects(9);
    for layout in BOUNDED_LAYOUTS {
        let rects = compute_layout(layout, a, 9, &aspects);
        for (i, r) in rects.iter().enumerate() {
            assert!(
                r.left >= a.left && r.top >= a.top && r.right <= a.right && r.bottom <= a.bottom,
                "{layout:?} rect {i} ({r:?}) is out of bounds ({a:?})"
            );
        }
    }
}

// ── Non-degenerate rects ─────────────────────────────────────

#[test]
fn rects_have_positive_dimensions() {
    let aspects = uniform_aspects(6);
    for layout in ALL_LAYOUTS {
        let rects = compute_layout(layout, area(1280, 720), 6, &aspects);
        for (i, r) in rects.iter().enumerate() {
            assert!(
                r.right > r.left && r.bottom > r.top,
                "{layout:?} rect {i} has non-positive dimensions: {r:?}"
            );
        }
    }
}

// ── Layout cycling ───────────────────────────────────────────

#[test]
fn layout_next_cycles_through_all_variants() {
    let mut layout = LayoutType::Grid;
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(layout);
        layout = layout.next();
    }
    // After 7 steps we should be back to Grid.
    assert_eq!(seen[0], seen[7]);
    // All intermediate variants should be distinct.
    let distinct: HashSet<_> = seen[..7].iter().collect();
    assert_eq!(distinct.len(), 7);
}

// ── Aspect-ratio edge case ───────────────────────────────────

#[test]
fn aspect_ratio_handles_zero_height() {
    let hint = AspectHint {
        width: 1920.0,
        height: 0.0,
    };
    assert!((hint.ratio() - 1.0).abs() < f64::EPSILON);
}

// ── Layout labels ────────────────────────────────────────────

#[test]
fn layout_labels_are_non_empty() {
    for layout in ALL_LAYOUTS {
        assert!(!layout.label().is_empty(), "{layout:?} has empty label");
    }
}

// ── Small area stress test ───────────────────────────────────

#[test]
fn layouts_handle_very_small_area() {
    let tiny = area(20, 20);
    let aspects = uniform_aspects(4);
    for layout in ALL_LAYOUTS {
        // Should not panic.
        let rects = compute_layout(layout, tiny, 4, &aspects);
        assert_eq!(rects.len(), 4, "{layout:?} should still return 4 rects");
    }
}

// ── Row / Column layout tests ────────────────────────────────

#[test]
fn row_layout_extends_beyond_area_when_content_overflows() {
    let a = area(800, 600);
    let aspects = uniform_aspects(10);
    let rects = compute_layout(LayoutType::Row, a, 10, &aspects);
    assert_eq!(rects.len(), 10);
    // With 10 landscape windows the total width should exceed 800.
    let max_right = rects.iter().map(|r| r.right).max().unwrap();
    assert!(max_right > a.right, "Row should overflow horizontally");
}

#[test]
fn column_layout_extends_beyond_area_when_content_overflows() {
    let a = area(800, 600);
    let aspects = uniform_aspects(10);
    let rects = compute_layout(LayoutType::Column, a, 10, &aspects);
    assert_eq!(rects.len(), 10);
    // With 10 wide windows, total height should exceed 600.
    let max_bottom = rects.iter().map(|r| r.bottom).max().unwrap();
    assert!(max_bottom > a.bottom, "Column should overflow vertically");
}

#[test]
fn row_layout_fits_single_window() {
    let a = area(1280, 720);
    let aspects = uniform_aspects(1);
    let rects = compute_layout(LayoutType::Row, a, 1, &aspects);
    assert_eq!(rects.len(), 1);
    // Single window should fill the area (accounting for padding).
    assert!(rects[0].right <= a.right);
}

#[test]
fn column_layout_fits_single_window() {
    let a = area(1280, 720);
    let aspects = uniform_aspects(1);
    let rects = compute_layout(LayoutType::Column, a, 1, &aspects);
    assert_eq!(rects.len(), 1);
    assert!(rects[0].bottom <= a.bottom);
}

// ── Mixed aspect ratios ──────────────────────────────────────

#[test]
fn mosaic_handles_mixed_aspects() {
    let aspects = vec![
        AspectHint {
            width: 1920.0,
            height: 1080.0,
        },
        AspectHint {
            width: 1080.0,
            height: 1920.0,
        },
        AspectHint {
            width: 800.0,
            height: 600.0,
        },
        AspectHint {
            width: 600.0,
            height: 800.0,
        },
    ];
    let rects = compute_layout(LayoutType::Mosaic, area(1280, 720), 4, &aspects);
    assert_eq!(rects.len(), 4);
    for r in &rects {
        assert!(r.right > r.left && r.bottom > r.top);
    }
}
