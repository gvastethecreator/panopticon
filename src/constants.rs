//! Application-wide constants for Panopticon.
//!
//! Colour values use **BGR** format, which is the native Windows
//! [`COLORREF`](windows::Win32::Foundation::COLORREF) encoding.

// ── UI geometry ──────────────────────────────────────────────

/// Height of the toolbar area in pixels.
pub const TOOLBAR_HEIGHT: i32 = 48;

/// Height of the title/footer strip drawn over each thumbnail.
pub const THUMBNAIL_FOOTER_HEIGHT: i32 = 34;

/// Accent-strip height used for thumbnail cards.
pub const THUMBNAIL_ACCENT_HEIGHT: i32 = 3;

// ── Timers ───────────────────────────────────────────────────

/// Timer ID for periodic window-list refresh.
pub const TIMER_REFRESH: usize = 1;

/// Timer ID for lightweight layout animations.
pub const TIMER_ANIMATION: usize = 2;

/// Interval between automatic window refreshes, in milliseconds.
pub const REFRESH_INTERVAL_MS: u32 = 2000;

/// Duration of layout transition animations, in milliseconds.
pub const ANIMATION_DURATION_MS: u32 = 180;

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

/// Softer accent used for pills, tags and subtle UI highlights.
pub const ACCENT_SOFT_COLOR: u32 = 0x005C_4A38;

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

// ── Scroll ───────────────────────────────────────────────────

/// Pixels scrolled per mouse-wheel notch in Row / Column modes.
pub const SCROLL_STEP: i32 = 60;

/// Thickness of the hover-only overlay scrollbar.
pub const SCROLLBAR_THICKNESS: i32 = 8;

/// Distance from the window edge to the overlay scrollbar.
pub const SCROLLBAR_MARGIN: i32 = 10;

/// Minimum thumb size for the overlay scrollbar.
pub const SCROLLBAR_MIN_THUMB: i32 = 36;

/// Duration of the subtle edge feedback animation, in milliseconds.
pub const EDGE_FEEDBACK_DURATION_MS: u32 = 120;

/// Maximum pixel offset used by the subtle edge feedback animation.
pub const EDGE_FEEDBACK_DISTANCE: i32 = 10;

/// `A` key — toggle animations.
pub const VK_A: u16 = 0x41;

/// `H` key — toggle header visibility.
pub const VK_H: u16 = 0x48;

/// `I` key — toggle thumbnail metadata visibility.
pub const VK_I: u16 = 0x49;

/// `O` key — open the settings window.
pub const VK_O: u16 = 0x4F;

/// `P` key — toggle always-on-top mode.
pub const VK_P: u16 = 0x50;

/// Number-row shortcut for layout 1.
pub const VK_1: u16 = 0x31;

/// Number-row shortcut for layout 2.
pub const VK_2: u16 = 0x32;

/// Number-row shortcut for layout 3.
pub const VK_3: u16 = 0x33;

/// Number-row shortcut for layout 4.
pub const VK_4: u16 = 0x34;

/// Number-row shortcut for layout 5.
pub const VK_5: u16 = 0x35;

/// Number-row shortcut for layout 6.
pub const VK_6: u16 = 0x36;

/// Number-row shortcut for layout 7.
pub const VK_7: u16 = 0x37;
