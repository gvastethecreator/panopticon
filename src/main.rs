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
//! The binary owns the Win32 window, message loop, painting logic and runtime
//! state. State is attached directly to the application `HWND` via
//! `GWLP_USERDATA`, which avoids a process-global singleton and follows the
//! canonical Win32 pattern.

mod app;

use app::tray::{
    draw_window_icon, handle_tray_message, AppIcons, TrayAction, TrayIcon, TrayMenuState,
    WM_TRAYICON,
};
use panopticon::constants::{
    BG_COLOR, BORDER_COLOR, FALLBACK_TEXT_COLOR, HOVER_BORDER_COLOR, LABEL_COLOR, MAX_TITLE_CHARS,
    MUTED_TEXT_COLOR, PANEL_BG_COLOR, TB_COLOR, TEXT_COLOR, THUMBNAIL_FOOTER_HEIGHT, TIMER_REFRESH,
    TITLE_TRUNCATE_AT, TOOLBAR_HEIGHT, VK_ESCAPE, VK_R, VK_TAB,
};
use panopticon::layout::{compute_layout, AspectHint, LayoutType};
use panopticon::settings::AppSettings;
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};

use std::ffi::c_void;
use std::mem;
use std::ptr::NonNull;

use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::DwmQueryThumbnailSourceSize;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect,
    InvalidateRect, SetBkMode, SetTextColor, DRAW_TEXT_FORMAT, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_RIGHT, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC, PAINTSTRUCT, TRANSPARENT,
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

/// Root application state stored in the Win32 window's `GWLP_USERDATA` slot.
struct AppState {
    hwnd: HWND,
    windows: Vec<ManagedWindow>,
    current_layout: LayoutType,
    hover_index: Option<usize>,
    tray_icon: Option<TrayIcon>,
    icons: AppIcons,
    settings: AppSettings,
}

/// Obtain a mutable reference to the application state stored on the window.
///
/// # Safety
///
/// Must only be called on the window thread for `hwnd`, and the returned
/// mutable reference must not be held across re-entrant calls that also fetch
/// the same state.
unsafe fn app_from_hwnd(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    NonNull::new(ptr).map(|state| &mut *state.as_ptr())
}

// ───────────────────────── Entry Point ─────────────────────────

fn main() {
    let _log_guard = panopticon::logging::init().ok();
    tracing::info!("Panopticon starting");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let icons = AppIcons::new().unwrap_or_else(|error| {
        tracing::error!(%error, "custom app icon generation failed; falling back to system icon");
        AppIcons::fallback_system()
    });
    let settings = AppSettings::load_or_default().unwrap_or_else(|error| {
        tracing::error!(%error, "failed to load settings; using defaults");
        AppSettings::default()
    });

    let state = Box::new(AppState {
        hwnd: HWND::default(),
        windows: Vec::new(),
        current_layout: settings.initial_layout,
        hover_index: None,
        tray_icon: None,
        icons,
        settings,
    });

    let state_ptr = Box::into_raw(state);
    let hwnd = create_main_window(state_ptr);

    // SAFETY: state pointer was stored in `GWLP_USERDATA` during `WM_NCCREATE`.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            match TrayIcon::add(hwnd, state.icons.small) {
                Ok(tray_icon) => state.tray_icon = Some(tray_icon),
                Err(error) => tracing::error!(%error, "failed to initialise tray icon"),
            }
        }
    }

    refresh_windows(hwnd);
    recompute_layout(hwnd);

    reset_refresh_timer(hwnd);

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
fn create_main_window(state_ptr: *mut AppState) -> HWND {
    // SAFETY: `state_ptr` points to a boxed `AppState` allocated in `main`
    // and lives until `WM_NCDESTROY` reclaims it.
    unsafe {
        let instance = GetModuleHandleW(None).expect("GetModuleHandleW failed");
        let class_name = w!("PanopticonClass");
        let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
        let icons = &(*state_ptr).icons;

        let wc = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: icons.large,
            hIconSm: icons.small,
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszClassName: class_name,
            ..Default::default()
        };

        RegisterClassExW(&wc);

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Panopticon — Window Viewer"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1320,
            840,
            None,
            None,
            hinstance,
            Some(state_ptr.cast::<c_void>()),
        )
        .expect("CreateWindowExW failed")
    }
}

