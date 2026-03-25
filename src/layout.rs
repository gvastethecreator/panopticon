//! Layout engine for Panopticon.
//!
//! Provides five mathematical layout algorithms that arrange `n` window
//! thumbnails inside a given rectangular area.  All functions are **pure**
//! (no side effects, no I/O) and therefore fully unit-testable.

use windows::Win32::Foundation::RECT;

/// The five layout modes specified in the PRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutType {
    /// Equal-sized cells in a √n × √n grid.
    Grid,
    /// Rows with aspect-ratio-weighted column widths.
    Mosaic,
    /// Primary window (60 % width) plus a sidebar stack.
    Bento,
    /// Golden-ratio spiral subdivision.
    Fibonacci,
    /// Masonry-style shortest-column-first placement.
    Columns,
}

impl LayoutType {
    /// Return the next layout in the cycle.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Grid => Self::Mosaic,
            Self::Mosaic => Self::Bento,
            Self::Bento => Self::Fibonacci,
            Self::Fibonacci => Self::Columns,
            Self::Columns => Self::Grid,
        }
    }

    /// Human-readable name for the toolbar label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Mosaic => "Mosaic",
            Self::Bento => "Bento",
            Self::Fibonacci => "Fibonacci",
            Self::Columns => "Columns",
        }
    }
}

/// Aspect-ratio hint for a single window (`width / height`).
#[derive(Debug, Clone, Copy)]
pub struct AspectHint {
    /// Horizontal extent (pixels or arbitrary units).
    pub width: f64,
    /// Vertical extent (pixels or arbitrary units).
    pub height: f64,
}

impl AspectHint {
    /// Compute the aspect ratio, falling back to `1.0` if `height` is zero.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        if self.height == 0.0 {
            1.0
        } else {
            self.width / self.height
        }
    }
}

/// Padding (in pixels) applied inside each cell.
const PADDING: i32 = 6;

/// Compute destination [`RECT`]s for `count` windows inside `area`.
///
/// Each layout mode distributes the rectangles differently; see
/// [`LayoutType`] for descriptions.
#[must_use]
pub fn compute_layout(
    layout: LayoutType,
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
) -> Vec<RECT> {
    if count == 0 {
        return Vec::new();
    }
    match layout {
        LayoutType::Grid => grid_layout(area, count),
        LayoutType::Mosaic => mosaic_layout(area, count, aspects),
        LayoutType::Bento => bento_layout(area, count),
        LayoutType::Fibonacci => fibonacci_layout(area, count),
        LayoutType::Columns => columns_layout(area, count, aspects),
    }
}

// ───────────────────────── Grid ─────────────────────────

/// Equal-sized cells in a √n × √n grid.
fn grid_layout(area: RECT, count: usize) -> Vec<RECT> {
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(cols);

    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);
    let cell_w = total_w / cols as f64;
    let cell_h = total_h / rows as f64;

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let col = i % cols;
        let row = i / cols;
        rects.push(padded_rect(
            area.left + (col as f64 * cell_w) as i32,
            area.top + (row as f64 * cell_h) as i32,
            area.left + ((col + 1) as f64 * cell_w) as i32,
            area.top + ((row + 1) as f64 * cell_h) as i32,
        ));
    }
    rects
}

// ───────────────────────── Mosaic ─────────────────────────

/// Rows of windows where each row height adapts to aspect ratios.
fn mosaic_layout(area: RECT, count: usize, aspects: &[AspectHint]) -> Vec<RECT> {
    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);

    let cols = (count as f64).sqrt().ceil() as usize;
    let rows_count = count.div_ceil(cols);
    let row_h = total_h / rows_count as f64;

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let row = i / cols;
        let items_in_row = if row == rows_count - 1 && !count.is_multiple_of(cols) {
            count % cols
        } else {
            cols
        };

        // Weight each cell by its aspect ratio within the row
        let row_start = row * cols;
        let row_end = (row_start + items_in_row).min(count);
        let total_ratio: f64 = (row_start..row_end)
            .map(|j| aspects.get(j).map_or(1.5, |a| a.ratio().max(0.3)))
            .sum();

        let my_ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
        let my_w = total_w * (my_ratio / total_ratio);

        let x_offset: f64 = (row_start..i)
            .map(|j| {
                let r = aspects.get(j).map_or(1.5, |a| a.ratio().max(0.3));
                total_w * (r / total_ratio)
            })
            .sum();

        rects.push(padded_rect(
            area.left + x_offset as i32,
            area.top + (row as f64 * row_h) as i32,
            area.left + (x_offset + my_w) as i32,
            area.top + ((row + 1) as f64 * row_h) as i32,
        ));
    }
    rects
}

