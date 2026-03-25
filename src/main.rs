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
use app::options::{
    open_options_window, open_tag_dialog, OptionsSubmit, TagCreateSubmit, WM_OPTIONS_APPLY,
    WM_OPTIONS_CLOSED, WM_TAG_CREATED,
};
use panopticon::constants::{
    ACCENT_COLOR, ANIMATION_DURATION_MS, BORDER_COLOR, FALLBACK_TEXT_COLOR, HOVER_BORDER_COLOR,
    LABEL_COLOR, MAX_TITLE_CHARS, MUTED_TEXT_COLOR, PANEL_BG_COLOR, SCROLL_STEP,
    SCROLLBAR_MARGIN, SCROLLBAR_MIN_THUMB, SCROLLBAR_THICKNESS, TB_COLOR, TEXT_COLOR,
    THUMBNAIL_ACCENT_HEIGHT, THUMBNAIL_FOOTER_HEIGHT, TIMER_ANIMATION, TIMER_REFRESH,
    TITLE_TRUNCATE_AT, TOOLBAR_HEIGHT, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_A,
    VK_ESCAPE, VK_H, VK_I, VK_O, VK_P, VK_R, VK_TAB,
};
use panopticon::layout::{compute_layout, AspectHint, LayoutType, ScrollDirection};
use panopticon::settings::{AppSelectionEntry, AppSettings, DockEdge};
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE,
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect, FrameRect,
    GetMonitorInfoW, InvalidateRect, MonitorFromWindow, SetBkMode, SetTextColor, DRAW_TEXT_FORMAT,
    DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC,
    MONITORINFO, MONITOR_DEFAULTTOPRIMARY, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, ABN_POSCHANGED, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetWindowPos, TrackPopupMenu,
    HWND_NOTOPMOST, HWND_TOPMOST, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
};

/// Callback message posted by the shell when the app-bar needs repositioning.
const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

/// Timer used to keep the hover-only scrollbar in sync with pointer presence.
const TIMER_HOVER_VISIBILITY: usize = 3;

/// Standard Win32 value: one wheel notch equals 120 delta units.
const WHEEL_DELTA: i32 = 120;

const CMD_WINDOW_HIDE_APP: u16 = 1;
const CMD_WINDOW_TOGGLE_ASPECT_RATIO: u16 = 2;
const CMD_WINDOW_TOGGLE_HIDE_ON_SELECT: u16 = 3;
const CMD_WINDOW_CREATE_TAG_FROM_APP: u16 = 4;
const CMD_WINDOW_TAG_BASE: u16 = 100;

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
    /// Scroll offset in pixels (horizontal for Row, vertical for Column).
    scroll_offset: i32,
    /// Total content extent along the scroll axis (pixels).
    content_extent: i32,
    /// Whether this window is currently registered as a Win32 app-bar.
    is_appbar: bool,
    /// Modeless options window, if currently open.
    options_window: Option<HWND>,
    /// Whether the mouse cursor is currently inside the main window.
    mouse_inside: bool,
    /// Instance profile name from `--profile <name>` CLI argument.
    profile_name: Option<String>,
}

