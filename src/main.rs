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
    ACCENT_SOFT_COLOR, ANIMATION_DURATION_MS, BG_COLOR, BORDER_COLOR, FALLBACK_TEXT_COLOR,
    HOVER_BORDER_COLOR, LABEL_COLOR, MAX_TITLE_CHARS, MUTED_TEXT_COLOR, PANEL_BG_COLOR, TB_COLOR,
    TEXT_COLOR, THUMBNAIL_ACCENT_HEIGHT, THUMBNAIL_FOOTER_HEIGHT, TIMER_ANIMATION, TIMER_REFRESH,
    TITLE_TRUNCATE_AT, TOOLBAR_HEIGHT, VK_ESCAPE, VK_R, VK_TAB,
};
use panopticon::layout::{compute_layout, AspectHint, LayoutType};
use panopticon::settings::AppSettings;
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::mem;
use std::ptr::NonNull;
use std::time::Instant;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
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
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetWindowPos, TrackPopupMenu,
    HWND_NOTOPMOST, HWND_TOPMOST, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
};

const CMD_WINDOW_HIDE_APP: u16 = 1;
const CMD_WINDOW_TOGGLE_ASPECT_RATIO: u16 = 2;
const CMD_WINDOW_TOGGLE_HIDE_ON_SELECT: u16 = 3;

// ───────────────────────── Application State ─────────────────────────

/// A window tracked by Panopticon, including its DWM thumbnail handle.
struct ManagedWindow {
    info: WindowInfo,
    thumbnail: Option<Thumbnail>,
    target_rect: RECT,
    display_rect: RECT,
    animation_from_rect: RECT,
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
    animation_started_at: Option<Instant>,
}

