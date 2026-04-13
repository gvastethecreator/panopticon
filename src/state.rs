//! Application state types and thread-local infrastructure.
//!
//! Centralizes the root [`AppState`] struct, supporting types
//! ([`ManagedWindow`], [`ThemeAnimation`], [`DragState`]), the
//! [`PendingAction`] event queue, and the thread-local accessors used by the
//! Win32 subclass and secondary-window modules.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use panopticon::layout::{LayoutType, Separator};
use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::WindowInfo;

use windows::Win32::Foundation::{HWND, RECT, SIZE};
use windows::Win32::UI::WindowsAndMessaging::WNDPROC;

use crate::app::tray::{AppIcons, TrayAction, TrayIcon};
use crate::{MainWindow, SettingsWindow, TagDialogWindow};

// ───────────────────────── Constants ─────────────────────────

pub(crate) const THUMBNAIL_INFO_STRIP_HEIGHT: i32 = 26;
pub(crate) const THUMBNAIL_CONTENT_PADDING: i32 = 6;
pub(crate) const THEME_TRANSITION_DURATION_MS: u32 = 220;
pub(crate) const HIDDEN_THUMBNAIL_RECT: RECT = RECT {
    left: 0,
    top: 0,
    right: 1,
    bottom: 1,
};

// ───────────────────────── Core types ─────────────────────────

/// Root application state shared via `Rc<RefCell<…>>`.
pub(crate) struct AppState {
    pub(crate) hwnd: HWND,
    pub(crate) windows: Vec<ManagedWindow>,
    pub(crate) current_layout: LayoutType,
    pub(crate) active_hwnd: Option<HWND>,
    pub(crate) tray_icon: Option<TrayIcon>,
    pub(crate) icons: AppIcons,
    pub(crate) settings: AppSettings,
    pub(crate) animation_started_at: Option<Instant>,
    pub(crate) content_extent: i32,
    pub(crate) is_appbar: bool,
    pub(crate) profile_name: Option<String>,
    pub(crate) last_size: (i32, i32),
    /// Cached separators from the last layout computation.
    pub(crate) separators: Vec<Separator>,
    /// Active drag state: separator index being dragged.
    pub(crate) drag_separator: Option<DragState>,
    /// Last background image path loaded into the main window.
    pub(crate) loaded_background_path: Option<String>,
    /// Last theme snapshot rendered into Slint globals.
    pub(crate) current_theme: theme_catalog::UiTheme,
    /// Optional animated transition between theme snapshots.
    pub(crate) theme_animation: Option<ThemeAnimation>,
}

/// A window tracked by Panopticon, including its DWM thumbnail handle.
pub(crate) struct ManagedWindow {
    pub(crate) info: WindowInfo,
    pub(crate) thumbnail: Option<Thumbnail>,
    pub(crate) target_rect: RECT,
    pub(crate) display_rect: RECT,
    pub(crate) animation_from_rect: RECT,
    pub(crate) source_size: SIZE,
    /// Last time the DWM thumbnail was actually updated (for interval mode).
    pub(crate) last_thumb_update: Option<Instant>,
    /// Last destination rectangle applied to the DWM thumbnail.
    pub(crate) last_thumb_dest: Option<RECT>,
    /// Last visibility flag applied to the DWM thumbnail.
    pub(crate) last_thumb_visible: bool,
    /// Cached Slint image of the window's application icon.
    pub(crate) cached_icon: Option<slint::Image>,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeAnimation {
    pub(crate) to: theme_catalog::UiTheme,
    pub(crate) from_rgb: theme_catalog::RgbThemeSnapshot,
    pub(crate) to_rgb: theme_catalog::RgbThemeSnapshot,
    pub(crate) started_at: Instant,
}

/// Tracks an in-progress separator drag.
#[derive(Debug, Clone)]
pub(crate) struct DragState {
    /// Separator index (maps to the handle `index` field in Slint).
    pub(crate) separator_index: usize,
    /// Whether the separator is horizontal (drag vertically).
    pub(crate) horizontal: bool,
    /// Ratio-array index of the separator.
    pub(crate) ratio_index: usize,
    /// Total extent of the axis at drag start (width or height of content area).
    pub(crate) axis_extent: f64,
    /// Last pointer offset inside the handle, used for incremental movement.
    pub(crate) last_pointer_offset: f64,
}

/// Tracks middle-button pan drag state.
pub(crate) struct MiddlePanState {
    pub(crate) active: bool,
    pub(crate) last_x: i32,
    pub(crate) last_y: i32,
}

pub(crate) enum PendingAction {
    Tray(TrayAction),
    Reposition,
    HideToTray,
    Refresh,
    Exit,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartupArgs {
    Run { profile: Option<String> },
    PrintAndExit { message: String, stderr: bool },
}

// ───────────────────────── WNDPROC typed wrapper ─────────────────────────

/// Type-safe storage for a saved `WNDPROC` pointer.
///
/// Win32 subclassing stores the original window procedure as an `isize`
/// returned by `GetWindowLongPtrW`.  This wrapper encapsulates the
/// `isize ↔ WNDPROC` conversion (which requires `mem::transmute`) so
/// that unsafety is confined to a single, well-documented location.
#[derive(Clone, Copy, Default)]
pub(crate) struct SavedWndProc(isize);

impl SavedWndProc {
    /// Store the value returned by `GetWindowLongPtrW(…, GWL_WNDPROC)`.
    pub(crate) const fn from_raw(raw: isize) -> Self {
        Self(raw)
    }

