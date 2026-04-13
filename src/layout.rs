//! Layout engine for Panopticon.
//!
//! Provides seven mathematical layout algorithms that arrange `n` window
//! thumbnails inside a given rectangular area.  All functions are **pure**
//! (no side effects, no I/O) and therefore fully unit-testable.
//!
//! Each layout can be customised via [`LayoutCustomization`], which stores
//! user-tweaked column/row ratios that override the default distribution.
//! Draggable separators ([`Separator`]) are computed alongside the rects
//! so the UI can render resize handles.

use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::RECT;

/// The five layout modes specified in the PRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Single horizontal row — each window takes 100 % height; scrollable.
    Row,
    /// Single vertical column — each window takes 100 % width; scrollable.
    Column,
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
            Self::Columns => Self::Row,
            Self::Row => Self::Column,
            Self::Column => Self::Grid,
        }
    }

    /// Stable storage key used for persisted layout customizations.
    #[must_use]
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Mosaic => "Mosaic",
            Self::Bento => "Bento",
            Self::Fibonacci => "Fibonacci",
            Self::Columns => "Columns",
            Self::Row => "Row",
            Self::Column => "Column",
        }
    }

    /// Translation key for the user-facing layout label.
    #[must_use]
    pub const fn translation_key(self) -> &'static str {
        match self {
            Self::Grid => "layout.grid",
            Self::Mosaic => "layout.mosaic",
            Self::Bento => "layout.bento",
            Self::Fibonacci => "layout.fibonacci",
            Self::Columns => "layout.columns",
            Self::Row => "layout.row",
            Self::Column => "layout.column",
        }
    }

    /// Backward-compatible stable label accessor.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.storage_key()
    }

    /// Axis along which this layout may produce content that overflows the
    /// visible area and therefore supports scrolling.
    #[must_use]
    pub fn scroll_direction(self) -> ScrollDirection {
        match self {
            Self::Row => ScrollDirection::Horizontal,
            Self::Column => ScrollDirection::Vertical,
            _ => ScrollDirection::None,
        }
    }
}

/// Scrolling axis for a layout mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Content fits within the visible area — no scrolling.
    None,
    /// Content may overflow horizontally (Row mode).
    Horizontal,
    /// Content may overflow vertically (Column mode).
    Vertical,
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

// ───────────────────────── Layout Customization ─────────────────────────

/// User-tweaked proportions that override the default layout distribution.
///
/// `col_ratios` and `row_ratios` are **normalized** (sum ≈ 1.0).  An empty
/// vector means "use algorithm defaults".  When the number of stored ratios
/// does not match the current window count, the engine falls back to
/// defaults automatically.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LayoutCustomization {
    /// Relative column widths (normalized weights summing to 1.0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub col_ratios: Vec<f64>,
    /// Relative row heights (normalized weights summing to 1.0).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_ratios: Vec<f64>,
}

// f64 doesn't impl Eq, but our ratios are well-behaved (no NaN).
impl Eq for LayoutCustomization {}

impl LayoutCustomization {
    /// Return `true` when no custom ratios are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.col_ratios.is_empty() && self.row_ratios.is_empty()
    }
}

/// Normalize a ratio vector so it sums to 1.0.
fn normalize_ratios(ratios: &[f64]) -> Vec<f64> {
    if ratios.is_empty() {
        return Vec::new();
    }

    if ratios
        .iter()
        .any(|ratio| !ratio.is_finite() || *ratio <= 0.0)
    {
        return default_ratios(ratios.len());
    }

    let sum: f64 = ratios.iter().sum();
    if !sum.is_finite() || sum <= 0.0 {
        return default_ratios(ratios.len());
    }

    let mut out = Vec::with_capacity(ratios.len());
    out.extend(ratios.iter().map(|r| r / sum));
    out
}

fn matching_ratios(ratios: Option<&[f64]>, expected_len: usize) -> Option<&[f64]> {
    ratios.filter(|ratios| ratios.len() == expected_len)
}