struct ToolbarStatus<'a> {
    hidden_count: usize,
    refresh_label: &'a str,
    always_on_top: bool,
    animate_transitions: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowMenuAction {
    HideApp,
    ToggleAspectRatio,
    ToggleHideOnSelect,
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
        animation_started_at: None,
    });

    let state_ptr = Box::into_raw(state);
    let hwnd = create_main_window(state_ptr);

    // SAFETY: state pointer was stored in `GWLP_USERDATA` during `WM_NCCREATE`.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            apply_topmost_mode(state.hwnd, state.settings.always_on_top);
            match TrayIcon::add(hwnd, state.icons.small) {
                Ok(tray_icon) => state.tray_icon = Some(tray_icon),
                Err(error) => tracing::error!(%error, "failed to initialise tray icon"),
            }
        }
    }

    let _ = refresh_windows(hwnd);
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
#[allow(clippy::too_many_lines)]
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
                if refresh_windows(hwnd) {
                    recompute_layout(hwnd);
                    let _ = InvalidateRect(hwnd, None, true);
                }
            } else if wparam.0 == TIMER_ANIMATION {
                advance_animation(hwnd);
            }
            LRESULT(0)
        }
        WM_SHOWWINDOW => {
            if wparam.0 != 0 {
                let _ = refresh_windows(hwnd);
                recompute_layout(hwnd);
                reset_refresh_timer(hwnd);
                let _ = InvalidateRect(hwnd, None, true);
            } else {
                release_all_thumbnails(hwnd);
                reset_refresh_timer(hwnd);
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
        WM_RBUTTONUP => {
            handle_right_click(hwnd, lparam_x(lparam), lparam_y(lparam));
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
            let tray_state = tray_menu_state(hwnd);
            if let Some(action) = handle_tray_message(hwnd, lparam, &tray_state) {
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
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
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
            let _ = refresh_windows(hwnd);
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
        TrayAction::ToggleAnimateTransitions => {
            update_settings(hwnd, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
        }
        TrayAction::ToggleDefaultAspectRatio => {
            update_settings(hwnd, |settings| {
                settings.preserve_aspect_ratio = !settings.preserve_aspect_ratio;
            });
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::ToggleDefaultHideOnSelect => {
            update_settings(hwnd, |settings| {
                settings.hide_on_select = !settings.hide_on_select;
            });
        }
        TrayAction::ToggleAlwaysOnTop => {
            update_settings(hwnd, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            // SAFETY: state lives on the current UI thread.
            unsafe {
                if let Some(state) = app_from_hwnd(hwnd) {
                    apply_topmost_mode(hwnd, state.settings.always_on_top);
                }
            }
        }
        TrayAction::RestoreHidden(app_id) => {
            update_settings(hwnd, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
            });
            let _ = refresh_windows(hwnd);
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::RestoreAllHidden => {
            update_settings(hwnd, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            let _ = refresh_windows(hwnd);
            recompute_layout(hwnd);
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
            animate_transitions: state.settings.animate_transitions,
            preserve_aspect_ratio: state.settings.preserve_aspect_ratio,
            hide_on_select: state.settings.hide_on_select,
            always_on_top: state.settings.always_on_top,
            hidden_apps: state.settings.hidden_app_entries(),
        }
    } else {
        TrayMenuState {
            window_visible: false,
            minimize_to_tray: true,
            close_to_tray: true,
            refresh_interval_ms: 2_000,
            animate_transitions: true,
            preserve_aspect_ratio: false,
            hide_on_select: true,
            always_on_top: false,
            hidden_apps: Vec::new(),
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
        let interval = effective_refresh_interval(hwnd);
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
    release_all_thumbnails(hwnd);
    // SAFETY: `hwnd` is our main window; hiding it keeps the message loop alive.
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    reset_refresh_timer(hwnd);
}

fn restore_from_tray(hwnd: HWND) {
    // SAFETY: `hwnd` is our main window.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
        let _ = InvalidateRect(hwnd, None, true);
    }
    let _ = refresh_windows(hwnd);
    recompute_layout(hwnd);
    reset_refresh_timer(hwnd);
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
        let accent_brush = CreateSolidBrush(COLORREF(ACCENT_SOFT_COLOR));

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
            &ToolbarStatus {
                hidden_count: state.settings.hidden_app_entries().len(),
                refresh_label: &state.settings.refresh_interval_label(),
                always_on_top: state.settings.always_on_top,
                animate_transitions: state.settings.animate_transitions,
            },
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
                accent_brush,
                panel_brush,
            );
        }

        let _ = DeleteObject(bg_brush);
        let _ = DeleteObject(toolbar_brush);
        let _ = DeleteObject(panel_brush);
        let _ = DeleteObject(border_brush);
        let _ = DeleteObject(footer_brush);
        let _ = DeleteObject(hover_brush);
        let _ = DeleteObject(accent_brush);
        let _ = EndPaint(hwnd, &ps);
    }
}

fn paint_windows(
    hdc: HDC,
    state: &AppState,
    border_brush: HBRUSH,
    footer_brush: HBRUSH,
    hover_brush: HBRUSH,
    accent_brush: HBRUSH,
    panel_brush: HBRUSH,
) {
    for (index, managed_window) in state.windows.iter().enumerate() {
        let outer_rect = managed_window.display_rect;
        // SAFETY: `hdc` and brushes are valid for the active paint pass.
        unsafe {
            FrameRect(hdc, &outer_rect, border_brush);
        }

        let accent_rect = RECT {
            left: outer_rect.left,
            top: outer_rect.top,
            right: outer_rect.right,
            bottom: outer_rect.top + THUMBNAIL_ACCENT_HEIGHT,
        };
        // SAFETY: `hdc` and brushes are valid for the active paint pass.
        unsafe {
            FillRect(hdc, &accent_rect, accent_brush);
        }

        if state.hover_index == Some(index) {
            let inner = RECT {
                left: outer_rect.left + 1,
                top: outer_rect.top + 1,
                right: outer_rect.right - 1,
                bottom: outer_rect.bottom - 1,
            };
            // SAFETY: `hdc` and brushes are valid for the active paint pass.
            unsafe {
                FrameRect(hdc, &outer_rect, hover_brush);
                FrameRect(hdc, &inner, hover_brush);
            }
        }

        let footer_rect = RECT {
            left: outer_rect.left,
            top: outer_rect.bottom - THUMBNAIL_FOOTER_HEIGHT,
            right: outer_rect.right,
            bottom: outer_rect.bottom,
        };
        // SAFETY: `hdc` and brushes are valid for the active paint pass.
        unsafe {
            FillRect(hdc, &footer_rect, footer_brush);
        }

        if managed_window.thumbnail.is_none() {
            paint_thumbnail_placeholder(hdc, managed_window, footer_rect, panel_brush);
        }

        let process_rect = RECT {
            left: footer_rect.left + 10,
            top: footer_rect.top,
            right: footer_rect.right - 120,
            bottom: footer_rect.bottom,
        };
        draw_text_line(
            hdc,
            &truncate_title(&managed_window.info.title),
            process_rect,
            LABEL_COLOR,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );

        draw_text_line(
            hdc,
            &truncate_title(&managed_window.info.app_label()),
            RECT {
                left: footer_rect.left + 120,
                top: footer_rect.top,
                right: footer_rect.right - 10,
                bottom: footer_rect.bottom,
            },
            MUTED_TEXT_COLOR,
            DT_RIGHT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    }
}

fn paint_thumbnail_placeholder(
    hdc: HDC,
    managed_window: &ManagedWindow,
    _footer_rect: RECT,
    panel_brush: HBRUSH,
) {
    let placeholder_rect = preview_area_for_card(managed_window.display_rect);
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
    status: &ToolbarStatus<'_>,
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

    let status_text = format!(
        "{}  ·  {} visibles  ·  {} ocultas  ·  {} refresh",
        layout.label(),
        window_count,
        status.hidden_count,
        status.refresh_label
    );
    draw_text_line(
        hdc,
        &status_text,
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
        &format!(
            "{}  ·  {}  ·  click der.: opciones por app  ·  Esc salir",
            if status.always_on_top {
                "siempre visible"
            } else {
                "ventana normal"
            },
            if status.animate_transitions {
                "animaciones on"
            } else {
                "animaciones off"
            }
        ),
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
fn refresh_windows(hwnd: HWND) -> bool {
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return false;
    };

    let host_visible = unsafe { IsWindowVisible(state.hwnd).as_bool() };
    let discovered_all: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|window| window.hwnd != state.hwnd)
        .collect();

    for window in &discovered_all {
        state
            .settings
            .refresh_app_label(&window.app_id, &window.app_label());
    }

    let discovered: Vec<WindowInfo> = discovered_all
        .into_iter()
        .filter(|window| !state.settings.is_hidden(&window.app_id))
        .collect();

    let discovered_map: HashMap<isize, WindowInfo> = discovered
        .iter()
        .cloned()
        .map(|window| (window.hwnd.0 as isize, window))
        .collect();
    let discovered_hwnds: HashSet<isize> = discovered_map.keys().copied().collect();

    let previous_len = state.windows.len();
    state
        .windows
        .retain(|managed_window| discovered_hwnds.contains(&(managed_window.info.hwnd.0 as isize)));
    let mut changed = state.windows.len() != previous_len;

    for managed_window in &mut state.windows {
        let Some(fresh) = discovered_map.get(&(managed_window.info.hwnd.0 as isize)) else {
            continue;
        };

        let metadata_changed = fresh.title != managed_window.info.title
            || fresh.app_id != managed_window.info.app_id
            || fresh.process_name != managed_window.info.process_name
            || fresh.class_name != managed_window.info.class_name;
        if metadata_changed {
            managed_window.info = fresh.clone();
            changed = true;
        }

        if host_visible {
            let thumbnail_created = ensure_thumbnail(state.hwnd, managed_window);
            if thumbnail_created {
                changed = true;
            }

            if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
                let fresh_size = query_source_size(thumbnail.handle());
                if fresh_size.cx != managed_window.source_size.cx
                    || fresh_size.cy != managed_window.source_size.cy
                {
                    managed_window.source_size = fresh_size;
                    changed = true;
                }
            }
        }
    }

    let existing_hwnds: HashSet<isize> = state
        .windows
        .iter()
        .map(|managed_window| managed_window.info.hwnd.0 as isize)
        .collect();

    for info in discovered {
        if existing_hwnds.contains(&(info.hwnd.0 as isize)) {
            continue;
        }

        let mut managed_window = ManagedWindow {
            info,
            thumbnail: None,
            target_rect: RECT::default(),
            display_rect: RECT::default(),
            animation_from_rect: RECT::default(),
            source_size: SIZE { cx: 800, cy: 600 },
        };
        if host_visible {
            let _ = ensure_thumbnail(state.hwnd, &mut managed_window);
        }
        state.windows.push(managed_window);
        changed = true;
    }

    changed
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
        state.animation_started_at = None;
        unsafe {
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
        }
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

    let can_animate = state.settings.animate_transitions
        && unsafe { IsWindowVisible(state.hwnd).as_bool() }
        && state
            .windows
            .iter()
            .any(|managed_window| rect_has_area(managed_window.display_rect));
    let mut animation_needed = false;

    for (index, managed_window) in state.windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(index) {
            let previous_rect = if rect_has_area(managed_window.display_rect) {
                managed_window.display_rect
            } else {
                rect
            };

            managed_window.animation_from_rect = previous_rect;
            managed_window.target_rect = rect;
            if can_animate && previous_rect != rect {
                animation_needed = true;
            } else {
                managed_window.display_rect = rect;
            }
        }
    }

    if animation_needed {
        state.animation_started_at = Some(Instant::now());
        unsafe {
            SetTimer(hwnd, TIMER_ANIMATION, 16, None);
        }
    } else {
        state.animation_started_at = None;
        unsafe {
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
        }
        for managed_window in &mut state.windows {
            managed_window.display_rect = managed_window.target_rect;
        }
    }

    update_window_previews(state);
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

    let hit = state
        .windows
        .iter()
        .find(|managed_window| point_in_rect(managed_window.display_rect, x, y))
        .map(|managed_window| {
            (
                managed_window.info.clone(),
                state
                    .settings
                    .hide_on_select_for(&managed_window.info.app_id),
            )
        });

    if let Some((info, hide_on_select)) = hit {
        tracing::info!(title = %info.title, app_id = %info.app_id, "activating window");
        activate_window(info.hwnd);
        if hide_on_select {
            hide_to_tray(hwnd);
        }
    }
}

fn handle_right_click(hwnd: HWND, x: i32, y: i32) {
    if y < TOOLBAR_HEIGHT {
        return;
    }

    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let hit = state
        .windows
        .iter()
        .find(|managed_window| point_in_rect(managed_window.display_rect, x, y))
        .map(|managed_window| {
            (
                managed_window.info.clone(),
                state
                    .settings
                    .preserve_aspect_ratio_for(&managed_window.info.app_id),
                state
                    .settings
                    .hide_on_select_for(&managed_window.info.app_id),
            )
        });

    if let Some((info, preserve_aspect_ratio, hide_on_select)) = hit {
        if let Some(action) =
            show_window_context_menu(hwnd, &info, preserve_aspect_ratio, hide_on_select)
        {
            handle_window_menu_action(hwnd, &info, action);
        }
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
        state
            .windows
            .iter()
            .position(|managed_window| point_in_rect(managed_window.display_rect, x, y))
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

fn handle_window_menu_action(hwnd: HWND, info: &WindowInfo, action: WindowMenuAction) {
    match action {
        WindowMenuAction::HideApp => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_hidden(&info.app_id, &info.app_label());
            });
            let _ = refresh_windows(hwnd);
            recompute_layout(hwnd);
        }
        WindowMenuAction::ToggleAspectRatio => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_app_preserve_aspect_ratio(&info.app_id, &info.app_label());
            });
            recompute_layout(hwnd);
        }
        WindowMenuAction::ToggleHideOnSelect => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_app_hide_on_select(&info.app_id, &info.app_label());
            });
        }
    }

    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
    }
}