// ───────────────────────── Window Procedure ─────────────────────────

/// Win32 window procedure (callback).
///
/// # Safety
///
/// Called by the OS on the message-loop thread. `hwnd` is a valid window
/// handle for the lifetime of the call.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            if create_struct.is_null() {
                return LRESULT(0);
            }

            let state_ptr = (*create_struct).lpCreateParams.cast::<AppState>();
            if state_ptr.is_null() {
                return LRESULT(0);
            }

            (*state_ptr).hwnd = hwnd;
            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            LRESULT(1)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_SIZE => {
            if wparam.0 == SIZE_MINIMIZED as usize && should_hide_on_minimize(hwnd) {
                hide_to_tray(hwnd);
            } else {
                recompute_layout(hwnd);
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_REFRESH {
                refresh_windows(hwnd);
                recompute_layout(hwnd);
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            if wparam.0 != 0 {
                recompute_layout(hwnd);
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            handle_hover(hwnd, lparam_x(lparam), lparam_y(lparam));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            handle_click(hwnd, lparam_x(lparam), lparam_y(lparam));
            LRESULT(0)
        }
        WM_KEYDOWN => {
            handle_keydown(hwnd, wparam.0 as u16);
            LRESULT(0)
        }
        WM_CLOSE => {
            if should_hide_on_close(hwnd) {
                hide_to_tray(hwnd);
            } else {
                request_exit(hwnd);
            }
            LRESULT(0)
        }
        tray_message if tray_message == WM_TRAYICON => {
            if let Some(action) = handle_tray_message(hwnd, lparam, tray_menu_state(hwnd)) {
                handle_tray_action(hwnd, action);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = app_from_hwnd(hwnd) {
                state.windows.clear();
                if let Some(tray_icon) = state.tray_icon.as_mut() {
                    tray_icon.remove();
                }
            }
            let _ = KillTimer(hwnd, TIMER_REFRESH);
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let state_ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut AppState;
            if let Some(state_ptr) = NonNull::new(state_ptr) {
                drop(Box::from_raw(state_ptr.as_ptr()));
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ───────────────────────── Input + Tray ─────────────────────────

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
fn handle_keydown(hwnd: HWND, vk: u16) {
    match vk {
        VK_TAB => {
            cycle_layout(hwnd, "keyboard");
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        VK_ESCAPE => request_exit(hwnd),
        VK_R => {
            tracing::debug!("manual refresh requested");
            refresh_windows(hwnd);
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        _ => {}
    }
}

fn handle_tray_action(hwnd: HWND, action: TrayAction) {
    match action {
        TrayAction::Toggle => toggle_window_visibility(hwnd),
        TrayAction::Refresh => {
            refresh_windows(hwnd);
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::NextLayout => {
            cycle_layout(hwnd, "tray");
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::ToggleMinimizeToTray => update_settings(hwnd, |settings| {
            settings.minimize_to_tray = !settings.minimize_to_tray;
        }),
        TrayAction::ToggleCloseToTray => update_settings(hwnd, |settings| {
            settings.close_to_tray = !settings.close_to_tray;
        }),
        TrayAction::CycleRefreshInterval => {
            update_settings(hwnd, AppSettings::cycle_refresh_interval);
            reset_refresh_timer(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::Exit => request_exit(hwnd),
    }
}

fn tray_menu_state(hwnd: HWND) -> TrayMenuState {
    // SAFETY: state lives on the current window thread.
    let state = unsafe { app_from_hwnd(hwnd) };
    if let Some(state) = state {
        TrayMenuState {
            window_visible: unsafe { IsWindowVisible(hwnd).as_bool() },
            minimize_to_tray: state.settings.minimize_to_tray,
            close_to_tray: state.settings.close_to_tray,
            refresh_interval_ms: state.settings.refresh_interval_ms,
        }
    } else {
        TrayMenuState {
            window_visible: false,
            minimize_to_tray: true,
            close_to_tray: true,
            refresh_interval_ms: 2_000,
        }
    }
}

fn should_hide_on_minimize(hwnd: HWND) -> bool {
    // SAFETY: state lives on the current window thread.
    unsafe { app_from_hwnd(hwnd).is_none_or(|state| state.settings.minimize_to_tray) }
}

fn should_hide_on_close(hwnd: HWND) -> bool {
    // SAFETY: state lives on the current window thread.
    unsafe { app_from_hwnd(hwnd).is_none_or(|state| state.settings.close_to_tray) }
}

fn update_settings(hwnd: HWND, mutate: impl FnOnce(&mut AppSettings)) {
    // SAFETY: state lives on the current window thread.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            mutate(&mut state.settings);
            state.settings = state.settings.normalized();
            if let Err(error) = state.settings.save() {
                tracing::error!(%error, "failed to persist settings");
            }
        }
    }
}

fn cycle_layout(hwnd: HWND, source: &str) {
    // SAFETY: state lives on the current window thread.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            state.current_layout = state.current_layout.next();
            state.settings.initial_layout = state.current_layout;
            if let Err(error) = state.settings.save() {
                tracing::error!(%error, source = source, "failed to persist selected layout");
            }
            tracing::debug!(layout = ?state.current_layout, source = source, "layout switched");
        }
    }
}

fn reset_refresh_timer(hwnd: HWND) {
    // SAFETY: valid HWND for the main window; replacing the same timer ID is supported.
    unsafe {
        let _ = KillTimer(hwnd, TIMER_REFRESH);
        let interval =
            app_from_hwnd(hwnd).map_or(2_000, |state| state.settings.refresh_interval_ms);
        SetTimer(hwnd, TIMER_REFRESH, interval, None);
    }
}

fn toggle_window_visibility(hwnd: HWND) {
    // SAFETY: `hwnd` is our main window.
    unsafe {
        if IsWindowVisible(hwnd).as_bool() {
            hide_to_tray(hwnd);
        } else {
            restore_from_tray(hwnd);
        }
    }
}

fn hide_to_tray(hwnd: HWND) {
    // SAFETY: `hwnd` is our main window; hiding it keeps the message loop alive.
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

fn restore_from_tray(hwnd: HWND) {
    // SAFETY: `hwnd` is our main window.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
        let _ = InvalidateRect(hwnd, None, true);
    }
    recompute_layout(hwnd);
}

fn request_exit(hwnd: HWND) {
    tracing::info!("closing Panopticon");
    // SAFETY: `hwnd` is our main window and may be destroyed directly.
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

// ───────────────────────── Painting ─────────────────────────

/// Paint the current frame.
fn paint(hwnd: HWND) {
    // SAFETY: state lives in `GWLP_USERDATA` and is accessed on the same UI thread.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            paint_impl(hwnd, state);
        }
    }
}

fn paint_impl(hwnd: HWND, state: &AppState) {
    // SAFETY: all GDI calls operate on the HDC from `BeginPaint`, which is
    // valid until `EndPaint`.
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let mut client_rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut client_rect);

        let bg_brush = CreateSolidBrush(COLORREF(BG_COLOR));
        let toolbar_brush = CreateSolidBrush(COLORREF(TB_COLOR));
        let panel_brush = CreateSolidBrush(COLORREF(PANEL_BG_COLOR));
        let border_brush = CreateSolidBrush(COLORREF(BORDER_COLOR));
        let footer_brush = CreateSolidBrush(COLORREF(TB_COLOR));
        let hover_brush = CreateSolidBrush(COLORREF(HOVER_BORDER_COLOR));

        FillRect(hdc, &client_rect, bg_brush);

        let toolbar_rect = RECT {
            left: client_rect.left,
            top: client_rect.top,
            right: client_rect.right,
            bottom: client_rect.top + TOOLBAR_HEIGHT,
        };
        FillRect(hdc, &toolbar_rect, toolbar_brush);

        let separator_rect = RECT {
            left: toolbar_rect.left,
            top: toolbar_rect.bottom - 1,
            right: toolbar_rect.right,
            bottom: toolbar_rect.bottom,
        };
        FillRect(hdc, &separator_rect, border_brush);

        paint_toolbar(
            hdc,
            toolbar_rect,
            state.current_layout,
            state.windows.len(),
            &state.settings.refresh_interval_label(),
        );

        if state.windows.is_empty() {
            paint_empty_state(hdc, client_rect, state);
        } else {
            paint_windows(
                hdc,
                state,
                border_brush,
                footer_brush,
                hover_brush,
                panel_brush,
            );
        }

        let _ = DeleteObject(bg_brush);
        let _ = DeleteObject(toolbar_brush);
        let _ = DeleteObject(panel_brush);
        let _ = DeleteObject(border_brush);
        let _ = DeleteObject(footer_brush);
        let _ = DeleteObject(hover_brush);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_windows(
    hdc: HDC,
    state: &AppState,
    border_brush: HBRUSH,
    footer_brush: HBRUSH,
    hover_brush: HBRUSH,
    panel_brush: HBRUSH,
) {
    for (index, managed_window) in state.windows.iter().enumerate() {
        // SAFETY: `hdc` and brushes are valid for the active paint pass.
        unsafe {
            FrameRect(hdc, &managed_window.target_rect, border_brush);
        }

        if state.hover_index == Some(index) {
            let inner = RECT {
                left: managed_window.target_rect.left + 1,
                top: managed_window.target_rect.top + 1,
                right: managed_window.target_rect.right - 1,
                bottom: managed_window.target_rect.bottom - 1,
            };
            // SAFETY: `hdc` and brushes are valid for the active paint pass.
            unsafe {
                FrameRect(hdc, &managed_window.target_rect, hover_brush);
                FrameRect(hdc, &inner, hover_brush);
            }
        }

        let footer_rect = RECT {
            left: managed_window.target_rect.left,
            top: managed_window.target_rect.bottom - THUMBNAIL_FOOTER_HEIGHT,
            right: managed_window.target_rect.right,
            bottom: managed_window.target_rect.bottom,
        };
        // SAFETY: `hdc` and brushes are valid for the active paint pass.
        unsafe {
            FillRect(hdc, &footer_rect, footer_brush);
        }

        if managed_window.thumbnail.is_none() {
            paint_thumbnail_placeholder(hdc, managed_window, footer_rect, panel_brush);
        }

        let title_rect = RECT {
            left: footer_rect.left + 10,
            top: footer_rect.top,
            right: footer_rect.right - 10,
            bottom: footer_rect.bottom,
        };
        draw_text_line(
            hdc,
            &truncate_title(&managed_window.info.title),
            title_rect,
            LABEL_COLOR,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
}

fn paint_thumbnail_placeholder(
    hdc: HDC,
    managed_window: &ManagedWindow,
    footer_rect: RECT,
    panel_brush: HBRUSH,
) {
    let placeholder_rect = RECT {
        left: managed_window.target_rect.left + 1,
        top: managed_window.target_rect.top + 1,
        right: managed_window.target_rect.right - 1,
        bottom: footer_rect.top,
    };
    // SAFETY: `hdc` and brushes are valid for the active paint pass.
    unsafe {
        FillRect(hdc, &placeholder_rect, panel_brush);
    }
    draw_window_icon(hdc, managed_window.info.hwnd, placeholder_rect, 32);

    let caption_rect = RECT {
        left: placeholder_rect.left + 12,
        top: placeholder_rect.bottom - 34,
        right: placeholder_rect.right - 12,
        bottom: placeholder_rect.bottom - 10,
    };
    draw_text_line(
        hdc,
        "Live preview unavailable",
        caption_rect,
        FALLBACK_TEXT_COLOR,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );
}

fn paint_toolbar(
    hdc: HDC,
    toolbar_rect: RECT,
    layout: LayoutType,
    window_count: usize,
    refresh_label: &str,
) {
    draw_text_line(
        hdc,
        "Panopticon",
        RECT {
            left: toolbar_rect.left + 14,
            top: toolbar_rect.top,
            right: toolbar_rect.left + 220,
            bottom: toolbar_rect.bottom,
        },
        TEXT_COLOR,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );

    let status = format!(
        "{}  ·  {} windows  ·  {} refresh",
        layout.label(),
        window_count,
        refresh_label
    );
    draw_text_line(
        hdc,
        &status,
        RECT {
            left: toolbar_rect.left + 180,
            top: toolbar_rect.top,
            right: toolbar_rect.right - 260,
            bottom: toolbar_rect.bottom,
        },
        LABEL_COLOR,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );

    draw_text_line(
        hdc,
        "Tab layout  ·  R refresh  ·  tray menu  ·  Esc exit",
        RECT {
            left: toolbar_rect.left + 240,
            top: toolbar_rect.top,
            right: toolbar_rect.right - 14,
            bottom: toolbar_rect.bottom,
        },
        MUTED_TEXT_COLOR,
        DT_RIGHT | DT_SINGLELINE | DT_VCENTER,
    );
}

fn paint_empty_state(hdc: HDC, client_rect: RECT, state: &AppState) {
    let content_rect = RECT {
        left: client_rect.left,
        top: client_rect.top + TOOLBAR_HEIGHT,
        right: client_rect.right,
        bottom: client_rect.bottom,
    };

    let card_width = 380;
    let card_height = 196;
    let card_rect = RECT {
        left: content_rect.left + ((content_rect.right - content_rect.left - card_width) / 2),
        top: content_rect.top + ((content_rect.bottom - content_rect.top - card_height) / 2),
        right: content_rect.left
            + ((content_rect.right - content_rect.left - card_width) / 2)
            + card_width,
        bottom: content_rect.top
            + ((content_rect.bottom - content_rect.top - card_height) / 2)
            + card_height,
    };

    // SAFETY: `hdc` is valid for the current paint pass.
    unsafe {
        let panel_brush = CreateSolidBrush(COLORREF(PANEL_BG_COLOR));
        let border_brush = CreateSolidBrush(COLORREF(BORDER_COLOR));
        FillRect(hdc, &card_rect, panel_brush);
        FrameRect(hdc, &card_rect, border_brush);
        let _ = DeleteObject(panel_brush);
        let _ = DeleteObject(border_brush);
        let _ = DrawIconEx(
            hdc,
            card_rect.left + ((card_rect.right - card_rect.left - 48) / 2),
            card_rect.top + 26,
            state.icons.large,
            48,
            48,
            0,
            None,
            DI_NORMAL,
        );
    }

    draw_text_line(
        hdc,
        "No windows available to preview",
        RECT {
            left: card_rect.left + 24,
            top: card_rect.top + 90,
            right: card_rect.right - 24,
            bottom: card_rect.top + 122,
        },
        TEXT_COLOR,
        DT_CENTER | DT_SINGLELINE | DT_VCENTER,
    );

    draw_text_line(
        hdc,
        "Open or restore any desktop window. Panopticon will keep watching from the tray.",
        RECT {
            left: card_rect.left + 24,
            top: card_rect.top + 126,
            right: card_rect.right - 24,
            bottom: card_rect.top + 164,
        },
        MUTED_TEXT_COLOR,
        DT_CENTER | DT_VCENTER,
    );
}

fn draw_text_line(hdc: HDC, text: &str, rect: RECT, color: u32, flags: DRAW_TEXT_FORMAT) {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = rect;

    // SAFETY: text buffer and destination rectangle are valid for the current draw.
    unsafe {
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(color));
        DrawTextW(hdc, &mut wide, &mut rect, flags);
    }
}

/// Truncate a window title to [`MAX_TITLE_CHARS`], appending `...` if needed.
fn truncate_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() > MAX_TITLE_CHARS {
        let mut short: String = chars[..TITLE_TRUNCATE_AT].iter().collect();
        short.push_str("...");
        short
    } else {
        title.to_owned()
    }
}

// ───────────────────────── Window Refresh ─────────────────────────

/// Synchronise the internal window list with the current desktop state.
fn refresh_windows(hwnd: HWND) {
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let discovered: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();

    let discovered_hwnds: Vec<HWND> = discovered.iter().map(|window| window.hwnd).collect();
    state
        .windows
        .retain(|managed_window| discovered_hwnds.contains(&managed_window.info.hwnd));

    for info in &discovered {
        if !state
            .windows
            .iter()
            .any(|managed_window| managed_window.info.hwnd == info.hwnd)
        {
            let thumbnail = Thumbnail::register(state.hwnd, info.hwnd).ok();
            let source_size = thumbnail
                .as_ref()
                .map_or(SIZE { cx: 800, cy: 600 }, |thumb| {
                    query_source_size(thumb.handle())
                });
            state.windows.push(ManagedWindow {
                info: info.clone(),
                thumbnail,
                target_rect: RECT::default(),
                source_size,
            });
        }
    }

    for managed_window in &mut state.windows {
        if let Some(fresh) = discovered
            .iter()
            .find(|window| window.hwnd == managed_window.info.hwnd)
        {
            managed_window.info.title.clone_from(&fresh.title);
        }
        if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
            let fresh_size = query_source_size(thumbnail.handle());
            if fresh_size.cx != managed_window.source_size.cx
                || fresh_size.cy != managed_window.source_size.cy
            {
                managed_window.source_size = fresh_size;
            }
        }
    }
}

/// Query the native pixel size of a DWM thumbnail source.
fn query_source_size(handle: isize) -> SIZE {
    // SAFETY: `handle` is a live `HTHUMBNAIL` from `DwmRegisterThumbnail`.
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
fn recompute_layout(hwnd: HWND) {
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

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
        .map(|managed_window| AspectHint {
            width: f64::from(managed_window.source_size.cx),
            height: f64::from(managed_window.source_size.cy),
        })
        .collect();

    let rects = compute_layout(
        state.current_layout,
        content_area,
        state.windows.len(),
        &aspects,
    );

    for (index, managed_window) in state.windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(index) {
            managed_window.target_rect = rect;
            if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
                if thumbnail.update(rect, true).is_err() {
                    tracing::warn!(title = %managed_window.info.title, "thumbnail update failed — dropping");
                    managed_window.thumbnail = None;
                }
            }
        }
    }
}

// ───────────────────────── Click to Activate ─────────────────────────

/// Handle a left-button click: switch layout (toolbar) or activate a window.
fn handle_click(hwnd: HWND, x: i32, y: i32) {
    if y < TOOLBAR_HEIGHT {
        cycle_layout(hwnd, "toolbar");
        recompute_layout(hwnd);
        unsafe {
            let _ = InvalidateRect(hwnd, None, true);
        }
        return;
    }

    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let hit = state.windows.iter().find(|managed_window| {
        let rect = &managed_window.target_rect;
        x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
    });

    if let Some(managed_window) = hit {
        tracing::info!(title = %managed_window.info.title, "activating window");
        activate_window(managed_window.info.hwnd);
        hide_to_tray(hwnd);
    }
}

/// Bring the given window to the foreground, restoring it if minimised.
fn activate_window(hwnd: HWND) {
    // SAFETY: FFI calls with a valid `HWND`.
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

// ───────────────────────── Hover ─────────────────────────

/// Update the hover index when the mouse moves over a thumbnail.
fn handle_hover(hwnd: HWND, x: i32, y: i32) {
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let new_hover = if y >= TOOLBAR_HEIGHT {
        state.windows.iter().position(|managed_window| {
            let rect = &managed_window.target_rect;
            x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
        })
    } else {
        None
    };

    if new_hover != state.hover_index {
        state.hover_index = new_hover;
        // SAFETY: valid `HWND` for the main window.
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}
