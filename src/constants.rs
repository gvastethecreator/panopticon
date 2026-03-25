//! Application-wide constants for Panopticon.
//!
//! Colour values use **BGR** format, which is the native Windows
//! [`COLORREF`](windows::Win32::Foundation::COLORREF) encoding.

// ── UI geometry ──────────────────────────────────────────────

/// Height of the toolbar area in pixels.
pub const TOOLBAR_HEIGHT: i32 = 36;

// ── Timers ───────────────────────────────────────────────────

/// Timer ID for periodic window-list refresh.
pub const TIMER_REFRESH: usize = 1;

/// Interval between automatic window refreshes, in milliseconds.
pub const REFRESH_INTERVAL_MS: u32 = 2000;

// ── Colours (BGR) ────────────────────────────────────────────

/// Dark background colour.
pub const BG_COLOR: u32 = 0x0020_1A18;

/// Toolbar background colour.
pub const TB_COLOR: u32 = 0x0030_2824;

/// Primary text colour (light grey).
pub const TEXT_COLOR: u32 = 0x00CC_CCCC;

/// Window-title label colour.
pub const LABEL_COLOR: u32 = 0x00AA_AAAA;

/// Highlight border colour for hovered thumbnails.
pub const HOVER_BORDER_COLOR: u32 = 0x0055_88CC;

/// Fallback text colour for minimised-window placeholders.
pub const FALLBACK_TEXT_COLOR: u32 = 0x0066_6666;

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