fn show_window_context_menu(
    hwnd: HWND,
    info: &WindowInfo,
    preserve_aspect_ratio: bool,
    hide_on_select: bool,
) -> Option<WindowMenuAction> {
    // SAFETY: menu is created and destroyed on the UI thread.
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let hide_label = wide("Hide from layout");
        let aspect_label = wide("Respect aspect ratio");
        let hide_after_open_label = wide("Hide Panopticon after opening this app");

        let _ = info;
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_WINDOW_HIDE_APP as usize,
            PCWSTR(hide_label.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_menu_flag(preserve_aspect_ratio),
            CMD_WINDOW_TOGGLE_ASPECT_RATIO as usize,
            PCWSTR(aspect_label.as_ptr()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | checked_menu_flag(hide_on_select),
            CMD_WINDOW_TOGGLE_HIDE_ON_SELECT as usize,
            PCWSTR(hide_after_open_label.as_ptr()),
        );

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            cursor.x,
            cursor.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);

        match command.0 as u16 {
            CMD_WINDOW_HIDE_APP => Some(WindowMenuAction::HideApp),
            CMD_WINDOW_TOGGLE_ASPECT_RATIO => Some(WindowMenuAction::ToggleAspectRatio),
            CMD_WINDOW_TOGGLE_HIDE_ON_SELECT => Some(WindowMenuAction::ToggleHideOnSelect),
            _ => None,
        }
    }
}