fn scaled_segments(total: f64, slots: usize, ratios: Option<&[f64]>) -> Vec<f64> {
    ratios.map_or_else(
        || vec![total / slots as f64; slots],
        |ratios| {
            let normalized = normalize_ratios(ratios);
            let mut out = Vec::with_capacity(normalized.len());
            out.extend(normalized.iter().map(|ratio| total * ratio));
            out
        },
    )
}

fn cumulative_positions(lengths: &[f64]) -> Vec<f64> {
    let mut positions = Vec::with_capacity(lengths.len() + 1);
    positions.push(0.0);
    let mut acc = 0.0;
    for &length in lengths {
        acc += length;
        positions.push(acc);
    }
    positions
}

// ───────────────────────── Separator ─────────────────────────

/// A draggable divider between adjacent layout cells.
#[derive(Debug, Clone, Copy)]
pub struct Separator {
    /// Position along the split axis (logical px, area-relative).
    pub position: i32,
    /// `true` = horizontal line (drag vertically to resize rows).
    /// `false` = vertical line (drag horizontally to resize columns).
    pub horizontal: bool,
    /// Index into `col_ratios` (vertical sep) or `row_ratios` (horizontal sep).
    pub ratio_index: usize,
    /// Start of the handle on the perpendicular axis.
    pub extent_start: i32,
    /// End of the handle on the perpendicular axis.
    pub extent_end: i32,
}

/// Result of a layout computation including separators for resize handles.
pub struct LayoutResult {
    /// Destination rectangles for each window.
    pub rects: Vec<RECT>,
    /// Draggable separators the UI can render as resize handles.
    pub separators: Vec<Separator>,
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
    compute_layout_custom(layout, area, count, aspects, None).rects
}

/// Compute destination rectangles **and** draggable separators.
///
/// When `custom` is `Some`, the stored ratios override the default
/// algorithm proportions where applicable.
#[must_use]
pub fn compute_layout_custom(
    layout: LayoutType,
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    if count == 0 {
        return LayoutResult {
            rects: Vec::new(),
            separators: Vec::new(),
        };
    }
    match layout {
        LayoutType::Grid => grid_layout_custom(area, count, custom),
        LayoutType::Mosaic => mosaic_layout_custom(area, count, aspects, custom),
        LayoutType::Bento => bento_layout_custom(area, count, custom),
        LayoutType::Fibonacci => {
            let rects = fibonacci_layout(area, count);
            LayoutResult {
                rects,
                separators: Vec::new(),
            }
        }
        LayoutType::Columns => columns_layout_custom(area, count, aspects, custom),
        LayoutType::Row => single_row_layout_custom(area, count, aspects, custom),
        LayoutType::Column => single_column_layout_custom(area, count, aspects, custom),
    }
}

// ───────────────────────── Grid ─────────────────────────

/// Equal-sized cells in a √n × √n grid, with optional custom ratios.
fn grid_layout_custom(
    area: RECT,
    count: usize,
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = count.div_ceil(cols);

    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);

    let col_w = scaled_segments(
        total_w,
        cols,
        matching_ratios(custom.map(|c| c.col_ratios.as_slice()), cols),
    );
    let row_h = scaled_segments(
        total_h,
        rows,
        matching_ratios(custom.map(|c| c.row_ratios.as_slice()), rows),
    );

    let col_x = cumulative_positions(&col_w);
    let row_y = cumulative_positions(&row_h);

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let col = i % cols;
        let row = i / cols;
        rects.push(padded_rect(
            area.left + col_x[col] as i32,
            area.top + row_y[row] as i32,
            area.left + col_x[col + 1] as i32,
            area.top + row_y[row + 1] as i32,
        ));
    }

    // Actual number of cells in the last (possibly partial) row.
    let last_row_cells = {
        let rem = count % cols;
        if rem == 0 {
            cols
        } else {
            rem
        }
    };

    // Separators — extents are clipped to the area where cells exist on both
    // sides, so handles never float over empty space in an incomplete grid.
    let mut separators = Vec::with_capacity(cols.saturating_sub(1) + rows.saturating_sub(1));

    // Vertical separators (between columns)
    for c in 0..cols.saturating_sub(1) {
        // If the last row doesn't reach col c+1, stop the handle before that row.
        let extent_row_end = if c + 1 >= last_row_cells {
            rows - 1
        } else {
            rows
        };
        separators.push(Separator {
            position: area.left + col_x[c + 1] as i32,
            horizontal: false,
            ratio_index: c,
            extent_start: area.top,
            extent_end: area.top + row_y[extent_row_end] as i32,
        });
    }
    // Horizontal separators (between rows)
    for r in 0..rows.saturating_sub(1) {
        // If the row below is the partial last row, clip to the columns present.
        let extent_col_end = if r + 1 == rows - 1 && last_row_cells < cols {
            last_row_cells
        } else {
            cols
        };
        separators.push(Separator {
            position: area.top + row_y[r + 1] as i32,
            horizontal: true,
            ratio_index: r,
            extent_start: area.left,
            extent_end: area.left + col_x[extent_col_end] as i32,
        });
    }

    LayoutResult { rects, separators }
}