struct ToolbarStatus {
    hidden_count: usize,
    refresh_label: String,
    always_on_top: bool,
    animate_transitions: bool,
    filters_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WindowMenuAction {
    HideApp,
    ToggleAspectRatio,
    ToggleHideOnSelect,
    CreateTagFromApp,
    ToggleTag(String),
}

/// Parse `--profile <name>` from the command-line arguments.
fn parse_profile_from_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut index = 1;
    while index < args.len() {
        if args[index] == "--profile" && index + 1 < args.len() {
            let name = args[index + 1].trim().to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }
        index += 1;
    }
    None
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
    let profile = parse_profile_from_args();
    tracing::info!(profile = ?profile, "Panopticon starting");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::Relaxed);
    }

    let icons = AppIcons::new().unwrap_or_else(|error| {
        tracing::error!(%error, "custom app icon generation failed; falling back to system icon");
        AppIcons::fallback_system()
    });
    let settings = AppSettings::load_or_default(profile.as_deref()).unwrap_or_else(|error| {
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
        scroll_offset: 0,
        content_extent: 0,
        is_appbar: false,
        options_window: None,
        mouse_inside: false,
        profile_name: profile,
    });

    let state_ptr = Box::into_raw(state);
    let hwnd = match create_main_window(state_ptr) {
        Ok(hwnd) => hwnd,
        Err(err) => {
            tracing::error!("Failed to create main window: {}", err);
            // Reclaim the leaked box to avoid memory leak if we gracefully exited,
            // though process exit handles it anyway.
            let _ = unsafe { Box::from_raw(state_ptr) };
            std::process::exit(1);
        }
    };

    // SAFETY: state pointer was stored in `GWLP_USERDATA` during `WM_NCCREATE`.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            apply_window_appearance(hwnd, &state.settings);
            apply_topmost_mode(state.hwnd, state.settings.always_on_top);
            match TrayIcon::add(hwnd, state.icons.small) {
                Ok(tray_icon) => state.tray_icon = Some(tray_icon),
                Err(error) => tracing::error!(%error, "failed to initialise tray icon"),
            }
            // Register app-bar and position the window if a dock edge is set.
            if state.settings.dock_edge.is_some() {
                apply_dock_mode(state);
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
fn create_main_window(state_ptr: *mut AppState) -> std::result::Result<HWND, &'static str> {
    // SAFETY: `state_ptr` points to a boxed `AppState` allocated in `main`
    // and lives until `WM_NCDESTROY` reclaims it.
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|_| "GetModuleHandleW failed")?;
        let hinstance = windows::Win32::Foundation::HINSTANCE(instance.0);
        let icons = &(*state_ptr).icons;
        let profile = &(*state_ptr).profile_name;
        let settings = &(*state_ptr).settings;

        // Unique class name per profile so multiple instances coexist.
        let class_name_owned: Vec<u16> = match profile {
            Some(name) => format!("PanopticonClass_{name}")
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect(),
            None => "PanopticonClass\0".encode_utf16().collect(),
        };
        let class_name = PCWSTR(class_name_owned.as_ptr());

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

        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err("RegisterClassExW failed");
        }

        // Window title includes profile name for multi-instance identification.
        let title_text = match profile {
            Some(name) => format!("Panopticon [{name}]"),
            None => "Panopticon — Window Viewer".to_owned(),
        };
        let title_wide: Vec<u16> = title_text
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // Dock mode uses a borderless popup; floating mode uses the standard frame.
        let (style, width, height) = if settings.dock_edge.is_some() {
            let w = settings.fixed_width.unwrap_or(300) as i32;
            let h = settings.fixed_height.unwrap_or(600) as i32;
            (WS_POPUP | WS_VISIBLE, w, h)
        } else {
            let w = settings.fixed_width.map_or(1320, |v| v as i32);
            let h = settings.fixed_height.map_or(840, |v| v as i32);
            (WS_OVERLAPPEDWINDOW | WS_VISIBLE, w, h)
        };

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            PCWSTR(title_wide.as_ptr()),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            None,
            None,
            hinstance,
            Some(state_ptr.cast::<c_void>()),
        )
        .map_err(|_| "CreateWindowExW failed")?;

        Ok(hwnd)
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
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if taskbar_msg != 0 && msg == taskbar_msg {
        if let Some(state) = app_from_hwnd(hwnd) {
            if let Some(tray) = state.tray_icon.as_mut() {
                tray.readd(state.icons.small);
            }
        }
        return LRESULT(0);
    }

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
            } else if wparam.0 == TIMER_HOVER_VISIBILITY {
                update_mouse_presence(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as i32;
            handle_scroll(hwnd, delta);
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
        appbar_cb if appbar_cb == WM_APPBAR_CALLBACK => {
            if wparam.0 as u32 == ABN_POSCHANGED {
                if let Some(state) = app_from_hwnd(hwnd) {
                    if state.is_appbar {
                        reposition_appbar(state);
                    }
                }
            }
            LRESULT(0)
        }
        options_apply if options_apply == WM_OPTIONS_APPLY => {
            let payload = lparam.0 as *mut OptionsSubmit;
            if !payload.is_null() {
                let payload = Box::from_raw(payload);
                apply_settings_snapshot(hwnd, payload.settings);
            }
            LRESULT(0)
        }
        options_closed if options_closed == WM_OPTIONS_CLOSED => {
            if let Some(state) = app_from_hwnd(hwnd) {
                state.options_window = None;
            }
            LRESULT(0)
        }
        tag_created if tag_created == WM_TAG_CREATED => {
            let payload = lparam.0 as *mut TagCreateSubmit;
            if !payload.is_null() {
                let payload = Box::from_raw(payload);
                apply_tag_creation(
                    hwnd,
                    &payload.app_id,
                    &payload.display_name,
                    &payload.tag_name,
                    &payload.color_hex,
                );
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = app_from_hwnd(hwnd) {
                if let Some(options_hwnd) = state.options_window.take() {
                    let _ = DestroyWindow(options_hwnd);
                }
                if state.is_appbar {
                    unregister_appbar(hwnd);
                    state.is_appbar = false;
                }
                state.windows.clear();
                if let Some(tray_icon) = state.tray_icon.as_mut() {
                    tray_icon.remove();
                }
            }
            let _ = KillTimer(hwnd, TIMER_ANIMATION);
            let _ = KillTimer(hwnd, TIMER_HOVER_VISIBILITY);
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
        VK_1 => set_layout(hwnd, LayoutType::Grid, "keyboard"),
        VK_2 => set_layout(hwnd, LayoutType::Mosaic, "keyboard"),
        VK_3 => set_layout(hwnd, LayoutType::Bento, "keyboard"),
        VK_4 => set_layout(hwnd, LayoutType::Fibonacci, "keyboard"),
        VK_5 => set_layout(hwnd, LayoutType::Columns, "keyboard"),
        VK_6 => set_layout(hwnd, LayoutType::Row, "keyboard"),
        VK_7 => set_layout(hwnd, LayoutType::Column, "keyboard"),
        VK_A => {
            update_settings(hwnd, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        VK_H => {
            update_settings(hwnd, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        VK_I => {
            update_settings(hwnd, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        VK_O => open_or_focus_options_window(hwnd),
        VK_P => {
            update_settings(hwnd, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            unsafe {
                if let Some(state) = app_from_hwnd(hwnd) {
                    apply_topmost_mode(hwnd, state.settings.always_on_top);
                }
            }
        }
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

#[allow(clippy::too_many_lines)]
fn handle_tray_action(hwnd: HWND, action: TrayAction) {
    match action {
        TrayAction::Toggle => toggle_window_visibility(hwnd),
        TrayAction::Refresh => refresh_and_repaint(hwnd),
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
        TrayAction::SetMonitorFilter(filter) => {
            update_settings(hwnd, |settings| {
                settings.set_monitor_filter(filter.as_deref());
            });
            refresh_and_repaint(hwnd);
        }
        TrayAction::SetTagFilter(filter) => {
            update_settings(hwnd, |settings| {
                settings.set_tag_filter(filter.as_deref());
            });
            refresh_and_repaint(hwnd);
        }
        TrayAction::SetAppFilter(filter) => {
            update_settings(hwnd, |settings| {
                settings.set_app_filter(filter.as_deref());
            });
            refresh_and_repaint(hwnd);
        }
        TrayAction::RestoreHidden(app_id) => {
            update_settings(hwnd, |settings| {
                let _ = settings.restore_hidden_app(&app_id);
            });
            refresh_and_repaint(hwnd);
        }
        TrayAction::RestoreAllHidden => {
            update_settings(hwnd, |settings| {
                let _ = settings.restore_all_hidden_apps();
            });
            refresh_and_repaint(hwnd);
        }
        TrayAction::SetDockEdge(edge) => {
            // SAFETY: state lives on the current UI thread.
            unsafe {
                if let Some(state) = app_from_hwnd(hwnd) {
                    // Unregister existing app-bar.
                    if state.is_appbar {
                        unregister_appbar(hwnd);
                        state.is_appbar = false;
                    }
                    state.settings.dock_edge = edge;
                    state.settings = state.settings.normalized();
                    if let Err(error) = state.settings.save(state.profile_name.as_deref()) {
                        tracing::error!(%error, "failed to persist settings");
                    }
                    if edge.is_some() {
                        apply_dock_mode(state);
                    } else {
                        // Restore floating window style.
                        let _ = SetWindowLongPtrW(
                            hwnd,
                            GWL_STYLE,
                            (WS_OVERLAPPEDWINDOW | WS_VISIBLE).0 as isize,
                        );
                        let _ = SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                        apply_topmost_mode(hwnd, state.settings.always_on_top);
                    }
                }
            }
            refresh_and_repaint(hwnd);
        }
        TrayAction::ToggleToolbar => {
            update_settings(hwnd, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        TrayAction::OpenSettingsWindow => open_or_focus_options_window(hwnd),
        TrayAction::Exit => request_exit(hwnd),
    }
}

fn tray_menu_state(hwnd: HWND) -> TrayMenuState {
    // SAFETY: state lives on the current window thread.
    let state = unsafe { app_from_hwnd(hwnd) };
    if let Some(state) = state {
        let available_windows: Vec<WindowInfo> = enumerate_windows()
            .into_iter()
            .filter(|window| window.hwnd != hwnd)
            .collect();
        for window in &available_windows {
            state
                .settings
                .refresh_app_label(&window.app_id, &window.app_label());
        }

        TrayMenuState {
            window_visible: unsafe { IsWindowVisible(hwnd).as_bool() },
            minimize_to_tray: state.settings.minimize_to_tray,
            close_to_tray: state.settings.close_to_tray,
            refresh_interval_ms: state.settings.refresh_interval_ms,
            animate_transitions: state.settings.animate_transitions,
            preserve_aspect_ratio: state.settings.preserve_aspect_ratio,
            hide_on_select: state.settings.hide_on_select,
            always_on_top: state.settings.always_on_top,
            active_monitor_filter: state.settings.active_monitor_filter.clone(),
            available_monitors: collect_available_monitors(&available_windows),
            active_tag_filter: state.settings.active_tag_filter.clone(),
            available_tags: state.settings.known_tags(),
            active_app_filter: state.settings.active_app_filter.clone(),
            available_apps: collect_available_apps(&available_windows),
            hidden_apps: state.settings.hidden_app_entries(),
            dock_edge: state.settings.dock_edge,
            show_toolbar: state.settings.show_toolbar,
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
            active_monitor_filter: None,
            available_monitors: Vec::new(),
            active_tag_filter: None,
            available_tags: Vec::new(),
            active_app_filter: None,
            available_apps: Vec::new(),
            hidden_apps: Vec::new(),
            dock_edge: None,
            show_toolbar: true,
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
            if let Err(error) = state.settings.save(state.profile_name.as_deref()) {
                tracing::error!(%error, "failed to persist settings");
            }
        }
    }
}

fn refresh_and_repaint(hwnd: HWND) {
    let _ = refresh_windows(hwnd);
    recompute_layout(hwnd);
    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
    }
}

fn collect_available_monitors(windows: &[WindowInfo]) -> Vec<String> {
    let monitors: BTreeSet<String> = windows
        .iter()
        .map(|window| window.monitor_name.clone())
        .collect();
    monitors.into_iter().collect()
}

fn collect_available_apps(windows: &[WindowInfo]) -> Vec<AppSelectionEntry> {
    let mut app_map: HashMap<String, String> = HashMap::new();
    for window in windows {
        app_map
            .entry(window.app_id.clone())
            .or_insert_with(|| window.app_label());
    }

    let mut apps: Vec<AppSelectionEntry> = app_map
        .into_iter()
        .map(|(app_id, label)| AppSelectionEntry { app_id, label })
        .collect();
    apps.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then(left.app_id.cmp(&right.app_id))
    });
    apps
}

fn active_filter_summary(settings: &AppSettings) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(monitor) = &settings.active_monitor_filter {
        parts.push(format!("monitor:{monitor}"));
    }
    if let Some(group) = settings.active_group_filter_label() {
        parts.push(group);
    }

    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn cycle_layout(hwnd: HWND, source: &str) {
    // SAFETY: state lives on the current window thread.
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            state.current_layout = state.current_layout.next();
            state.settings.initial_layout = state.current_layout;
            if let Err(error) = state.settings.save(state.profile_name.as_deref()) {
                tracing::error!(%error, source = source, "failed to persist selected layout");
            }
            tracing::debug!(layout = ?state.current_layout, source = source, "layout switched");
        }
    }
}

fn set_layout(hwnd: HWND, layout: LayoutType, source: &str) {
    unsafe {
        if let Some(state) = app_from_hwnd(hwnd) {
            if state.current_layout == layout {
                return;
            }

            state.current_layout = layout;
            state.settings.initial_layout = layout;
            if let Err(error) = state.settings.save(state.profile_name.as_deref()) {
                tracing::error!(%error, source = source, "failed to persist selected layout");
            }
        }
    }

    recompute_layout(hwnd);
    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
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

        let background_color = state.settings.background_color_bgr();
        let accent_color = state.settings.active_tag_filter.as_deref().map_or(
            ACCENT_COLOR,
            |tag| state.settings.tag_color_bgr(tag),
        );
        let active_group_background = state.settings.active_tag_filter.as_deref().map(|tag| {
            blend_bgr(background_color, state.settings.tag_color_bgr(tag), 1, 4)
        });

        let bg_brush = CreateSolidBrush(COLORREF(background_color));
        let toolbar_brush = CreateSolidBrush(COLORREF(TB_COLOR));
        let panel_brush = CreateSolidBrush(COLORREF(PANEL_BG_COLOR));
        let border_brush = CreateSolidBrush(COLORREF(BORDER_COLOR));
        let footer_brush = CreateSolidBrush(COLORREF(TB_COLOR));
        let hover_brush = CreateSolidBrush(COLORREF(HOVER_BORDER_COLOR));
        let accent_brush = CreateSolidBrush(COLORREF(accent_color));

        FillRect(hdc, &client_rect, bg_brush);

        let toolbar_h = if state.settings.show_toolbar {
            TOOLBAR_HEIGHT
        } else {
            0
        };
        let footer_h = footer_height_for(&state.settings);
        let content_rect = RECT {
            left: client_rect.left,
            top: client_rect.top + toolbar_h,
            right: client_rect.right,
            bottom: client_rect.bottom,
        };

        if let Some(group_bg) = active_group_background {
            let group_brush = CreateSolidBrush(COLORREF(group_bg));
            FillRect(hdc, &content_rect, group_brush);
            let _ = DeleteObject(group_brush);
        }

        if state.settings.show_toolbar {
            let toolbar_rect = RECT {
                left: client_rect.left,
                top: client_rect.top,
                right: client_rect.right,
                bottom: client_rect.top + toolbar_h,
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
                    refresh_label: state.settings.refresh_interval_label(),
                    always_on_top: state.settings.always_on_top,
                    animate_transitions: state.settings.animate_transitions,
                    filters_label: active_filter_summary(&state.settings),
                },
            );
        }

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
                footer_h,
            );
        }

        paint_overlay_scrollbar(hdc, state, client_rect, accent_color);

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
    footer_height: i32,
) {
    let scroll_dir = state.current_layout.scroll_direction();
    let scroll_offset = state.scroll_offset;

    let mut client_rect = RECT::default();
    unsafe {
        let _ = GetClientRect(state.hwnd, &mut client_rect);
    }

    for (index, managed_window) in state.windows.iter().enumerate() {
        let outer_rect = apply_scroll_rect(managed_window.display_rect, scroll_offset, scroll_dir);

        // Skip windows entirely outside the visible client area.
        if !rects_overlap(outer_rect, client_rect) {
            continue;
        }

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

        if managed_window.thumbnail.is_none() {
            paint_thumbnail_placeholder(hdc, managed_window.info.hwnd, outer_rect, panel_brush, footer_height);
        }

        if footer_height > 0 {
            let footer_rect = RECT {
                left: outer_rect.left,
                top: outer_rect.bottom - footer_height,
                right: outer_rect.right,
                bottom: outer_rect.bottom,
            };
            // SAFETY: `hdc` and brushes are valid for the active paint pass.
            unsafe {
                FillRect(hdc, &footer_rect, footer_brush);
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
}

fn paint_thumbnail_placeholder(
    hdc: HDC,
    source_hwnd: HWND,
    card_rect: RECT,
    panel_brush: HBRUSH,
    footer_height: i32,
) {
    let placeholder_rect = preview_area_for_card(card_rect, footer_height);
    // SAFETY: `hdc` and brushes are valid for the active paint pass.
    unsafe {
        FillRect(hdc, &placeholder_rect, panel_brush);
    }
    draw_window_icon(hdc, source_hwnd, placeholder_rect, 32);

    if footer_height > 0 {
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
}

fn paint_toolbar(
    hdc: HDC,
    toolbar_rect: RECT,
    layout: LayoutType,
    window_count: usize,
    status: &ToolbarStatus,
) {
    let mut status_text = format!(
        "{}  ·  {} visibles  ·  {} ocultas  ·  {} refresh",
        layout.label(),
        window_count,
        status.hidden_count,
        status.refresh_label
    );
    if let Some(filters_label) = &status.filters_label {
        status_text.push_str("  ·  ");
        status_text.push_str(filters_label);
    }

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
    let helper_text = active_filter_summary(&state.settings).map_or_else(
        || {
            "Open or restore any desktop window. Panopticon will keep watching from the tray."
                .to_owned()
        },
        |filters| {
            format!(
                "No windows match the current filters ({filters}). Adjust them from the tray or restore more apps."
            )
        },
    );

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    let content_rect = RECT {
        left: client_rect.left,
        top: client_rect.top + toolbar_h,
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
        &helper_text,
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

    let active_monitor_filter = state.settings.active_monitor_filter.clone();
    let active_tag_filter = state.settings.active_tag_filter.clone();
    let active_app_filter = state.settings.active_app_filter.clone();
    let discovered: Vec<WindowInfo> = discovered_all
        .into_iter()
        .filter(|window| {
            active_monitor_filter
                .as_deref()
                .is_none_or(|monitor| window.monitor_name == monitor)
        })
        .filter(|window| {
            active_tag_filter
                .as_deref()
                .is_none_or(|tag| state.settings.app_has_tag(&window.app_id, tag))
        })
        .filter(|window| {
            active_app_filter
                .as_deref()
                .is_none_or(|app_id| window.app_id == app_id)
        })
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
            || fresh.class_name != managed_window.info.class_name
            || fresh.monitor_name != managed_window.info.monitor_name;
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

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    let content_area = RECT {
        left: client_rect.left,
        top: client_rect.top + toolbar_h,
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

    // Track content extent for scrolling in Row / Column modes.
    let scroll_dir = state.current_layout.scroll_direction();
    state.content_extent = match scroll_dir {
        ScrollDirection::Horizontal => rects.iter().map(|r| r.right).max().unwrap_or(0),
        ScrollDirection::Vertical => rects.iter().map(|r| r.bottom).max().unwrap_or(0),
        ScrollDirection::None => 0,
    };
    // max_scroll = (total content end) − (visible area end).
    let max_scroll = match scroll_dir {
        ScrollDirection::Horizontal => (state.content_extent - content_area.right).max(0),
        ScrollDirection::Vertical => (state.content_extent - content_area.bottom).max(0),
        ScrollDirection::None => 0,
    };
    state.scroll_offset = state.scroll_offset.clamp(0, max_scroll);

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
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    if y < toolbar_h {
        cycle_layout(hwnd, "toolbar");
        recompute_layout(hwnd);
        unsafe {
            let _ = InvalidateRect(hwnd, None, true);
        }
        return;
    }

    let (sx, sy) = scroll_adjusted_point(x, y, state);

    let hit = state
        .windows
        .iter()
        .find(|managed_window| point_in_rect(managed_window.display_rect, sx, sy))
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
    // SAFETY: state lives on the current window thread.
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    if y < toolbar_h {
        return;
    }

    let (sx, sy) = scroll_adjusted_point(x, y, state);

    let hit = state
        .windows
        .iter()
        .find(|managed_window| point_in_rect(managed_window.display_rect, sx, sy))
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

    let just_entered = !state.mouse_inside;
    if just_entered {
        state.mouse_inside = true;
    }
    unsafe {
        SetTimer(hwnd, TIMER_HOVER_VISIBILITY, 120, None);
    }

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    let new_hover = if y >= toolbar_h {
        let (sx, sy) = scroll_adjusted_point(x, y, state);
        state
            .windows
            .iter()
            .position(|managed_window| point_in_rect(managed_window.display_rect, sx, sy))
    } else {
        None
    };

    if new_hover != state.hover_index || (just_entered && max_scroll_for_state(state) > 0) {
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
            refresh_and_repaint(hwnd);
        }
        WindowMenuAction::ToggleAspectRatio => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_app_preserve_aspect_ratio(&info.app_id, &info.app_label());
            });
            recompute_layout(hwnd);
            unsafe {
                let _ = InvalidateRect(hwnd, None, true);
            }
        }
        WindowMenuAction::ToggleHideOnSelect => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_app_hide_on_select(&info.app_id, &info.app_label());
            });
        }
        WindowMenuAction::CreateTagFromApp => {
            open_create_tag_dialog(hwnd, info);
        }
        WindowMenuAction::ToggleTag(tag) => {
            update_settings(hwnd, |settings| {
                let _ = settings.toggle_app_tag(&info.app_id, &info.app_label(), &tag);
            });
            refresh_and_repaint(hwnd);
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
        let create_tag_label = wide("Create custom tag…");
        let tags_title = wide("Assign existing tags");
        let known_tags = app_from_hwnd(hwnd)
            .map(|state| state.settings.known_tags())
            .unwrap_or_default();
        let current_tags: HashSet<String> = app_from_hwnd(hwnd)
            .map(|state| state.settings.tags_for(&info.app_id).into_iter().collect())
            .unwrap_or_default();
        let mut tag_labels: Vec<Vec<u16>> = Vec::with_capacity(known_tags.len());
        let mut tag_actions: Vec<(u16, String)> = Vec::with_capacity(known_tags.len());

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
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_WINDOW_CREATE_TAG_FROM_APP as usize,
            PCWSTR(create_tag_label.as_ptr()),
        );

        if !known_tags.is_empty() {
            let tags_menu = CreatePopupMenu().ok()?;
            for (index, tag) in known_tags.iter().enumerate() {
                let Some(command_id) = CMD_WINDOW_TAG_BASE.checked_add(index as u16) else {
                    break;
                };

                tag_labels.push(wide(tag));
                if let Some(label) = tag_labels.last() {
                    let _ = AppendMenuW(
                        tags_menu,
                        MF_STRING | checked_menu_flag(current_tags.contains(tag)),
                        command_id as usize,
                        PCWSTR(label.as_ptr()),
                    );
                }
                tag_actions.push((command_id, tag.clone()));
            }

            let _ = AppendMenuW(
                menu,
                windows::Win32::UI::WindowsAndMessaging::MF_POPUP,
                tags_menu.0 as usize,
                PCWSTR(tags_title.as_ptr()),
            );
        }

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
            CMD_WINDOW_CREATE_TAG_FROM_APP => Some(WindowMenuAction::CreateTagFromApp),
            dynamic => tag_actions.into_iter().find_map(|(command_id, tag)| {
                (dynamic == command_id).then_some(WindowMenuAction::ToggleTag(tag))
            }),
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
    let scroll_dir = state.current_layout.scroll_direction();
    let scroll_offset = state.scroll_offset;
    let footer_height = footer_height_for(settings);

    let mut client_rect = RECT::default();
    unsafe {
        let _ = GetClientRect(state.hwnd, &mut client_rect);
    }

    for managed_window in &mut state.windows {
        let preserve_aspect_ratio = settings.preserve_aspect_ratio_for(&managed_window.info.app_id);
        let _ = ensure_thumbnail(owner_hwnd, managed_window);
        if let Some(thumbnail) = managed_window.thumbnail.as_ref() {
            let scrolled =
                apply_scroll_rect(managed_window.display_rect, scroll_offset, scroll_dir);
            let visible = rects_overlap(scrolled, client_rect);
            let destination = preview_destination_rect(
                scrolled,
                managed_window.source_size,
                preserve_aspect_ratio,
                footer_height,
            );
            if thumbnail.update(destination, visible).is_err() {
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
    footer_height: i32,
) -> RECT {
    let preview_area = preview_area_for_card(card_rect, footer_height);
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

fn preview_area_for_card(card_rect: RECT, footer_height: i32) -> RECT {
    RECT {
        left: card_rect.left + 1,
        top: card_rect.top + THUMBNAIL_ACCENT_HEIGHT,
        right: card_rect.right - 1,
        bottom: card_rect.bottom - footer_height,
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

fn footer_height_for(settings: &AppSettings) -> i32 {
    if settings.show_window_info {
        THUMBNAIL_FOOTER_HEIGHT
    } else {
        0
    }
}

fn open_or_focus_options_window(hwnd: HWND) {
    unsafe {
        let Some(state) = app_from_hwnd(hwnd) else {
            return;
        };

        if let Some(existing) = state.options_window {
            if IsWindow(existing).as_bool() {
                let _ = ShowWindow(existing, SW_SHOW);
                let _ = SetForegroundWindow(existing);
                return;
            }
        }

        match open_options_window(hwnd, &state.settings) {
            Ok(options_hwnd) => state.options_window = Some(options_hwnd),
            Err(error) => tracing::error!(%error, "failed to open settings window"),
        }
    }
}

fn open_create_tag_dialog(hwnd: HWND, info: &WindowInfo) {
    let suggested_name = suggested_tag_name(&info.app_label());
    let suggested_color = unsafe {
        app_from_hwnd(hwnd)
            .map(|state| state.settings.tag_color_hex(&suggested_name))
            .unwrap_or_else(|| String::from("D29A5C"))
    };

    if let Err(error) = open_tag_dialog(
        hwnd,
        &info.app_id,
        &info.app_label(),
        &suggested_name,
        &suggested_color,
    ) {
        tracing::error!(%error, "failed to open tag dialog");
    }
}

fn suggested_tag_name(label: &str) -> String {
    let lowered = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();
    lowered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn apply_tag_creation(
    hwnd: HWND,
    app_id: &str,
    display_name: &str,
    tag_name: &str,
    color_hex: &str,
) {
    update_settings(hwnd, |settings| {
        let _ = settings.assign_tag_with_color(app_id, display_name, tag_name, color_hex);
    });
    refresh_and_repaint(hwnd);
}

fn apply_settings_snapshot(hwnd: HWND, new_settings: AppSettings) {
    unsafe {
        let Some(state) = app_from_hwnd(hwnd) else {
            return;
        };

        let old_dock_edge = state.settings.dock_edge;
        state.settings = new_settings.normalized();
        state.current_layout = state.settings.initial_layout;

        if let Err(error) = state.settings.save(state.profile_name.as_deref()) {
            tracing::error!(%error, "failed to persist settings");
        }

        apply_window_appearance(hwnd, &state.settings);
        apply_topmost_mode(hwnd, state.settings.always_on_top);

        if old_dock_edge != state.settings.dock_edge {
            if state.is_appbar {
                unregister_appbar(hwnd);
                state.is_appbar = false;
            }

            if state.settings.dock_edge.is_some() {
                apply_dock_mode(state);
            } else {
                restore_floating_window_style(hwnd);
                apply_fixed_window_size(hwnd, &state.settings);
            }
        } else if state.is_appbar {
            reposition_appbar(state);
        } else {
            restore_floating_window_style(hwnd);
            apply_fixed_window_size(hwnd, &state.settings);
        }
    }

    reset_refresh_timer(hwnd);
    recompute_layout(hwnd);
    unsafe {
        let _ = InvalidateRect(hwnd, None, true);
    }
}

fn restore_floating_window_style(hwnd: HWND) {
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_OVERLAPPEDWINDOW | WS_VISIBLE).0 as isize);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

fn apply_fixed_window_size(hwnd: HWND, settings: &AppSettings) {
    let Some(width) = settings.fixed_width.map(|value| value as i32) else {
        if settings.fixed_height.is_none() {
            return;
        }
        let mut window_rect = RECT::default();
        unsafe {
            let _ = GetWindowRect(hwnd, &mut window_rect);
        }
        let current_width = window_rect.right - window_rect.left;
        let height = settings.fixed_height.map_or(window_rect.bottom - window_rect.top, |value| value as i32);
        unsafe {
            let _ = SetWindowPos(hwnd, None, 0, 0, current_width, height, SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER);
        }
        return;
    };

    let mut window_rect = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut window_rect);
    }
    let height = settings
        .fixed_height
        .map_or(window_rect.bottom - window_rect.top, |value| value as i32);
    unsafe {
        let _ = SetWindowPos(hwnd, None, 0, 0, width, height, SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER);
    }
}

fn apply_window_appearance(hwnd: HWND, settings: &AppSettings) {
    let dark_mode: i32 = 1;
    let corner_preference = DWMWCP_ROUND;
    let backdrop_type = if settings.use_system_backdrop {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const c_void,
            mem::size_of_val(&dark_mode) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner_preference as *const _ as *const c_void,
            mem::size_of_val(&corner_preference) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop_type as *const _ as *const c_void,
            mem::size_of_val(&backdrop_type) as u32,
        );
    }
}

fn update_mouse_presence(hwnd: HWND) {
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let was_inside = state.mouse_inside;
    let mut cursor = POINT::default();
    let mut window_rect = RECT::default();
    let is_inside = unsafe {
        let _ = GetCursorPos(&mut cursor);
        let _ = GetWindowRect(hwnd, &mut window_rect);
        point_in_rect(window_rect, cursor.x, cursor.y)
    };

    state.mouse_inside = is_inside;
    if !is_inside {
        state.hover_index = None;
        unsafe {
            let _ = KillTimer(hwnd, TIMER_HOVER_VISIBILITY);
        }
    }

    if was_inside != is_inside && max_scroll_for_state(state) > 0 {
        unsafe {
            let _ = InvalidateRect(hwnd, None, false);
        }
    }
}

fn max_scroll_for_state(state: &AppState) -> i32 {
    let mut client_rect = RECT::default();
    unsafe {
        let _ = GetClientRect(state.hwnd, &mut client_rect);
    }

    match state.current_layout.scroll_direction() {
        ScrollDirection::Horizontal => (state.content_extent - client_rect.right).max(0),
        ScrollDirection::Vertical => (state.content_extent - client_rect.bottom).max(0),
        ScrollDirection::None => 0,
    }
}

fn paint_overlay_scrollbar(hdc: HDC, state: &AppState, client_rect: RECT, accent_color: u32) {
    let Some(thumb_rect) = scrollbar_thumb_rect(state, client_rect) else {
        return;
    };

    unsafe {
        let thumb_brush = CreateSolidBrush(COLORREF(blend_bgr(accent_color, 0x00FF_FFFF, 1, 5)));
        let track_brush = CreateSolidBrush(COLORREF(blend_bgr(state.settings.background_color_bgr(), 0x00FF_FFFF, 1, 12)));
        FillRect(hdc, &expand_rect(thumb_rect, 2), track_brush);
        FillRect(hdc, &thumb_rect, thumb_brush);
        let _ = DeleteObject(thumb_brush);
        let _ = DeleteObject(track_brush);
    }
}

fn scrollbar_thumb_rect(state: &AppState, client_rect: RECT) -> Option<RECT> {
    if !state.mouse_inside {
        return None;
    }

    let max_scroll = max_scroll_for_state(state);
    if max_scroll <= 0 {
        return None;
    }

    let toolbar_h = if state.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };
    let content_rect = RECT {
        left: client_rect.left,
        top: client_rect.top + toolbar_h,
        right: client_rect.right,
        bottom: client_rect.bottom,
    };

    match state.current_layout.scroll_direction() {
        ScrollDirection::Horizontal => {
            let track_len = (content_rect.right - content_rect.left - SCROLLBAR_MARGIN * 2).max(0);
            let visible = (content_rect.right - content_rect.left).max(1);
            let content = state.content_extent.max(visible);
            let thumb_len = ((visible * track_len) / content).clamp(SCROLLBAR_MIN_THUMB, track_len.max(SCROLLBAR_MIN_THUMB));
            let travel = (track_len - thumb_len).max(0);
            let thumb_offset = if max_scroll == 0 { 0 } else { (state.scroll_offset * travel) / max_scroll };
            Some(RECT {
                left: content_rect.left + SCROLLBAR_MARGIN + thumb_offset,
                top: content_rect.bottom - SCROLLBAR_MARGIN - SCROLLBAR_THICKNESS,
                right: content_rect.left + SCROLLBAR_MARGIN + thumb_offset + thumb_len,
                bottom: content_rect.bottom - SCROLLBAR_MARGIN,
            })
        }
        ScrollDirection::Vertical => {
            let track_len = (content_rect.bottom - content_rect.top - SCROLLBAR_MARGIN * 2).max(0);
            let visible = (content_rect.bottom - content_rect.top).max(1);
            let content = state.content_extent.max(visible);
            let thumb_len = ((visible * track_len) / content).clamp(SCROLLBAR_MIN_THUMB, track_len.max(SCROLLBAR_MIN_THUMB));
            let travel = (track_len - thumb_len).max(0);
            let thumb_offset = if max_scroll == 0 { 0 } else { (state.scroll_offset * travel) / max_scroll };
            Some(RECT {
                left: content_rect.right - SCROLLBAR_MARGIN - SCROLLBAR_THICKNESS,
                top: content_rect.top + SCROLLBAR_MARGIN + thumb_offset,
                right: content_rect.right - SCROLLBAR_MARGIN,
                bottom: content_rect.top + SCROLLBAR_MARGIN + thumb_offset + thumb_len,
            })
        }
        ScrollDirection::None => None,
    }
}

fn expand_rect(rect: RECT, amount: i32) -> RECT {
    RECT {
        left: rect.left - amount,
        top: rect.top - amount,
        right: rect.right + amount,
        bottom: rect.bottom + amount,
    }
}

fn blend_bgr(base: u32, overlay: u32, numerator: u32, denominator: u32) -> u32 {
    let [base_b, base_g, base_r] = color_channels_bgr(base);
    let [overlay_b, overlay_g, overlay_r] = color_channels_bgr(overlay);
    compose_bgr(
        blend_channel(base_b, overlay_b, numerator, denominator),
        blend_channel(base_g, overlay_g, numerator, denominator),
        blend_channel(base_r, overlay_r, numerator, denominator),
    )
}

const fn color_channels_bgr(color: u32) -> [u8; 3] {
    [
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ]
}

const fn compose_bgr(blue: u8, green: u8, red: u8) -> u32 {
    ((blue as u32) << 16) | ((green as u32) << 8) | red as u32
}

const fn blend_channel(base: u8, overlay: u8, numerator: u32, denominator: u32) -> u8 {
    (((base as u32 * (denominator - numerator)) + (overlay as u32 * numerator)) / denominator)
        as u8
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

// ───────────────────────── Scroll helpers ─────────────────────────

/// Handle `WM_MOUSEWHEEL`: adjust `scroll_offset` and update thumbnails.
fn handle_scroll(hwnd: HWND, wheel_delta: i32) {
    let Some(state) = (unsafe { app_from_hwnd(hwnd) }) else {
        return;
    };

    let scroll_dir = state.current_layout.scroll_direction();
    if scroll_dir == ScrollDirection::None {
        return;
    }

    let max_scroll = max_scroll_for_state(state);
    if max_scroll <= 0 {
        return;
    }

    // Negative delta → scroll toward higher offsets (down / right).
    let step = -(wheel_delta * SCROLL_STEP / WHEEL_DELTA);
    let next_offset = (state.scroll_offset + step).clamp(0, max_scroll);
    if next_offset == state.scroll_offset {
        return;
    }

    state.scroll_offset = next_offset;

    update_window_previews(state);
    unsafe {
        let _ = InvalidateRect(hwnd, None, false);
    }
}

/// Offset a [`RECT`] by the current scroll amount along the active axis.
fn apply_scroll_rect(rect: RECT, offset: i32, direction: ScrollDirection) -> RECT {
    match direction {
        ScrollDirection::Horizontal => RECT {
            left: rect.left - offset,
            top: rect.top,
            right: rect.right - offset,
            bottom: rect.bottom,
        },
        ScrollDirection::Vertical => RECT {
            left: rect.left,
            top: rect.top - offset,
            right: rect.right,
            bottom: rect.bottom - offset,
        },
        ScrollDirection::None => rect,
    }
}

/// Convert a mouse point from client coordinates to content coordinates by
/// reversing the current scroll offset.
fn scroll_adjusted_point(x: i32, y: i32, state: &AppState) -> (i32, i32) {
    match state.current_layout.scroll_direction() {
        ScrollDirection::Horizontal => (x + state.scroll_offset, y),
        ScrollDirection::Vertical => (x, y + state.scroll_offset),
        ScrollDirection::None => (x, y),
    }
}

/// Check whether two [`RECT`]s overlap (share any area).
fn rects_overlap(a: RECT, b: RECT) -> bool {
    a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top
}

// ───────────────────────── App-bar (dock) helpers ─────────────────────────

/// Switch the window to a borderless popup and register it as a Win32 app-bar.
fn apply_dock_mode(state: &mut AppState) {
    let hwnd = state.hwnd;

    // SAFETY: changing the window style on the UI thread.
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_VISIBLE).0 as isize);
    }

    if register_appbar(hwnd) {
        state.is_appbar = true;
        reposition_appbar(state);
    } else {
        tracing::warn!("failed to register app-bar");
    }
}

/// Register this window as a desktop app-bar via `SHAppBarMessage(ABM_NEW)`.
fn register_appbar(hwnd: HWND) -> bool {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: WM_APPBAR_CALLBACK,
        ..Default::default()
    };
    // SAFETY: valid APPBARDATA with a live HWND.
    unsafe { SHAppBarMessage(ABM_NEW, &mut abd) != 0 }
}

/// Remove the app-bar registration for `hwnd`.
fn unregister_appbar(hwnd: HWND) {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    // SAFETY: valid APPBARDATA; no-op if not currently registered.
    unsafe {
        let _ = SHAppBarMessage(ABM_REMOVE, &mut abd);
    }
}

/// Query the system for the correct position and reserve screen space.
#[allow(clippy::similar_names)]
fn reposition_appbar(state: &mut AppState) {
    let Some(edge) = state.settings.dock_edge else {
        return;
    };

    let hwnd = state.hwnd;
    let monitor_rect = get_monitor_rect(hwnd);
    let abe = dock_edge_to_abe(edge);
    let thickness = match edge {
        DockEdge::Left | DockEdge::Right => state.settings.fixed_width.unwrap_or(300) as i32,
        DockEdge::Top | DockEdge::Bottom => state.settings.fixed_height.unwrap_or(200) as i32,
    };

    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uEdge: abe,
        rc: monitor_rect,
        ..Default::default()
    };

    // SAFETY: synchronous shell messages on the UI thread.
    unsafe {
        // Let the system adjust the proposed rect for conflicts.
        let _ = SHAppBarMessage(ABM_QUERYPOS, &mut abd);

        // Set the opposite edge to enforce our configured thickness.
        match edge {
            DockEdge::Left => abd.rc.right = abd.rc.left + thickness,
            DockEdge::Right => abd.rc.left = abd.rc.right - thickness,
            DockEdge::Top => abd.rc.bottom = abd.rc.top + thickness,
            DockEdge::Bottom => abd.rc.top = abd.rc.bottom - thickness,
        }

        let _ = SHAppBarMessage(ABM_SETPOS, &mut abd);

        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            abd.rc.left,
            abd.rc.top,
            abd.rc.right - abd.rc.left,
            abd.rc.bottom - abd.rc.top,
            SWP_NOACTIVATE,
        );
    }
}

/// Map a [`DockEdge`] to the Win32 `ABE_*` constant.
const fn dock_edge_to_abe(edge: DockEdge) -> u32 {
    match edge {
        DockEdge::Left => ABE_LEFT,
        DockEdge::Right => ABE_RIGHT,
        DockEdge::Top => ABE_TOP,
        DockEdge::Bottom => ABE_BOTTOM,
    }
}

/// Return the full monitor rectangle for the monitor that contains `hwnd`.
fn get_monitor_rect(hwnd: HWND) -> RECT {
    // SAFETY: read-only query against the display system.
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            info.rcMonitor
        } else {
            RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
            }
        }
    }
}
