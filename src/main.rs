#![windows_subsystem = "windows"]
// Win32 interop requires pervasive integer ↔ float casting.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::wildcard_imports,
    // Win32 FFI functions accept *mut/*const via implicit &/&mut coercion.
    clippy::borrow_as_ptr
)]

//! Binary entry point for Panopticon — a real-time window thumbnail viewer.
//!
//! This module owns the Win32 window, message loop, global state, and painting
//! logic.  Domain logic (layouts, thumbnails, enumeration) lives in the library
//! crate so it can be unit-tested independently.

use panopticon::constants::{
    BG_COLOR, FALLBACK_TEXT_COLOR, HOVER_BORDER_COLOR, LABEL_COLOR, MAX_TITLE_CHARS,
    REFRESH_INTERVAL_MS, TB_COLOR, TEXT_COLOR, TIMER_REFRESH, TITLE_TRUNCATE_AT, TOOLBAR_HEIGHT,
    VK_ESCAPE, VK_R, VK_TAB,
};
use panopticon::layout::{compute_layout, AspectHint, LayoutType};
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};

use std::cell::UnsafeCell;
use std::mem;

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::DwmQueryThumbnailSourceSize;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect,
    InvalidateRect, SetBkMode, SetTextColor, DT_CENTER, DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER,
    HBRUSH, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

// ───────────────────────── Application State ─────────────────────────

/// A window tracked by Panopticon, including its DWM thumbnail handle.
struct ManagedWindow {
    info: WindowInfo,
    thumbnail: Option<Thumbnail>,
    target_rect: RECT,
    source_size: SIZE,
}

/// Root application state held for the lifetime of the process.
struct AppState {
    hwnd: HWND,
    windows: Vec<ManagedWindow>,
    current_layout: LayoutType,
    hover_index: Option<usize>,
}

/// Thread-unsafe holder for [`AppState`].
///
/// # Safety
///
/// [`Sync`] is implemented manually because [`UnsafeCell`] is `!Sync`.
/// This is sound **only** because:
///
/// 1. The Win32 message loop runs entirely on one thread.
/// 2. All reads/writes happen sequentially within message handler callbacks
///    dispatched by [`DispatchMessageW`].
/// 3. No reference to the inner state escapes to another thread.
struct AppStateHolder {
    inner: UnsafeCell<Option<AppState>>,
}

// SAFETY: see `AppStateHolder` doc – single-threaded Win32 message loop.
unsafe impl Sync for AppStateHolder {}

static APP: AppStateHolder = AppStateHolder {
    inner: UnsafeCell::new(None),
};

/// Returns `true` once the global [`AppState`] has been initialised.
///
/// This guard prevents panics when the window procedure receives messages
/// during [`CreateWindowExW`] (e.g. `WM_SIZE`), before state is ready.
fn is_state_ready() -> bool {
    // SAFETY: non-mutating read on the single message-loop thread.
    unsafe { (*APP.inner.get()).is_some() }
}

/// Obtain a mutable reference to the global application state.
///
/// # Safety
///
/// Must be called **only** from the Win32 message-loop thread and **only**
/// after [`AppState`] has been initialised (see [`is_state_ready`]).
/// The returned reference must not be held across calls that themselves
/// invoke `app()`.
unsafe fn app() -> &'static mut AppState {
    (*APP.inner.get())
        .as_mut()
        .expect("AppState not initialised")
}

// ───────────────────────── Entry Point ─────────────────────────

fn main() {
    // Logging — keep the guard alive for the entire process.
    let _log_guard = panopticon::logging::init().ok();

    tracing::info!("Panopticon starting");

    // Per-Monitor DPI awareness — must be set before any window is created.
    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let hwnd = create_main_window();

    // Initialise global state.
    // SAFETY: single-threaded; wnd_proc handlers are guarded by `is_state_ready()`
    // so they are no-ops during `CreateWindowExW`.
    unsafe {
        *APP.inner.get() = Some(AppState {
            hwnd,
            windows: Vec::new(),
            current_layout: LayoutType::Grid,
            hover_index: None,
        });
    }

    refresh_windows();
    recompute_layout();

    // Periodic timer for automatic refresh.
    // SAFETY: valid HWND, non-zero timer ID.
    unsafe {
        SetTimer(hwnd, TIMER_REFRESH, REFRESH_INTERVAL_MS, None);
    }

    tracing::info!("entering message loop");

    // SAFETY: standard Win32 message pump.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    tracing::info!("Panopticon exiting");
}