fn advance_animation(hwnd: HWND) {
    // SAFETY: state lives on the current UI thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let Some(started_at) = state.animation_started_at else {
        unsafe {
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
        }
        return;
    };

    if !unsafe { IsWindowVisible(hwnd).as_bool() } {
        state.animation_started_at = None;
        unsafe {
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
        }
        return;
    }

    let elapsed_ms = started_at.elapsed().as_millis() as u32;
    let progress = (elapsed_ms as f32 / ANIMATION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);

    for managed_window in &mut state.windows {
        managed_window.display_rect = lerp_rect(
            managed_window.animation_from_rect,
            managed_window.target_rect,
            eased,
        );
    }

    update_window_previews(state);

    unsafe {
        let _ = InvalidateRect(hwnd, None, false);
    }

    if progress >= 1.0 {
        state.animation_started_at = None;
        unsafe {
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
        }
    }
}

fn update_window_previews(state: &mut AppState) {
    if !unsafe { IsWindowVisible(state.hwnd).as_bool() } {
        return;
    }

    let settings = &state.settings;
    let owner_hwnd = state.hwnd;
    for managed_window in &mut state.windows {
        let preserve_aspect_ratio = settings.preserve_aspect_ratio_for(&managed_window.info.app_id);
        let _ = ensure_thumbnail(owner_hwnd, managed_window);
        if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
            let destination = preview_destination_rect(
                managed_window.display_rect,
                managed_window.source_size,
                preserve_aspect_ratio,
            );
            if thumbnail.update(destination, true).is_err() {
                tracing::warn!(title = %managed_window.info.title, "thumbnail update failed — dropping");
                managed_window.thumbnail = None;
            }
        }
    }
}