// ───────────────────────── Mosaic ─────────────────────────

/// Rows of windows where each row height adapts to aspect ratios.
fn mosaic_layout_custom(
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);

    let cols = (count as f64).sqrt().ceil() as usize;
    let rows_count = count.div_ceil(cols);

    let row_heights = scaled_segments(
        total_h,
        rows_count,
        matching_ratios(custom.map(|c| c.row_ratios.as_slice()), rows_count),
    );

    let row_y = cumulative_positions(&row_heights);

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let row = i / cols;
        let items_in_row = if row == rows_count - 1 && !count.is_multiple_of(cols) {
            count % cols
        } else {
            cols
        };

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
            area.top + row_y[row] as i32,
            area.left + (x_offset + my_w) as i32,
            area.top + row_y[row + 1] as i32,
        ));
    }

    // Separators: horizontal between rows
    let mut separators = Vec::new();
    for r in 0..rows_count.saturating_sub(1) {
        separators.push(Separator {
            position: area.top + row_y[r + 1] as i32,
            horizontal: true,
            ratio_index: r,
            extent_start: area.left,
            extent_end: area.right,
        });
    }

    LayoutResult { rects, separators }
}

// ───────────────────────── Bento ─────────────────────────

/// Primary window (60 % width) plus a sidebar stack.
fn bento_layout_custom(
    area: RECT,
    count: usize,
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    if count == 1 {
        return LayoutResult {
            rects: vec![padded_rect(area.left, area.top, area.right, area.bottom)],
            separators: Vec::new(),
        };
    }

    let total_w = f64::from(area.right - area.left);
    let total_h = f64::from(area.bottom - area.top);

    // Main panel width ratio (default 0.6)
    let main_frac = custom
        .and_then(|c| c.col_ratios.first().copied())
        .unwrap_or(0.6)
        .clamp(0.15, 0.85);
    let main_w = (total_w * main_frac) as i32;

    let mut rects = Vec::with_capacity(count);
    rects.push(padded_rect(
        area.left,
        area.top,
        area.left + main_w,
        area.bottom,
    ));

    let side_count = count - 1;

    let side_heights = scaled_segments(
        total_h,
        side_count,
        matching_ratios(custom.map(|c| c.row_ratios.as_slice()), side_count),
    );

    let side_y = cumulative_positions(&side_heights);

    for i in 0..side_count {
        rects.push(padded_rect(
            area.left + main_w,
            area.top + side_y[i] as i32,
            area.right,
            area.top + side_y[i + 1] as i32,
        ));
    }

    // Separators
    let mut separators = Vec::new();
    // Vertical: main/sidebar split
    separators.push(Separator {
        position: area.left + main_w,
        horizontal: false,
        ratio_index: 0,
        extent_start: area.top,
        extent_end: area.bottom,
    });
    // Horizontal: between sidebar rows
    for i in 0..side_count.saturating_sub(1) {
        separators.push(Separator {
            position: area.top + side_y[i + 1] as i32,
            horizontal: true,
            ratio_index: i,
            extent_start: area.left + main_w,
            extent_end: area.right,
        });
    }

    LayoutResult { rects, separators }
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
fn columns_layout_custom(
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    let num_cols = ((count as f64).sqrt().ceil() as usize).clamp(2, 5);
    let total_w = f64::from(area.right - area.left);

    let col_widths = scaled_segments(
        total_w,
        num_cols,
        matching_ratios(custom.map(|c| c.col_ratios.as_slice()), num_cols),
    );

    let col_x = cumulative_positions(&col_widths);

    let mut col_heights = vec![0.0f64; num_cols];
    let mut assignments: Vec<(usize, f64)> = Vec::with_capacity(count);

    for i in 0..count {
        let col = col_heights
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map_or(0, |(idx, _)| idx);

        let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
        let item_h = col_widths[col] / ratio;

        assignments.push((col, col_heights[col]));
        col_heights[col] += item_h;
    }

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
        let item_h = (col_widths[col] / ratio) * scale;
        let y = f64::from(area.top) + y_start * scale;

        rects.push(padded_rect(
            area.left + col_x[col] as i32,
            y as i32,
            area.left + col_x[col + 1] as i32,
            (y + item_h) as i32,
        ));
    }

    // Separators: vertical between columns
    let mut separators = Vec::new();
    for c in 0..num_cols.saturating_sub(1) {
        separators.push(Separator {
            position: area.left + col_x[c + 1] as i32,
            horizontal: false,
            ratio_index: c,
            extent_start: area.top,
            extent_end: area.bottom,
        });
    }

    LayoutResult { rects, separators }
}