// ───────────────────────── Window Creation ─────────────────────────

/// Register the Win32 window class and create the main Panopticon window.
///
/// # Panics
///
/// Panics if `GetModuleHandleW` or `CreateWindowExW` fails.
fn create_main_window() -> HWND {
    // SAFETY: standard Win32 window class registration and creation.
    unsafe {
        let instance = GetModuleHandleW(None).expect("GetModuleHandleW failed");
        let class_name = w!("PanopticonClass");
        let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);

        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassExW(&wc);

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Panopticon \u{2014} Window Viewer"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1280,
            800,
            None,
            None,
            hinstance,
            None,
        )
        .expect("CreateWindowExW failed")
    }
}

// ───────────────────────── Window Procedure ─────────────────────────

/// Win32 window procedure (callback).
///
/// # Safety
///
/// Called by the OS on the message-loop thread.  `hwnd` is a valid window
/// handle for the lifetime of the call.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => {
            // Suppress default erase — painting is done entirely in WM_PAINT.
            LRESULT(1)
        }
        WM_PAINT => {
            if is_state_ready() {
                paint(hwnd);
            } else {
                // State not yet ready (during CreateWindowExW) — just validate
                // the paint region so Windows does not keep re-sending WM_PAINT.
                let mut ps = PAINTSTRUCT::default();
                let _ = BeginPaint(hwnd, &mut ps);
                let _ = EndPaint(hwnd, &ps);
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if is_state_ready() {
                recompute_layout();
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_REFRESH && is_state_ready() {
                refresh_windows();
                recompute_layout();
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            if wparam.0 != 0 && is_state_ready() {
                recompute_layout();
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            if is_state_ready() {
                handle_hover(lparam_x(lparam), lparam_y(lparam), hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            if is_state_ready() {
                handle_click(lparam_x(lparam), lparam_y(lparam));
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if is_state_ready() {
                handle_keydown(wparam.0 as u16, hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Release all DWM thumbnails before the window is destroyed.
            // SAFETY: single-threaded access during window teardown.
            if let Some(state) = &mut *APP.inner.get() {
                state.windows.clear();
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ── Input helpers ──────────────────────────────────────────────

/// Extract the signed X coordinate from an `LPARAM`.
#[inline]
fn lparam_x(lp: LPARAM) -> i32 {
    (lp.0 & 0xFFFF) as i16 as i32
}

/// Extract the signed Y coordinate from an `LPARAM`.
#[inline]
fn lparam_y(lp: LPARAM) -> i32 {
    ((lp.0 >> 16) & 0xFFFF) as i16 as i32
}

/// Handle a `WM_KEYDOWN` virtual-key code.
fn handle_keydown(vk: u16, hwnd: HWND) {
    match vk {
        VK_TAB => {
            // SAFETY: state is ready (caller checked `is_state_ready`).
            let state = unsafe { app() };
            state.current_layout = state.current_layout.next();
            tracing::debug!(layout = ?state.current_layout, "layout switched");
            recompute_layout();
            // SAFETY: valid HWND.
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        VK_ESCAPE => {
            // SAFETY: valid HWND.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        VK_R => {
            tracing::debug!("manual refresh requested");
            refresh_windows();
            recompute_layout();
            // SAFETY: valid HWND.
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        _ => {}
    }
}

// ───────────────────────── Painting ─────────────────────────

/// Render the toolbar, thumbnail frames, hover highlights and title labels.
fn paint(hwnd: HWND) {
    // SAFETY: all GDI calls operate on the HDC from BeginPaint, which is
    // valid until EndPaint.  Global state is accessed on the single
    // message-loop thread.
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let mut client_rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut client_rect);

        // ── Background ───────────────────────────────────────
        let bg_brush = CreateSolidBrush(COLORREF(BG_COLOR));
        FillRect(hdc, &client_rect, bg_brush);
        let _ = DeleteObject(bg_brush);

        // ── Toolbar ──────────────────────────────────────────
        let toolbar_rect = RECT {
            left: client_rect.left,
            top: client_rect.top,
            right: client_rect.right,
            bottom: client_rect.top + TOOLBAR_HEIGHT,
        };
        let tb_brush = CreateSolidBrush(COLORREF(TB_COLOR));
        FillRect(hdc, &toolbar_rect, tb_brush);
        let _ = DeleteObject(tb_brush);

        // ── Toolbar label ────────────────────────────────────
        let state = app();
        let label = format!(
            "  Layout: {} | Windows: {} | [Tab] Switch Layout  [R] Refresh  [Click] Activate  [Esc] Exit",
            state.current_layout.label(),
            state.windows.len(),
        );
        let mut label_wide: Vec<u16> = label.encode_utf16().collect();

        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(TEXT_COLOR));

        let mut text_rect = toolbar_rect;
        text_rect.left += 8;
        text_rect.top += 2;
        DrawTextW(
            hdc,
            &mut label_wide,
            &mut text_rect,
            DT_SINGLELINE | DT_VCENTER,
        );

        // ── Thumbnails ───────────────────────────────────────
        let hover_brush = CreateSolidBrush(COLORREF(HOVER_BORDER_COLOR));

        for (i, mw) in state.windows.iter().enumerate() {
            // Hover border (2 px).
            if state.hover_index == Some(i) {
                FrameRect(hdc, &mw.target_rect, hover_brush);
                let inner = RECT {
                    left: mw.target_rect.left + 1,
                    top: mw.target_rect.top + 1,
                    right: mw.target_rect.right - 1,
                    bottom: mw.target_rect.bottom - 1,
                };
                FrameRect(hdc, &inner, hover_brush);
            }

            // Title label.
            let title_short = truncate_title(&mw.info.title);
            SetTextColor(hdc, COLORREF(LABEL_COLOR));
            let mut wide: Vec<u16> = title_short.encode_utf16().collect();
            let mut label_rect = RECT {
                left: mw.target_rect.left,
                top: mw.target_rect.bottom - 20,
                right: mw.target_rect.right,
                bottom: mw.target_rect.bottom,
            };
            DrawTextW(
                hdc,
                &mut wide,
                &mut label_rect,
                DT_SINGLELINE | DT_CENTER | DT_END_ELLIPSIS,
            );

            // Fallback label for minimised windows.
            if mw.thumbnail.is_none() {
                SetTextColor(hdc, COLORREF(FALLBACK_TEXT_COLOR));
                let mut fallback: Vec<u16> = "[minimized]".encode_utf16().collect();
                let mut fr = mw.target_rect;
                fr.bottom -= 20;
                DrawTextW(
                    hdc,
                    &mut fallback,
                    &mut fr,
                    DT_SINGLELINE | DT_CENTER | DT_VCENTER,
                );
            }
        }

        let _ = DeleteObject(hover_brush);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Truncate a window title to [`MAX_TITLE_CHARS`], appending "..." if needed.
fn truncate_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() > MAX_TITLE_CHARS {
        let mut s: String = chars[..TITLE_TRUNCATE_AT].iter().collect();
        s.push_str("...");
        s
    } else {
        title.to_owned()
    }
}

// ───────────────────────── Window Refresh ─────────────────────────

/// Synchronise the internal window list with the current desktop state.
///
/// * Removes thumbnails for windows that have been closed.
/// * Registers thumbnails for newly discovered windows.
/// * Updates titles and source sizes for existing windows.
fn refresh_windows() {
    // SAFETY: called from the single message-loop thread.
    let state = unsafe { app() };

    let discovered: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|w| w.hwnd != state.hwnd)
        .collect();

    let discovered_hwnds: Vec<HWND> = discovered.iter().map(|w| w.hwnd).collect();

    // Drop stale entries — `Thumbnail::drop` calls `DwmUnregisterThumbnail`.
    state
        .windows
        .retain(|mw| discovered_hwnds.contains(&mw.info.hwnd));

    // Register new windows.
    for info in &discovered {
        if !state.windows.iter().any(|mw| mw.info.hwnd == info.hwnd) {
            let thumb = Thumbnail::register(state.hwnd, info.hwnd).ok();
            let source_size = thumb
                .as_ref()
                .map_or(SIZE { cx: 800, cy: 600 }, |t| query_source_size(t.handle()));
            state.windows.push(ManagedWindow {
                info: info.clone(),
                thumbnail: thumb,
                target_rect: RECT::default(),
                source_size,
            });
        }
    }

    // Refresh titles and source sizes.
    for mw in &mut state.windows {
        if let Some(fresh) = discovered.iter().find(|fi| fi.hwnd == mw.info.hwnd) {
            mw.info.title.clone_from(&fresh.title);
        }
        if let Some(ref thumb) = mw.thumbnail {
            let fresh_size = query_source_size(thumb.handle());
            if fresh_size.cx != mw.source_size.cx || fresh_size.cy != mw.source_size.cy {
                mw.source_size = fresh_size;
            }
        }
    }
}

/// Query the native pixel size of a DWM thumbnail source.
fn query_source_size(handle: isize) -> SIZE {
    // SAFETY: `handle` is a live HTHUMBNAIL from `DwmRegisterThumbnail`.
    let mut size = unsafe { DwmQueryThumbnailSourceSize(handle).unwrap_or_default() };
    if size.cx == 0 {
        size.cx = 800;
    }
    if size.cy == 0 {
        size.cy = 600;
    }
    size
}

// ───────────────────────── Layout ─────────────────────────

/// Recalculate destination rectangles and update DWM thumbnail positions.
fn recompute_layout() {
    // SAFETY: called from the single message-loop thread.
    let state = unsafe { app() };
    if state.windows.is_empty() {
        return;
    }

    let mut client_rect = RECT::default();
    // SAFETY: `state.hwnd` is a valid window handle.
    unsafe {
        let _ = GetClientRect(state.hwnd, &mut client_rect);
    }

    let content_area = RECT {
        left: client_rect.left,
        top: client_rect.top + TOOLBAR_HEIGHT,
        right: client_rect.right,
        bottom: client_rect.bottom,
    };

    let aspects: Vec<AspectHint> = state
        .windows
        .iter()
        .map(|mw| AspectHint {
            width: f64::from(mw.source_size.cx),
            height: f64::from(mw.source_size.cy),
        })
        .collect();

    let rects = compute_layout(
        state.current_layout,
        content_area,
        state.windows.len(),
        &aspects,
    );

    for (i, mw) in state.windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(i) {
            mw.target_rect = rect;
            if let Some(ref thumb) = mw.thumbnail {
                if thumb.update(rect, true).is_err() {
                    tracing::warn!(title = %mw.info.title, "thumbnail update failed — dropping");
                    mw.thumbnail = None;
                }
            }
        }
    }
}

// ───────────────────────── Click to Activate ─────────────────────────

/// Handle a left-button click: switch layout (toolbar) or activate a window.
fn handle_click(x: i32, y: i32) {
    // SAFETY: called from the single message-loop thread.
    let state = unsafe { app() };

    // Click on toolbar → switch layout.
    if y < TOOLBAR_HEIGHT {
        state.current_layout = state.current_layout.next();
        tracing::debug!(layout = ?state.current_layout, "layout switched via click");
        recompute_layout();
        // SAFETY: valid HWND.
        unsafe {
            let _ = InvalidateRect(state.hwnd, None, true);
        }
        return;
    }

    // Find which thumbnail was clicked.
    let hit = state.windows.iter().find(|mw| {
        let r = &mw.target_rect;
        x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
    });

    if let Some(mw) = hit {
        let target = mw.info.hwnd;
        let self_hwnd = state.hwnd;
        tracing::info!(title = %mw.info.title, "activating window");
        activate_window(target);
        // SAFETY: valid HWND.
        unsafe {
            let _ = ShowWindow(self_hwnd, SW_MINIMIZE);
        }
    }
}

/// Bring the given window to the foreground, restoring it if minimised.
fn activate_window(hwnd: HWND) {
    // SAFETY: FFI calls with a valid HWND.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

// ───────────────────────── Hover ─────────────────────────

/// Update the hover index when the mouse moves over a thumbnail.
fn handle_hover(x: i32, y: i32, hwnd: HWND) {
    // SAFETY: called from the single message-loop thread.
    let state = unsafe { app() };

    let new_hover = if y >= TOOLBAR_HEIGHT {
        state.windows.iter().position(|mw| {
            let r = &mw.target_rect;
            x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
        })
    } else {
        None
    };

    if new_hover != state.hover_index {
        state.hover_index = new_hover;
        // SAFETY: valid HWND.
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}