fn ensure_thumbnail(owner_hwnd: HWND, managed_window: &mut ManagedWindow) -> bool {
    if managed_window.thumbnail.is_some() {
        return false;
    }

    if let Ok(thumbnail) = Thumbnail::register(owner_hwnd, managed_window.info.hwnd) {
        managed_window.source_size = query_source_size(thumbnail.handle());
        managed_window.thumbnail = Some(thumbnail);
        true
    } else {
        false
    }
}

fn release_all_thumbnails(hwnd: HWND) {
    // SAFETY: state lives on the current UI thread.
    if let Some(state) = unsafe { app_from_hwnd(hwnd) } {
        for managed_window in &mut state.windows {
            managed_window.thumbnail = None;
        }
    }
}

fn preview_destination_rect(
    card_rect: RECT,
    source_size: SIZE,
    preserve_aspect_ratio: bool,
) -> RECT {
    let preview_area = preview_area_for_card(card_rect);
    if !preserve_aspect_ratio || source_size.cx <= 0 || source_size.cy <= 0 {
        return preview_area;
    }

    let area_width = (preview_area.right - preview_area.left).max(1);
    let area_height = (preview_area.bottom - preview_area.top).max(1);
    let width_ratio = area_width as f32 / source_size.cx as f32;
    let height_ratio = area_height as f32 / source_size.cy as f32;
    let scale = width_ratio.min(height_ratio);
    let render_width = (source_size.cx as f32 * scale).round() as i32;
    let render_height = (source_size.cy as f32 * scale).round() as i32;

    RECT {
        left: preview_area.left + ((area_width - render_width) / 2),
        top: preview_area.top + ((area_height - render_height) / 2),
        right: preview_area.left + ((area_width - render_width) / 2) + render_width,
        bottom: preview_area.top + ((area_height - render_height) / 2) + render_height,
    }
}

fn preview_area_for_card(card_rect: RECT) -> RECT {
    RECT {
        left: card_rect.left + 1,
        top: card_rect.top + THUMBNAIL_ACCENT_HEIGHT,
        right: card_rect.right - 1,
        bottom: card_rect.bottom - THUMBNAIL_FOOTER_HEIGHT,
    }
}

fn effective_refresh_interval(hwnd: HWND) -> u32 {
    // SAFETY: state lives on the current UI thread.
    unsafe {
        app_from_hwnd(hwnd).map_or(2_000, |state| {
            if IsWindowVisible(hwnd).as_bool() {
                state.settings.refresh_interval_ms
            } else {
                state.settings.refresh_interval_ms.max(10_000)
            }
        })
    }
}

fn apply_topmost_mode(hwnd: HWND, always_on_top: bool) {
    // SAFETY: `hwnd` is the main application window; z-order changes are local to it.
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            if always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
}

fn point_in_rect(rect: RECT, x: i32, y: i32) -> bool {
    x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

fn rect_has_area(rect: RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn lerp_rect(from: RECT, to: RECT, progress: f32) -> RECT {
    RECT {
        left: lerp_i32(from.left, to.left, progress),
        top: lerp_i32(from.top, to.top, progress),
        right: lerp_i32(from.right, to.right, progress),
        bottom: lerp_i32(from.bottom, to.bottom, progress),
    }
}

fn lerp_i32(from: i32, to: i32, progress: f32) -> i32 {
    (from as f32 + (to - from) as f32 * progress).round() as i32
}

fn checked_menu_flag(enabled: bool) -> MENU_ITEM_FLAGS {
    if enabled {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
