//! Application-wide constants for Panopticon.

// ── UI geometry ──────────────────────────────────────────────

/// Height of the toolbar area in pixels.
pub const TOOLBAR_HEIGHT: i32 = 48;

/// Accent-strip height used for thumbnail cards.
pub const THUMBNAIL_ACCENT_HEIGHT: i32 = 3;

// ── Timers ───────────────────────────────────────────────────

/// Duration of layout transition animations, in milliseconds.
pub const ANIMATION_DURATION_MS: u32 = 180;

// ── Text limits ──────────────────────────────────────────────

/// Maximum character count shown in a window-title label.
pub const MAX_TITLE_CHARS: usize = 40;

/// Position at which a long title is truncated (before appending "…").
pub const TITLE_TRUNCATE_AT: usize = 37;