    /// Returns `true` when no original procedure has been captured.
    pub(crate) const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Returns the raw `isize` value for restoring via `SetWindowLongPtrW`.
    pub(crate) const fn as_raw(self) -> isize {
        self.0
    }

    /// Convert to a `WNDPROC` suitable for `CallWindowProcW`.
    ///
    /// # Safety
    ///
    /// The stored value **must** have been obtained from
    /// `GetWindowLongPtrW(hwnd, GWL_WNDPROC)` on a live window whose
    /// procedure has not been freed or replaced since capture.
    pub(crate) unsafe fn as_wndproc(self) -> WNDPROC {
        debug_assert!(
            !self.is_null(),
            "attempted to convert a null SavedWndProc to WNDPROC"
        );
        // SAFETY: caller guarantees the isize was obtained from a live
        // WNDPROC.  The transmute has no size or alignment mismatch
        // because `WNDPROC` is `Option<unsafe extern "system" fn(…)>`
        // which is pointer-sized, same as `isize`.
        unsafe { std::mem::transmute(self.0) }
    }
}

// ───────────────────────── Thread-local state ─────────────────────────

thread_local! {
    /// Saved original WNDPROC for the main window subclass.
    pub(crate) static ORIGINAL_WNDPROC: Cell<SavedWndProc> =
        const { Cell::new(SavedWndProc(0)) };

    /// Root application state, set once native HWND is available.
    pub(crate) static UI_STATE: RefCell<Option<Rc<RefCell<AppState>>>> =
        const { RefCell::new(None) };

    /// Weak reference to the Slint main window.
    pub(crate) static UI_WINDOW: RefCell<Option<slint::Weak<MainWindow>>> =
        const { RefCell::new(None) };

    /// Action queue drained every UI timer tick.
    pub(crate) static PENDING_ACTIONS: RefCell<Vec<PendingAction>> =
        const { RefCell::new(Vec::new()) };

    /// Settings window instance (if open).
    pub(crate) static SETTINGS_WIN: RefCell<Option<SettingsWindow>> =
        const { RefCell::new(None) };

    /// Tag-dialog window instance (if open).
    pub(crate) static TAG_DIALOG_WIN: RefCell<Option<TagDialogWindow>> =
        const { RefCell::new(None) };

    /// Middle-button pan state.
    pub(crate) static PAN_STATE: RefCell<MiddlePanState> =
        const { RefCell::new(MiddlePanState { active: false, last_x: 0, last_y: 0 }) };

    /// Instant when the last scroll event occurred; used by the scrollbar
    /// auto-hide timer to determine when to fade out.
    pub(crate) static SCROLL_LAST_ACTIVITY: Cell<Option<Instant>> =
        const { Cell::new(None) };
}

// ───────────────────────── Thread-local helpers ─────────────────────────

/// Queue a [`PendingAction`] for processing on the next UI timer tick.
pub(crate) fn queue_action(action: PendingAction) {
    PENDING_ACTIONS.with(|q| q.borrow_mut().push(action));
}

/// Queue an application exit.
pub(crate) fn queue_exit_request() {
    queue_action(PendingAction::Exit);
}