// ───────────────────────── Bento ─────────────────────────

/// Primary window (60 % width) plus a sidebar stack.
fn bento_layout(area: RECT, count: usize) -> Vec<RECT> {
    if count == 1 {
        return vec![padded_rect(area.left, area.top, area.right, area.bottom)];
    }

    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);

    // Main window takes ~60% width, rest share the right column
    let main_w = (total_w * 0.6) as i32;
    let mut rects = Vec::with_capacity(count);

    // First element: main/large
    rects.push(padded_rect(
        area.left,
        area.top,
        area.left + main_w,
        area.bottom,
    ));

    let side_count = count - 1;
    let side_h = total_h / side_count as f64;

    for i in 0..side_count {
        rects.push(padded_rect(
            area.left + main_w,
            area.top + (i as f64 * side_h) as i32,
            area.right,
            area.top + ((i + 1) as f64 * side_h) as i32,
        ));
    }
    rects
}

// ───────────────────────── Fibonacci ─────────────────────────

/// Golden-ratio spiral subdivision.
fn fibonacci_layout(area: RECT, count: usize) -> Vec<RECT> {
    let mut rects = Vec::with_capacity(count);
    let mut x = f64::from(area.left);
    let mut y = f64::from(area.top);
    let mut w = f64::from(area.right - area.left);
    let mut h = f64::from(area.bottom - area.top);

    for i in 0..count {
        if i == count - 1 {
            // Last element takes remaining space
            rects.push(padded_rect(
                x as i32,
                y as i32,
                (x + w) as i32,
                (y + h) as i32,
            ));
        } else {
            match i % 4 {
                0 => {
                    // Split horizontally, take left portion
                    let split = w * 0.618;
                    rects.push(padded_rect(
                        x as i32,
                        y as i32,
                        (x + split) as i32,
                        (y + h) as i32,
                    ));
                    x += split;
                    w -= split;
                }
                1 => {
                    // Split vertically, take top portion
                    let split = h * 0.618;
                    rects.push(padded_rect(
                        x as i32,
                        y as i32,
                        (x + w) as i32,
                        (y + split) as i32,
                    ));
                    y += split;
                    h -= split;
                }
                2 => {
                    // Split horizontally, take right portion
                    let split = w * 0.618;
                    rects.push(padded_rect(
                        (x + w - split) as i32,
                        y as i32,
                        (x + w) as i32,
                        (y + h) as i32,
                    ));
                    w -= split;
                }
                3 => {
                    // Split vertically, take bottom portion
                    let split = h * 0.618;
                    rects.push(padded_rect(
                        x as i32,
                        (y + h - split) as i32,
                        (x + w) as i32,
                        (y + h) as i32,
                    ));
                    h -= split;
                }
                _ => unreachable!(),
            }
        }
    }
    rects
}

// ───────────────────────── Columns ─────────────────────────

/// Masonry-style: distribute items into columns, filling the shortest first.
fn columns_layout(area: RECT, count: usize, aspects: &[AspectHint]) -> Vec<RECT> {
    let num_cols = ((count as f64).sqrt().ceil() as usize).clamp(2, 5);
    let total_w = f64::from(area.right - area.left);
    let col_w = total_w / num_cols as f64;

    let mut col_heights = vec![0.0f64; num_cols];
    let mut assignments: Vec<(usize, f64)> = Vec::with_capacity(count); // (col_index, y_start)

    for i in 0..count {
        // Find shortest column
        let col = col_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map_or(0, |(idx, _)| idx);

        let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
        let item_h = col_w / ratio;

        assignments.push((col, col_heights[col]));
        col_heights[col] += item_h;
    }

    // Scale to fit available height.
    let max_h = col_heights.iter().copied().fold(0.0_f64, f64::max);
    let available_h = f64::from(area.bottom - area.top);
    let scale = if max_h > 0.0 {
        available_h / max_h
    } else {
        1.0
    };

    let mut rects = Vec::with_capacity(count);
    for (i, &(col, y_start)) in assignments.iter().enumerate() {
        let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
        let item_h = (col_w / ratio) * scale;
        let y = f64::from(area.top) + y_start * scale;

        rects.push(padded_rect(
            area.left + (col as f64 * col_w) as i32,
            y as i32,
            area.left + ((col + 1) as f64 * col_w) as i32,
            (y + item_h) as i32,
        ));
    }
    rects
}

// ───────────────────────── Helpers ─────────────────────────

/// Create a [`RECT`] with [`PADDING`] inset on every side.
fn padded_rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left: left + PADDING,
        top: top + PADDING,
        right: (right - PADDING).max(left + PADDING + 1),
        bottom: (bottom - PADDING).max(top + PADDING + 1),
    }
}