// ───────────────────────── Row ─────────────────────────

/// Single horizontal row: each window takes the full available height.
///
/// Cell widths are proportional to each window's aspect ratio.  When the
/// total natural width fits within the area the cells are scaled up to
/// fill; otherwise the returned [`RECT`]s extend beyond `area.right`
/// and the caller is expected to provide horizontal scrolling.
fn single_row_layout_custom(
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    let height = f64::from(area.bottom - area.top);
    let width = f64::from(area.right - area.left);

    let widths: Vec<f64> =
        if let Some(ratios) = matching_ratios(custom.map(|c| c.col_ratios.as_slice()), count) {
            // Custom ratios → scale to total natural or available width
            let ratios = normalize_ratios(ratios);
            let natural: Vec<f64> = (0..count)
                .map(|i| {
                    let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
                    height * ratio
                })
                .collect();
            let total: f64 = natural.iter().sum();
            let extent = total.max(width);
            ratios.iter().map(|r| extent * r).collect()
        } else {
            let natural: Vec<f64> = (0..count)
                .map(|i| {
                    let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
                    height * ratio
                })
                .collect();
            let total: f64 = natural.iter().sum();
            if total <= width {
                let scale = width / total;
                natural.iter().map(|w| w * scale).collect()
            } else {
                natural
            }
        };

    let col_x = cumulative_positions(&widths);

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        rects.push(padded_rect(
            area.left + col_x[i] as i32,
            area.top,
            area.left + col_x[i + 1] as i32,
            area.bottom,
        ));
    }

    // Separators: vertical between cells
    let mut separators = Vec::new();
    for i in 0..count.saturating_sub(1) {
        separators.push(Separator {
            position: area.left + col_x[i + 1] as i32,
            horizontal: false,
            ratio_index: i,
            extent_start: area.top,
            extent_end: area.bottom,
        });
    }

    LayoutResult { rects, separators }
}

// ───────────────────────── Column (vertical strip) ─────────────────────────

