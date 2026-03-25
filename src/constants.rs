//! Application-wide constants for Panopticon.
//!
//! Colour values use **BGR** format, which is the native Windows
//! [`COLORREF`](windows::Win32::Foundation::COLORREF) encoding.

// ── UI geometry ──────────────────────────────────────────────

/// Height of the toolbar area in pixels.
pub const TOOLBAR_HEIGHT: i32 = 48;

/// Height of the title/footer strip drawn over each thumbnail.
pub const THUMBNAIL_FOOTER_HEIGHT: i32 = 24;

// ── Timers ───────────────────────────────────────────────────

/// Timer ID for periodic window-list refresh.
pub const TIMER_REFRESH: usize = 1;

/// Interval between automatic window refreshes, in milliseconds.
pub const REFRESH_INTERVAL_MS: u32 = 2000;

// ── Colours (BGR) ────────────────────────────────────────────

/// Primary application background colour.
pub const BG_COLOR: u32 = 0x0018_1513;

/// Toolbar background colour.
pub const TB_COLOR: u32 = 0x0022_1E1C;

/// Subtle panel / card colour used for empty states and window placeholders.
pub const PANEL_BG_COLOR: u32 = 0x002A_2522;

/// Border and separator colour.
pub const BORDER_COLOR: u32 = 0x0038_312E;

/// Accent colour used for highlights and the brand icon.
pub const ACCENT_COLOR: u32 = 0x00D2_9A5C;

/// Primary text colour (light grey).
pub const TEXT_COLOR: u32 = 0x00E6_E2DE;

/// Window-title label colour.
pub const LABEL_COLOR: u32 = 0x00C8_C1BA;

/// Muted secondary text colour.
pub const MUTED_TEXT_COLOR: u32 = 0x008D_867F;

/// Highlight border colour for hovered thumbnails.
pub const HOVER_BORDER_COLOR: u32 = 0x00D2_9A5C;

/// Fallback text colour for minimised-window placeholders.
pub const FALLBACK_TEXT_COLOR: u32 = 0x0080_7A74;

// ── Virtual-key codes ────────────────────────────────────────

/// Tab key — cycle layout mode.
pub const VK_TAB: u16 = 0x09;

/// Escape key — close the application.
pub const VK_ESCAPE: u16 = 0x1B;

/// `R` key — manual refresh.
pub const VK_R: u16 = 0x52;

// ── Text limits ──────────────────────────────────────────────

/// Maximum character count shown in a window-title label.
pub const MAX_TITLE_CHARS: usize = 40;

/// Position at which a long title is truncated (before appending "…").
pub const TITLE_TRUNCATE_AT: usize = 37;