/// Single vertical column: each window takes the full available width.
///
/// Cell heights are inversely proportional to the aspect ratio.  Overflowing
/// content extends below `area.bottom` for vertical scrolling.
fn single_column_layout_custom(
    area: RECT,
    count: usize,
    aspects: &[AspectHint],
    custom: Option<&LayoutCustomization>,
) -> LayoutResult {
    let width = f64::from(area.right - area.left);
    let height = f64::from(area.bottom - area.top);

    let heights: Vec<f64> =
        if let Some(ratios) = matching_ratios(custom.map(|c| c.row_ratios.as_slice()), count) {
            let ratios = normalize_ratios(ratios);
            let natural: Vec<f64> = (0..count)
                .map(|i| {
                    let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
                    width / ratio
                })
                .collect();
            let total: f64 = natural.iter().sum();
            let extent = total.max(height);
            ratios.iter().map(|r| extent * r).collect()
        } else {
            let natural: Vec<f64> = (0..count)
                .map(|i| {
                    let ratio = aspects.get(i).map_or(1.5, |a| a.ratio().max(0.3));
                    width / ratio
                })
                .collect();
            let total: f64 = natural.iter().sum();
            if total <= height {
                let scale = height / total;
                natural.iter().map(|h| h * scale).collect()
            } else {
                natural
            }
        };

    let row_y = cumulative_positions(&heights);

    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        rects.push(padded_rect(
            area.left,
            area.top + row_y[i] as i32,
            area.right,
            area.top + row_y[i + 1] as i32,
        ));
    }

    // Separators: horizontal between cells
    let mut separators = Vec::new();
    for i in 0..count.saturating_sub(1) {
        separators.push(Separator {
            position: area.top + row_y[i + 1] as i32,
            horizontal: true,
            ratio_index: i,
            extent_start: area.left,
            extent_end: area.right,
        });
    }

    LayoutResult { rects, separators }
}

// ───────────────────────── Helpers ─────────────────────────

/// Apply a drag delta to the separator at `ratio_index`, adjusting two
/// adjacent entries in the given `ratios` vector.
///
/// `delta_fraction` is the signed fraction of the total axis extent the
/// separator was moved (positive = right/down).  The function clamps so
/// that neither neighbour goes below `min_fraction`.
pub fn apply_separator_drag(
    ratios: &mut [f64],
    ratio_index: usize,
    delta_fraction: f64,
    min_fraction: f64,
) {
    if ratio_index + 1 >= ratios.len() {
        return;
    }
    let a = ratios[ratio_index];
    let b = ratios[ratio_index + 1];
    let new_a = (a + delta_fraction).clamp(min_fraction, a + b - min_fraction);
    let new_b = a + b - new_a;
    ratios[ratio_index] = new_a;
    ratios[ratio_index + 1] = new_b;
}

/// Apply a drag delta to a separator while redistributing the remaining space
/// proportionally across *all* items on each side of the split.
///
/// This feels more natural for scrollable strip layouts (`Row` / `Column`) and
/// proportional column layouts because the rest of the items accommodate the
/// change instead of only shifting position.
pub fn apply_separator_drag_grouped(
    ratios: &mut [f64],
    ratio_index: usize,
    delta_fraction: f64,
    min_fraction: f64,
) {
    if ratio_index + 1 >= ratios.len() {
        return;
    }

    let left_len = ratio_index + 1;
    let right_len = ratios.len() - left_len;
    if right_len == 0 {
        return;
    }

    let left_sum: f64 = ratios[..left_len].iter().sum();
    let right_sum: f64 = ratios[left_len..].iter().sum();
    if left_sum <= 0.0 || right_sum <= 0.0 {
        return;
    }

    let min_left = min_fraction * left_len as f64;
    let min_right = min_fraction * right_len as f64;
    let new_left_sum = (left_sum + delta_fraction).clamp(min_left, 1.0 - min_right);
    let new_right_sum = 1.0 - new_left_sum;
    let left_scale = new_left_sum / left_sum;
    let right_scale = new_right_sum / right_sum;

    for ratio in &mut ratios[..left_len] {
        *ratio *= left_scale;
    }
    for ratio in &mut ratios[left_len..] {
        *ratio *= right_scale;
    }
}

/// Build default equal ratios for `n` slots.
#[must_use]
pub fn default_ratios(n: usize) -> Vec<f64> {
    if n == 0 {
        Vec::new()
    } else {
        vec![1.0 / n as f64; n]
    }
}

/// Create a [`RECT`] with [`PADDING`] inset on every side.
fn padded_rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left: left + PADDING,
        top: top + PADDING,
        right: (right - PADDING).max(left + PADDING + 1),
        bottom: (bottom - PADDING).max(top + PADDING + 1),
    }
}
