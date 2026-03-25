#![windows_subsystem = "windows"]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::wildcard_imports
)]

//! Binary entry point for Panopticon — Slint UI with DWM thumbnail overlays.

mod app;

use app::settings_ui::{apply_settings_window_changes, populate_settings_window};
use app::tray::{handle_tray_message, AppIcons, TrayAction, TrayIcon, TrayMenuState, WM_TRAYICON};
use app::window_menu::{show_window_context_menu, WindowMenuAction};
use panopticon::constants::{
    ANIMATION_DURATION_MS, THUMBNAIL_ACCENT_HEIGHT, THUMBNAIL_FOOTER_HEIGHT, TOOLBAR_HEIGHT,
};
use panopticon::layout::{
    apply_separator_drag, compute_layout_custom, default_ratios, AspectHint, LayoutType,
    Separator, ScrollDirection,
};
use panopticon::settings::{AppSelectionEntry, AppSettings, DockEdge};
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::ffi::c_void;
use std::mem;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    CloseRequestResponse, ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel,
};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmSetWindowAttribute, DWMSBT_MAINWINDOW, DWMSBT_NONE,
    DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, ABN_POSCHANGED, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::*;

slint::include_modules!();

// ───────────────────────── Constants ─────────────────────────

/// Callback message posted by the shell when the app-bar needs repositioning.
const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

// ───────────────────────── Thread-local subclass state ─────────────────────────

thread_local! {
    static ORIGINAL_WNDPROC: Cell<isize> = const { Cell::new(0) };
    static UI_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = const { RefCell::new(None) };
    static UI_WINDOW: RefCell<Option<slint::Weak<MainWindow>>> = const { RefCell::new(None) };
    static PENDING_ACTIONS: RefCell<Vec<PendingAction>> = const { RefCell::new(Vec::new()) };
    static SETTINGS_WIN: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
    static TAG_DIALOG_WIN: RefCell<Option<TagDialogWindow>> = const { RefCell::new(None) };
}

// ───────────────────────── Types ─────────────────────────

enum PendingAction {
    Tray(TrayAction),
    Reposition,
    HideToTray,
    Refresh,
    Exit,
}

/// Tracks an in-progress separator drag.
#[derive(Debug, Clone)]
struct DragState {
    /// Separator index (maps to the handle `index` field in Slint).
    separator_index: usize,
    /// Whether the separator is horizontal (drag vertically).
    horizontal: bool,
    /// Ratio-array index of the separator.
    ratio_index: usize,
    /// Total extent of the axis at drag start (width or height of content area).
    axis_extent: f64,
}

/// A window tracked by Panopticon, including its DWM thumbnail handle.
struct ManagedWindow {
    info: WindowInfo,
    thumbnail: Option<Thumbnail>,
    target_rect: RECT,
    display_rect: RECT,
    animation_from_rect: RECT,
    source_size: SIZE,
}

/// Root application state shared via `Rc<RefCell<…>>`.
struct AppState {
    hwnd: HWND,
    windows: Vec<ManagedWindow>,
    current_layout: LayoutType,
    hover_index: Option<usize>,
    tray_icon: Option<TrayIcon>,
    icons: AppIcons,
    settings: AppSettings,
    animation_started_at: Option<Instant>,
    content_extent: i32,
    is_appbar: bool,
    mouse_inside: bool,
    profile_name: Option<String>,
    last_size: (i32, i32),
    /// Cached separators from the last layout computation.
    separators: Vec<Separator>,
    /// Active drag state: separator index being dragged.
    drag_separator: Option<DragState>,
}

// ───────────────────────── Entry Point ─────────────────────────

#[allow(clippy::too_many_lines)]
fn main() {
    let _log_guard = panopticon::logging::init().ok();
    let profile = parse_profile_from_args();
    tracing::info!(profile = ?profile, "Panopticon starting (Slint UI)");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::Relaxed);
    }

    let icons = AppIcons::new().unwrap_or_else(|error| {
        tracing::error!(%error, "icon generation failed; falling back");
        AppIcons::fallback_system()
    });
    let settings = AppSettings::load_or_default(profile.as_deref()).unwrap_or_else(|error| {
        tracing::error!(%error, "settings load failed; using defaults");
        AppSettings::default()
    });

    let main_window = MainWindow::new().unwrap();

    // Apply initial property values from settings.
    sync_settings_to_ui(&main_window, &settings);

    let state = Rc::new(RefCell::new(AppState {
        hwnd: HWND::default(),
        windows: Vec::new(),
        current_layout: settings.initial_layout,
        hover_index: None,
        tray_icon: None,
        icons,
        settings,
        animation_started_at: None,
        content_extent: 0,
        is_appbar: false,
        mouse_inside: false,
        profile_name: profile,
        last_size: (0, 0),
        separators: Vec::new(),
        drag_separator: None,
    }));

    // Show the window so the native HWND exists on next event-loop iteration.
    main_window.show().unwrap();

    // Slint callbacks (don't need HWND — they use state internally).
    setup_callbacks(&main_window, &state);

    // Handle close button.
    main_window.window().on_close_requested({
        let state = state.clone();
        move || {
            let s = state.borrow();
            if s.settings.close_to_tray {
                drop(s);
                release_all_thumbnails(&state);
                CloseRequestResponse::HideWindow
            } else {
                drop(s);
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Exit));
                CloseRequestResponse::KeepWindowShown
            }
        }
    });

    // ── Deferred native-HWND initialisation (runs once the event loop is live) ──
    let init_timer = Timer::default();
    init_timer.start(TimerMode::SingleShot, Duration::from_millis(0), {
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            let Some(win) = weak.upgrade() else { return };
            let Some(hwnd) = get_hwnd(win.window()) else {
                tracing::error!("HWND unavailable after event-loop start");
                return;
            };
            state.borrow_mut().hwnd = hwnd;

            // DWM appearance.
            apply_window_appearance(hwnd, &state.borrow().settings);
            apply_topmost_mode(hwnd, state.borrow().settings.always_on_top);

            // System tray.
            {
                let mut s = state.borrow_mut();
                match TrayIcon::add(hwnd, s.icons.small) {
                    Ok(tray) => s.tray_icon = Some(tray),
                    Err(error) => tracing::error!(%error, "tray icon registration failed"),
                }
            }

            // Subclass the Slint HWND to intercept tray / appbar / minimize messages.
            setup_subclass(hwnd, &state, &win);

            // Initial refresh + layout.
            refresh_windows(&state);
            recompute_and_update_ui(&state, &win);

            // App-bar registration (if dock edge is set).
            if state.borrow().settings.dock_edge.is_some() {
                let mut s = state.borrow_mut();
                apply_dock_mode(&mut s);
            }
        }
    });

    // ── Timers ──────────────────────────────────────

    // Fast UI timer: size polling, animation, DWM thumbnail sync, action drain.
    let ui_timer = Timer::default();
    ui_timer.start(TimerMode::Repeated, Duration::from_millis(16), {
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            let Some(win) = weak.upgrade() else { return };
            // Skip until HWND is initialised by the init timer.
            if state.borrow().hwnd.0.is_null() {
                return;
            }

            // Drain pending actions.
            let actions: Vec<PendingAction> =
                PENDING_ACTIONS.with(|q| q.borrow_mut().drain(..).collect());
            for action in actions {
                handle_pending_action(&state, &win, action);
            }

            // Check for window-size changes.
            let phys_size = win.window().size();
            let scale = win.window().scale_factor();
            let logical_w = (phys_size.width as f32 / scale).round() as i32;
            let logical_h = (phys_size.height as f32 / scale).round() as i32;
            let needs_relayout = {
                let s = state.borrow();
                logical_w != s.last_size.0 || logical_h != s.last_size.1
            };
            if needs_relayout {
                state.borrow_mut().last_size = (logical_w, logical_h);
                recompute_and_update_ui(&state, &win);
            }

            // Advance animations.
            advance_animation(&state, &win);

            // Re-sync DWM thumbnails (scroll changes, animation frames, etc.).
            update_dwm_thumbnails(&state, &win);
        }
    });

    // Slow refresh timer: window enumeration.
    let refresh_timer = Timer::default();
    refresh_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(state.borrow().settings.refresh_interval_ms as u64),
        {
            let state = state.clone();
            let weak = main_window.as_weak();
            move || {
                let visible = UI_STATE.with(|s| {
                    s.borrow().as_ref().is_some_and(|rc| {
                        rc.try_borrow()
                            .is_ok_and(|s| unsafe { IsWindowVisible(s.hwnd).as_bool() })
                    })
                });
                if !visible {
                    return;
                }
                if refresh_windows(&state) {
                    if let Some(win) = weak.upgrade() {
                        recompute_and_update_ui(&state, &win);
                    }
                }
            }
        },
    );

    tracing::info!("entering Slint event loop");
    slint::run_event_loop_until_quit().unwrap();
    let hwnd = state.borrow().hwnd;
    if !hwnd.0.is_null() {
        teardown_subclass(hwnd);
    }
    tracing::info!("Panopticon exiting");
}

// ───────────────────────── HWND Extraction ─────────────────────────

fn get_hwnd(window: &slint::Window) -> Option<HWND> {
    let slint_handle = window.window_handle();
    let raw = slint_handle.window_handle().ok()?;
    match raw.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
        _ => None,
    }
}

// ───────────────────────── HWND Subclass ─────────────────────────

fn setup_subclass(hwnd: HWND, state: &Rc<RefCell<AppState>>, main_window: &MainWindow) {
    UI_STATE.with(|s| *s.borrow_mut() = Some(state.clone()));
    UI_WINDOW.with(|w| *w.borrow_mut() = Some(main_window.as_weak()));

    let original = unsafe { GetWindowLongPtrW(hwnd, GWL_WNDPROC) };
    ORIGINAL_WNDPROC.with(|p| p.set(original));

    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_WNDPROC, subclass_proc as usize as isize);
    }
}

fn teardown_subclass(hwnd: HWND) {
    let original = ORIGINAL_WNDPROC.with(Cell::get);
    if original != 0 {
        unsafe {
            let _ = SetWindowLongPtrW(hwnd, GWL_WNDPROC, original);
        }
    }
    UI_STATE.with(|s| *s.borrow_mut() = None);
    UI_WINDOW.with(|w| *w.borrow_mut() = None);
}

#[inline]
#[allow(clippy::missing_transmute_annotations)]
fn forward_to_original(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let original = ORIGINAL_WNDPROC.with(Cell::get);
    // SAFETY: `original` points to winit's WndProc set during window creation.
    unsafe { CallWindowProcW(mem::transmute(original), hwnd, msg, wparam, lparam) }
}

#[expect(
    clippy::too_many_lines,
    reason = "El dispatcher Win32 se mantiene lineal para auditar mensajes y efectos colaterales en un solo lugar"
)]
unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // TaskbarCreated: re-register tray icon.
    let taskbar_msg = TASKBAR_CREATED_MSG.load(Ordering::Relaxed);
    if taskbar_msg != 0 && msg == taskbar_msg {
        UI_STATE.with(|s| {
            if let Some(rc) = s.borrow().as_ref() {
                if let Ok(mut st) = rc.try_borrow_mut() {
                    let small = st.icons.small;
                    if let Some(tray) = st.tray_icon.as_mut() {
                        tray.readd(small);
                    }
                }
            }
        });
        return forward_to_original(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_TRAYICON => {
            let mouse_msg = lparam.0 as u32;
            if mouse_msg == WM_LBUTTONUP {
                PENDING_ACTIONS
                    .with(|q| q.borrow_mut().push(PendingAction::Tray(TrayAction::Toggle)));
            } else if mouse_msg == WM_RBUTTONUP {
                // Build the menu state snapshot (borrows & releases state).
                let menu_state = UI_STATE.with(|s| {
                    s.borrow().as_ref().and_then(|rc| {
                        rc.try_borrow_mut()
                            .ok()
                            .map(|mut st| build_tray_menu_state(&mut st))
                    })
                });
                // `TrackPopupMenu` blocks; no borrows are held during the call.
                if let Some(menu_state) = menu_state {
                    if let Some(action) = handle_tray_message(hwnd, lparam, &menu_state) {
                        PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Tray(action)));
                    }
                }
            }
            LRESULT(0)
        }
        WM_APPBAR_CALLBACK => {
            if wparam.0 as u32 == ABN_POSCHANGED {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Reposition));
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let should_hide = UI_STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .and_then(|rc| rc.try_borrow().ok().map(|st| st.settings.close_to_tray))
                    .unwrap_or(false)
            });
            if should_hide {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::HideToTray));
            } else {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Exit));
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if wparam.0 as u32 == 1
            /* SIZE_MINIMIZED */
            {
                let should_hide = UI_STATE.with(|s| {
                    s.borrow()
                        .as_ref()
                        .and_then(|rc| rc.try_borrow().ok().map(|st| st.settings.minimize_to_tray))
                        .unwrap_or(false)
                });
                if should_hide {
                    PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::HideToTray));
                }
            }
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_SHOWWINDOW => {
            if wparam.0 != 0 {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Refresh));
            } else {
                // Release DWM thumbnails when the window is hidden.
                UI_STATE.with(|s| {
                    if let Some(rc) = s.borrow().as_ref() {
                        if let Ok(mut st) = rc.try_borrow_mut() {
                            for mw in &mut st.windows {
                                mw.thumbnail = None;
                            }
                        }
                    }
                });
            }
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        _ => forward_to_original(hwnd, msg, wparam, lparam),
    }
}

// ───────────────────────── Slint Callbacks ─────────────────────────

fn setup_callbacks(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) {
    main_window.on_thumbnail_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            handle_thumbnail_click(&state, &weak, index as usize);
        }
    });

    main_window.on_thumbnail_right_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, _x, _y| {
            handle_thumbnail_right_click(&state, &weak, index as usize);
        }
    });

    main_window.on_thumbnail_hovered({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            let mut s = state.borrow_mut();
            let new_index = Some(index as usize);
            if s.hover_index != new_index {
                s.hover_index = new_index;
                s.mouse_inside = true;
                drop(s);
                if let Some(win) = weak.upgrade() {
                    update_hover_in_model(&state, &win);
                }
            }
        }
    });

    main_window.on_toolbar_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            cycle_layout(&state);
            if let Some(win) = weak.upgrade() {
                recompute_and_update_ui(&state, &win);
            }
        }
    });

    main_window.on_resize_drag_started({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            handle_resize_drag_start(&state, &weak, index as usize, x as f64, y as f64);
        }
    });

    main_window.on_resize_drag_moved({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            handle_resize_drag_move(&state, &weak, index as usize, x as f64, y as f64);
        }
    });

    main_window.on_resize_drag_ended({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |_index| {
            handle_resize_drag_end(&state, &weak);
        }
    });

    main_window.on_key_pressed({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |key_text| handle_key(&state, &weak, &key_text)
    });
}

// ───────────────────────── Window Refresh ─────────────────────────

fn refresh_windows(state: &Rc<RefCell<AppState>>) -> bool {
    let mut s = state.borrow_mut();
    let host_hwnd = s.hwnd;
    if host_hwnd.0.is_null() {
        return false;
    }
    let host_visible = unsafe { IsWindowVisible(host_hwnd).as_bool() };

    let discovered_all: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|w| w.hwnd != host_hwnd)
        .collect();

    for w in &discovered_all {
        s.settings.refresh_app_label(&w.app_id, &w.app_label());
    }

    let monitor_f = s.settings.active_monitor_filter.clone();
    let tag_f = s.settings.active_tag_filter.clone();
    let app_f = s.settings.active_app_filter.clone();

    let discovered: Vec<WindowInfo> = discovered_all
        .into_iter()
        .filter(|w| monitor_f.as_deref().is_none_or(|m| w.monitor_name == m))
        .filter(|w| {
            tag_f
                .as_deref()
                .is_none_or(|t| s.settings.app_has_tag(&w.app_id, t))
        })
        .filter(|w| app_f.as_deref().is_none_or(|a| w.app_id == a))
        .filter(|w| !s.settings.is_hidden(&w.app_id))
        .collect();

    let discovered_map: HashMap<isize, WindowInfo> = discovered
        .iter()
        .cloned()
        .map(|w| (w.hwnd.0 as isize, w))
        .collect();
    let discovered_hwnds: HashSet<isize> = discovered_map.keys().copied().collect();

    let prev_len = s.windows.len();
    s.windows
        .retain(|mw| discovered_hwnds.contains(&(mw.info.hwnd.0 as isize)));
    let mut changed = s.windows.len() != prev_len;

    for mw in &mut s.windows {
        if let Some(fresh) = discovered_map.get(&(mw.info.hwnd.0 as isize)) {
            let metadata_changed = fresh.title != mw.info.title
                || fresh.app_id != mw.info.app_id
                || fresh.process_name != mw.info.process_name
                || fresh.class_name != mw.info.class_name
                || fresh.monitor_name != mw.info.monitor_name;
            if metadata_changed {
                mw.info = fresh.clone();
                changed = true;
            }
            if host_visible {
                if ensure_thumbnail(host_hwnd, mw) {
                    changed = true;
                }
                if let Some(thumb) = mw.thumbnail.as_ref() {
                    let fresh_size = query_source_size(thumb.handle());
                    if fresh_size.cx != mw.source_size.cx || fresh_size.cy != mw.source_size.cy {
                        mw.source_size = fresh_size;
                        changed = true;
                    }
                }
            }
        }
    }

    let existing: HashSet<isize> = s.windows.iter().map(|mw| mw.info.hwnd.0 as isize).collect();

    for info in discovered {
        if existing.contains(&(info.hwnd.0 as isize)) {
            continue;
        }
        let mut mw = ManagedWindow {
            info,
            thumbnail: None,
            target_rect: RECT::default(),
            display_rect: RECT::default(),
            animation_from_rect: RECT::default(),
            source_size: SIZE { cx: 800, cy: 600 },
        };
        if host_visible {
            let _ = ensure_thumbnail(host_hwnd, &mut mw);
        }
        s.windows.push(mw);
        changed = true;
    }

    changed
}

// ───────────────────────── Layout + UI Sync ─────────────────────────

fn recompute_and_update_ui(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let mut s = state.borrow_mut();
    if s.windows.is_empty() {
        s.animation_started_at = None;
        sync_settings_to_ui(win, &s.settings);
        drop(s);
        sync_model_to_slint(state, win);
        return;
    }

    let phys = win.window().size();
    let scale = win.window().scale_factor();
    let logical_w = (phys.width as f32 / scale).round() as i32;
    let logical_h = (phys.height as f32 / scale).round() as i32;
    let toolbar_h = if s.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };

    // Layout engine works in logical pixels, content-area relative (top = 0).
    let content_area = RECT {
        left: 0,
        top: 0,
        right: logical_w,
        bottom: (logical_h - toolbar_h).max(1),
    };

    let aspects: Vec<AspectHint> = s
        .windows
        .iter()
        .map(|mw| AspectHint {
            width: f64::from(mw.source_size.cx),
            height: f64::from(mw.source_size.cy),
        })
        .collect();

    let custom = s.settings.layout_custom(s.current_layout).cloned();
    let result = compute_layout_custom(
        s.current_layout,
        content_area,
        s.windows.len(),
        &aspects,
        custom.as_ref(),
    );
    let rects = result.rects;
    s.separators = result.separators;

    // Content extent for scrolling.
    let scroll_dir = s.current_layout.scroll_direction();
    s.content_extent = match scroll_dir {
        ScrollDirection::Horizontal => rects.iter().map(|r| r.right).max().unwrap_or(0),
        ScrollDirection::Vertical => rects.iter().map(|r| r.bottom).max().unwrap_or(0),
        ScrollDirection::None => 0,
    };

    let can_animate = s.settings.animate_transitions
        && !s.hwnd.0.is_null()
        && unsafe { IsWindowVisible(s.hwnd).as_bool() }
        && s.windows.iter().any(|mw| rect_has_area(mw.display_rect));
    let mut animation_needed = false;

    for (i, mw) in s.windows.iter_mut().enumerate() {
        if let Some(&rect) = rects.get(i) {
            let prev = if rect_has_area(mw.display_rect) {
                mw.display_rect
            } else {
                rect
            };
            mw.animation_from_rect = prev;
            mw.target_rect = rect;
            if can_animate && prev != rect {
                animation_needed = true;
            } else {
                mw.display_rect = rect;
            }
        }
    }

    if animation_needed {
        s.animation_started_at = Some(Instant::now());
    } else {
        s.animation_started_at = None;
        for mw in &mut s.windows {
            mw.display_rect = mw.target_rect;
        }
    }

    // Update Slint properties.
    let scroll_h = scroll_dir == ScrollDirection::Horizontal;
    let scroll_v = scroll_dir == ScrollDirection::Vertical;
    win.set_scroll_horizontal(scroll_h);
    win.set_scroll_vertical(scroll_v);
    win.set_content_width(s.content_extent as f32);
    win.set_content_height(s.content_extent as f32);

    sync_settings_to_ui(win, &s.settings);

    drop(s);
    sync_model_to_slint(state, win);
}

fn sync_settings_to_ui(win: &MainWindow, settings: &AppSettings) {
    win.set_show_toolbar(settings.show_toolbar);
    win.set_show_window_info(settings.show_window_info);
    win.set_is_always_on_top(settings.always_on_top);
    win.set_animate_transitions(settings.animate_transitions);
    win.set_refresh_label(SharedString::from(settings.refresh_interval_label()));
    win.set_filters_label(SharedString::from(
        active_filter_summary(settings).unwrap_or_default(),
    ));
}

fn sync_model_to_slint(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let s = state.borrow();
    let accent = tag_accent_color(&s.settings);
    let show_footer = s.settings.show_window_info;

    let data: Vec<ThumbnailData> = s
        .windows
        .iter()
        .enumerate()
        .map(|(i, mw)| ThumbnailData {
            x: mw.display_rect.left as f32,
            y: mw.display_rect.top as f32,
            width: (mw.display_rect.right - mw.display_rect.left) as f32,
            height: (mw.display_rect.bottom - mw.display_rect.top) as f32,
            title: SharedString::from(truncate_title(&mw.info.title)),
            app_label: SharedString::from(mw.info.app_label()),
            is_hovered: s.hover_index == Some(i),
            accent_color: accent,
            show_footer,
        })
        .collect();

    // Build resize handle data from cached separators.
    let handle_thickness: f32 = 10.0;
    let handles: Vec<ResizeHandleData> = s
        .separators
        .iter()
        .enumerate()
        .map(|(idx, sep)| {
            if sep.horizontal {
                ResizeHandleData {
                    x: sep.extent_start as f32,
                    y: sep.position as f32 - handle_thickness / 2.0,
                    width: (sep.extent_end - sep.extent_start) as f32,
                    height: handle_thickness,
                    horizontal: true,
                    index: idx as i32,
                }
            } else {
                ResizeHandleData {
                    x: sep.position as f32 - handle_thickness / 2.0,
                    y: sep.extent_start as f32,
                    width: handle_thickness,
                    height: (sep.extent_end - sep.extent_start) as f32,
                    horizontal: false,
                    index: idx as i32,
                }
            }
        })
        .collect();

    let active_drag = s
        .drag_separator
        .as_ref()
        .map_or(-1, |d| d.separator_index as i32);

    win.set_layout_label(SharedString::from(s.current_layout.label()));
    win.set_window_count(s.windows.len() as i32);
    win.set_hidden_count(s.settings.hidden_app_entries().len() as i32);

    drop(s);
    win.set_thumbnails(ModelRc::new(VecModel::from(data)));
    win.set_resize_handles(ModelRc::new(VecModel::from(handles)));
    win.set_active_drag_index(active_drag);
}

fn update_hover_in_model(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    sync_model_to_slint(state, win);
}

// ───────────────────────── DWM Thumbnails ─────────────────────────

fn update_dwm_thumbnails(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Ok(mut s) = state.try_borrow_mut() else {
        return;
    };
    if s.hwnd.0.is_null() || !unsafe { IsWindowVisible(s.hwnd).as_bool() } {
        return;
    }

    let scale = win.window().scale_factor();
    let phys = win.window().size();
    let toolbar_h = if s.settings.show_toolbar {
        TOOLBAR_HEIGHT as f32
    } else {
        0.0
    };
    let footer_h = if s.settings.show_window_info {
        THUMBNAIL_FOOTER_HEIGHT
    } else {
        0
    };
    let viewport_x = win.get_viewport_x();
    let viewport_y = win.get_viewport_y();

    let dest_hwnd = s.hwnd;
    let preserve_flags: Vec<bool> = s
        .windows
        .iter()
        .map(|mw| s.settings.preserve_aspect_ratio_for(&mw.info.app_id))
        .collect();
    for (i, mw) in s.windows.iter_mut().enumerate() {
        let preserve = preserve_flags[i];
        let _ = ensure_thumbnail(dest_hwnd, mw);
        if let Some(thumb) = mw.thumbnail.as_ref() {
            let dest = compute_dwm_rect(
                &mw.display_rect,
                mw.source_size,
                preserve,
                footer_h,
                toolbar_h,
                viewport_x,
                viewport_y,
                scale,
            );
            let visible = dest.left < phys.width as i32
                && dest.right > 0
                && dest.top < phys.height as i32
                && dest.bottom > 0;
            if thumb.update(dest, visible).is_err() {
                tracing::warn!(title = %mw.info.title, "thumbnail update failed — dropping");
                mw.thumbnail = None;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::many_single_char_names)]
fn compute_dwm_rect(
    card_rect: &RECT,
    source_size: SIZE,
    preserve_aspect: bool,
    footer_h: i32,
    toolbar_h: f32,
    viewport_x: f32,
    viewport_y: f32,
    scale: f32,
) -> RECT {
    let l = card_rect.left as f32 + 1.0;
    let t = card_rect.top as f32 + THUMBNAIL_ACCENT_HEIGHT as f32;
    let r = card_rect.right as f32 - 1.0;
    let b = card_rect.bottom as f32 - footer_h as f32;

    let (fl, ft, fr, fb) = if preserve_aspect && source_size.cx > 0 && source_size.cy > 0 {
        let aw = r - l;
        let ah = b - t;
        let wr = aw / source_size.cx as f32;
        let hr = ah / source_size.cy as f32;
        let s = wr.min(hr);
        let rw = source_size.cx as f32 * s;
        let rh = source_size.cy as f32 * s;
        (
            l + (aw - rw) / 2.0,
            t + (ah - rh) / 2.0,
            l + (aw - rw) / 2.0 + rw,
            t + (ah - rh) / 2.0 + rh,
        )
    } else {
        (l, t, r, b)
    };

    RECT {
        left: ((fl + viewport_x) * scale).round() as i32,
        top: ((ft + toolbar_h + viewport_y) * scale).round() as i32,
        right: ((fr + viewport_x) * scale).round() as i32,
        bottom: ((fb + toolbar_h + viewport_y) * scale).round() as i32,
    }
}

fn ensure_thumbnail(owner: HWND, mw: &mut ManagedWindow) -> bool {
    if mw.thumbnail.is_some() {
        return false;
    }
    if let Ok(thumb) = Thumbnail::register(owner, mw.info.hwnd) {
        mw.source_size = query_source_size(thumb.handle());
        mw.thumbnail = Some(thumb);
        true
    } else {
        false
    }
}

fn release_all_thumbnails(state: &Rc<RefCell<AppState>>) {
    if let Ok(mut s) = state.try_borrow_mut() {
        for mw in &mut s.windows {
            mw.thumbnail = None;
        }
    }
}

fn query_source_size(handle: isize) -> SIZE {
    let mut size = unsafe { DwmQueryThumbnailSourceSize(handle).unwrap_or_default() };
    if size.cx == 0 {
        size.cx = 800;
    }
    if size.cy == 0 {
        size.cy = 600;
    }
    size
}

// ───────────────────────── Click / Hover ─────────────────────────

fn handle_thumbnail_click(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    index: usize,
) {
    let s = state.borrow();
    let Some(mw) = s.windows.get(index) else {
        return;
    };
    let info = mw.info.clone();
    let hide_on_select = s.settings.hide_on_select_for(&info.app_id);
    drop(s);

    tracing::info!(title = %info.title, app_id = %info.app_id, "activating window");
    activate_window(info.hwnd);

    if hide_on_select {
        if let Some(win) = weak.upgrade() {
            release_all_thumbnails(state);
            win.hide().ok();
        }
    }
}

fn handle_thumbnail_right_click(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    index: usize,
) {
    let s = state.borrow();
    if s.hwnd.0.is_null() {
        return;
    }
    let Some(mw) = s.windows.get(index) else {
        return;
    };
    let info = mw.info.clone();
    let preserve = s.settings.preserve_aspect_ratio_for(&info.app_id);
    let hide_on = s.settings.hide_on_select_for(&info.app_id);
    let known_tags = s.settings.known_tags();
    let current_tags: HashSet<String> = s.settings.tags_for(&info.app_id).into_iter().collect();
    let hwnd = s.hwnd;
    drop(s);

    if let Some(action) = show_window_context_menu(
        hwnd,
        preserve,
        hide_on,
        &known_tags,
        &current_tags,
    ) {
        handle_window_menu_action(state, weak, &info, action);
    }
}

fn activate_window(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

// ───────────────────────── Context Menu ─────────────────────────

fn handle_window_menu_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
    action: WindowMenuAction,
) {
    let mut needs_window_refresh = false;
    let mut needs_ui_refresh = false;

    match action {
        WindowMenuAction::HideApp => {
            update_settings(state, |settings| {
                let _ = settings.toggle_hidden(&info.app_id, &info.app_label());
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
        WindowMenuAction::ToggleAspectRatio => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_preserve_aspect_ratio(&info.app_id, &info.app_label());
            });
            needs_ui_refresh = true;
        }
        WindowMenuAction::ToggleHideOnSelect => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_hide_on_select(&info.app_id, &info.app_label());
            });
        }
        WindowMenuAction::CreateTagFromApp => {
            open_create_tag_dialog(state, weak, info);
        }
        WindowMenuAction::ToggleTag(tag) => {
            update_settings(state, |settings| {
                let _ = settings.toggle_app_tag(&info.app_id, &info.app_label(), &tag);
            });
            needs_window_refresh = true;
            needs_ui_refresh = true;
        }
    }

    if needs_window_refresh {
        let _ = refresh_windows(state);
    }

    if needs_ui_refresh {
        refresh_ui(state, weak);
    }
}

fn open_create_tag_dialog(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    info: &WindowInfo,
) {
    let already_open = TAG_DIALOG_WIN.with(|dialog| {
        let guard = dialog.borrow();
        if let Some(existing) = guard.as_ref() {
            existing.show().ok();
            true
        } else {
            false
        }
    });
    if already_open {
        return;
    }

    let suggested_name = suggested_tag_name(&info.app_label());
    let suggested_color = state.borrow().settings.tag_color_hex(&suggested_name);

    let dialog = match TagDialogWindow::new() {
        Ok(dialog) => dialog,
        Err(error) => {
            tracing::error!(%error, app_id = %info.app_id, "failed to create tag dialog");
            return;
        }
    };

    dialog.set_app_label(SharedString::from(info.app_label()));
    dialog.set_tag_name(SharedString::from(suggested_name));
    dialog.set_color_index(tag_color_index(&suggested_color));

    dialog.on_create({
        let state = state.clone();
        let weak = weak.clone();
        let app_id = info.app_id.clone();
        let display_name = info.app_label();
        move || {
            TAG_DIALOG_WIN.with(|dialog_cell| {
                let guard = dialog_cell.borrow();
                let Some(dialog) = guard.as_ref() else { return };
                let tag_name = dialog.get_tag_name().to_string();
                let color_hex = tag_color_hex(dialog.get_color_index());
                drop(guard);

                apply_tag_creation(&state, &weak, &app_id, &display_name, &tag_name, &color_hex);
                close_tag_dialog_window();
            });
        }
    });

    dialog.on_closed(|| {
        close_tag_dialog_window();
    });

    dialog.window().on_close_requested(|| {
        close_tag_dialog_window();
        CloseRequestResponse::HideWindow
    });

    if let Err(error) = dialog.show() {
        tracing::error!(%error, app_id = %info.app_id, "failed to show tag dialog");
        return;
    }

    if let Some(dialog_hwnd) = get_hwnd(dialog.window()) {
        apply_window_appearance(dialog_hwnd, &state.borrow().settings);
    }

    TAG_DIALOG_WIN.with(|dialog_cell| *dialog_cell.borrow_mut() = Some(dialog));
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
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    app_id: &str,
    display_name: &str,
    tag_name: &str,
    color_hex: &str,
) {
    update_settings(state, |settings| {
        let _ = settings.assign_tag_with_color(app_id, display_name, tag_name, color_hex);
    });
    let _ = refresh_windows(state);
    refresh_ui(state, weak);
}

fn close_tag_dialog_window() {
    let taken = TAG_DIALOG_WIN.with(|dialog| dialog.borrow_mut().take());
    if let Some(dialog) = taken {
        dialog.hide().ok();
    }
}

fn tag_color_index(color_hex: &str) -> i32 {
    match color_hex.to_ascii_uppercase().as_str() {
        "5CA9FF" => 1,
        "3CCF91" => 2,
        "FF6B8A" => 3,
        "9B7BFF" => 4,
        "F4B740" => 5,
        _ => 0,
    }
}

fn tag_color_hex(index: i32) -> String {
    match index {
        1 => "5CA9FF",
        2 => "3CCF91",
        3 => "FF6B8A",
        4 => "9B7BFF",
        5 => "F4B740",
        _ => "D29A5C",
    }
    .to_owned()
}

// ───────────────────────── Keyboard ─────────────────────────

fn handle_key(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>, key: &str) -> bool {
    match key {
        "1" => {
            set_layout(state, weak, LayoutType::Grid);
            true
        }
        "2" => {
            set_layout(state, weak, LayoutType::Mosaic);
            true
        }
        "3" => {
            set_layout(state, weak, LayoutType::Bento);
            true
        }
        "4" => {
            set_layout(state, weak, LayoutType::Fibonacci);
            true
        }
        "5" => {
            set_layout(state, weak, LayoutType::Columns);
            true
        }
        "6" => {
            set_layout(state, weak, LayoutType::Row);
            true
        }
        "7" => {
            set_layout(state, weak, LayoutType::Column);
            true
        }
        "0" => {
            // Reset custom layout ratios for current layout
            reset_layout_custom(state);
            refresh_ui(state, weak);
            true
        }
        "a" | "A" => {
            update_settings(state, |s| {
                s.animate_transitions = !s.animate_transitions;
            });
            refresh_ui(state, weak);
            true
        }
        "h" | "H" => {
            update_settings(state, |s| {
                s.show_toolbar = !s.show_toolbar;
            });
            refresh_ui(state, weak);
            true
        }
        "i" | "I" => {
            update_settings(state, |s| {
                s.show_window_info = !s.show_window_info;
            });
            refresh_ui(state, weak);
            true
        }
        "o" | "O" => {
            open_settings_window(state, weak);
            true
        }
        "p" | "P" => {
            update_settings(state, |s| {
                s.always_on_top = !s.always_on_top;
            });
            let s = state.borrow();
            apply_topmost_mode(s.hwnd, s.settings.always_on_top);
            drop(s);
            refresh_ui(state, weak);
            true
        }
        "r" | "R" => {
            if refresh_windows(state) {
                refresh_ui(state, weak);
            }
            true
        }
        "\t" => {
            cycle_layout(state);
            refresh_ui(state, weak);
            true
        }
        "\u{001B}" => {
            // Escape
            PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Exit));
            true
        }
        _ => false,
    }
}

// ───────────────────────── Resize Drag ─────────────────────────

fn handle_resize_drag_start(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    separator_index: usize,
    _x: f64,
    _y: f64,
) {
    let mut s = state.borrow_mut();
    let Some(sep) = s.separators.get(separator_index).copied() else {
        return;
    };

    let phys = weak.upgrade().map(|w| w.window().size());
    let scale = weak.upgrade().map_or(1.0, |w| w.window().scale_factor());
    let toolbar_h = if s.settings.show_toolbar {
        TOOLBAR_HEIGHT
    } else {
        0
    };
    let logical_w = phys.map_or(1280, |p| (p.width as f32 / scale).round() as i32);
    let logical_h = phys.map_or(720, |p| (p.height as f32 / scale).round() as i32) - toolbar_h;

    let axis_extent = if sep.horizontal {
        logical_h as f64
    } else {
        logical_w as f64
    };

    s.drag_separator = Some(DragState {
        separator_index,
        horizontal: sep.horizontal,
        ratio_index: sep.ratio_index,
        axis_extent,
    });
}

fn handle_resize_drag_move(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    separator_index: usize,
    x: f64,
    y: f64,
) {
    let drag = {
        let s = state.borrow();
        match s.drag_separator.as_ref() {
            Some(d) if d.separator_index == separator_index => d.clone(),
            _ => return,
        }
    };

    // The mouse position is relative to the resize handle element.
    // Calculate delta from the center of the handle.
    let handle_center = 5.0; // half of handle_thickness
    let mouse_offset = if drag.horizontal {
        y - handle_center
    } else {
        x - handle_center
    };

    if drag.axis_extent <= 0.0 {
        return;
    }
    let delta_frac = mouse_offset / drag.axis_extent;

    // Ensure we have custom ratios for the current layout.
    let mut s = state.borrow_mut();
    let layout = s.current_layout;
    ensure_custom_ratios(&mut s, layout);

    let min_frac = 0.05;
    if let Some(custom) = s.settings.layout_customizations.get_mut(layout.label()) {
        let ratios = if drag.horizontal {
            &mut custom.row_ratios
        } else {
            &mut custom.col_ratios
        };
        if drag.ratio_index + 1 < ratios.len() {
            apply_separator_drag(ratios, drag.ratio_index, delta_frac, min_frac);
        }
    }

    // Save and recompute
    s.settings = s.settings.normalized();
    let _ = s.settings.save(s.profile_name.as_deref());
    drop(s);

    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
    }
}

fn handle_resize_drag_end(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
) {
    let mut s = state.borrow_mut();
    s.drag_separator = None;
    let _ = s.settings.save(s.profile_name.as_deref());
    drop(s);

    if let Some(win) = weak.upgrade() {
        sync_model_to_slint(state, &win);
    }
}

fn reset_layout_custom(state: &Rc<RefCell<AppState>>) {
    let mut s = state.borrow_mut();
    let layout = s.current_layout;
    s.settings.clear_layout_custom(layout);
    s.settings = s.settings.normalized();
    let _ = s.settings.save(s.profile_name.as_deref());
}

/// Ensure that custom ratios exist for the current layout; if absent,
/// seed them from the default equal distribution so dragging has a
/// starting point.
fn ensure_custom_ratios(s: &mut AppState, layout: LayoutType) {
    let count = s.windows.len();
    if count == 0 {
        return;
    }

    let entry = s
        .settings
        .layout_customizations
        .entry(layout.label().to_owned())
        .or_default();

    match layout {
        LayoutType::Grid => {
            let cols = (count as f64).sqrt().ceil() as usize;
            let rows = count.div_ceil(cols);
            if entry.col_ratios.len() != cols {
                entry.col_ratios = default_ratios(cols);
            }
            if entry.row_ratios.len() != rows {
                entry.row_ratios = default_ratios(rows);
            }
        }
        LayoutType::Mosaic => {
            let cols = (count as f64).sqrt().ceil() as usize;
            let rows_count = count.div_ceil(cols);
            if entry.row_ratios.len() != rows_count {
                entry.row_ratios = default_ratios(rows_count);
            }
        }
        LayoutType::Bento => {
            if entry.col_ratios.is_empty() {
                entry.col_ratios = vec![0.6];
            }
            let side_count = count.saturating_sub(1);
            if side_count > 0 && entry.row_ratios.len() != side_count {
                entry.row_ratios = default_ratios(side_count);
            }
        }
        LayoutType::Columns => {
            let num_cols = ((count as f64).sqrt().ceil() as usize).clamp(2, 5);
            if entry.col_ratios.len() != num_cols {
                entry.col_ratios = default_ratios(num_cols);
            }
        }
        LayoutType::Row => {
            if entry.col_ratios.len() != count {
                entry.col_ratios = default_ratios(count);
            }
        }
        LayoutType::Column => {
            if entry.row_ratios.len() != count {
                entry.row_ratios = default_ratios(count);
            }
        }
        LayoutType::Fibonacci => {}
    }
}

// ───────────────────────── Tray ─────────────────────────

fn build_tray_menu_state(state: &mut AppState) -> TrayMenuState {
    let available_windows: Vec<WindowInfo> = enumerate_windows()
        .into_iter()
        .filter(|w| w.hwnd != state.hwnd)
        .collect();
    for w in &available_windows {
        state.settings.refresh_app_label(&w.app_id, &w.app_label());
    }

    TrayMenuState {
        window_visible: unsafe { IsWindowVisible(state.hwnd).as_bool() },
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
}

#[allow(clippy::too_many_lines)]
fn handle_tray_action(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    action: TrayAction,
) {
    match action {
        TrayAction::Toggle => toggle_visibility(state, weak),
        TrayAction::Refresh => {
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::NextLayout => {
            cycle_layout(state);
            refresh_ui(state, weak);
        }
        TrayAction::ToggleMinimizeToTray => {
            update_settings(state, |s| {
                s.minimize_to_tray = !s.minimize_to_tray;
            });
        }
        TrayAction::ToggleCloseToTray => {
            update_settings(state, |s| {
                s.close_to_tray = !s.close_to_tray;
            });
        }
        TrayAction::CycleRefreshInterval => {
            update_settings(state, AppSettings::cycle_refresh_interval);
            refresh_ui(state, weak);
        }
        TrayAction::ToggleAnimateTransitions => {
            update_settings(state, |s| {
                s.animate_transitions = !s.animate_transitions;
            });
        }
        TrayAction::ToggleDefaultAspectRatio => {
            update_settings(state, |s| {
                s.preserve_aspect_ratio = !s.preserve_aspect_ratio;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleDefaultHideOnSelect => {
            update_settings(state, |s| {
                s.hide_on_select = !s.hide_on_select;
            });
        }
        TrayAction::ToggleAlwaysOnTop => {
            update_settings(state, |s| {
                s.always_on_top = !s.always_on_top;
            });
            let s = state.borrow();
            apply_topmost_mode(s.hwnd, s.settings.always_on_top);
        }
        TrayAction::SetMonitorFilter(filter) => {
            update_settings(state, |s| {
                s.set_monitor_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetTagFilter(filter) => {
            update_settings(state, |s| {
                s.set_tag_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetAppFilter(filter) => {
            update_settings(state, |s| {
                s.set_app_filter(filter.as_deref());
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::RestoreHidden(app_id) => {
            update_settings(state, |s| {
                let _ = s.restore_hidden_app(&app_id);
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::RestoreAllHidden => {
            update_settings(state, |s| {
                let _ = s.restore_all_hidden_apps();
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::SetDockEdge(edge) => {
            {
                let mut s = state.borrow_mut();
                if s.is_appbar {
                    unregister_appbar(s.hwnd);
                    s.is_appbar = false;
                }
                s.settings.dock_edge = edge;
                s.settings = s.settings.normalized();
                let _ = s.settings.save(s.profile_name.as_deref());
                if edge.is_some() {
                    apply_dock_mode(&mut s);
                } else {
                    restore_floating_style(s.hwnd);
                    apply_topmost_mode(s.hwnd, s.settings.always_on_top);
                }
            }
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::ToggleToolbar => {
            update_settings(state, |s| {
                s.show_toolbar = !s.show_toolbar;
            });
            refresh_ui(state, weak);
        }
        TrayAction::OpenSettingsWindow => {
            open_settings_window(state, weak);
        }
        TrayAction::Exit => {
            PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Exit));
        }
    }
}

fn handle_pending_action(state: &Rc<RefCell<AppState>>, win: &MainWindow, action: PendingAction) {
    let weak = win.as_weak();
    match action {
        PendingAction::Tray(ta) => handle_tray_action(state, &weak, ta),
        PendingAction::Reposition => {
            if let Ok(mut s) = state.try_borrow_mut() {
                if s.is_appbar {
                    reposition_appbar(&mut s);
                }
            }
        }
        PendingAction::HideToTray => {
            release_all_thumbnails(state);
            win.hide().ok();
        }
        PendingAction::Refresh => {
            if refresh_windows(state) {
                recompute_and_update_ui(state, win);
            }
        }
        PendingAction::Exit => {
            request_exit(state);
        }
    }
}

// ───────────────────────── Settings Window ─────────────────────────

fn open_settings_window(state: &Rc<RefCell<AppState>>, _main_weak: &slint::Weak<MainWindow>) {
    let already_open = SETTINGS_WIN.with(|h| h.borrow().is_some());
    if already_open {
        return;
    }

    let sw = match SettingsWindow::new() {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("failed to create settings window: {e}");
            return;
        }
    };

    populate_settings_window(&sw, &state.borrow().settings);

    sw.on_apply({
        let state = state.clone();
        move || {
            SETTINGS_WIN.with(|h| {
                let guard = h.borrow();
                let Some(sw) = guard.as_ref() else { return };
                let mut s = state.borrow_mut();
                let layout = apply_settings_window_changes(sw, &mut s.settings);
                s.current_layout = layout;
                s.settings = s.settings.normalized();
                let _ = s.settings.save(s.profile_name.as_deref());
                let hwnd = s.hwnd;
                let always_on_top = s.settings.always_on_top;
                let settings_clone = s.settings.clone();
                drop(s);
                apply_window_appearance(hwnd, &settings_clone);
                apply_topmost_mode(hwnd, always_on_top);
            });
        }
    });

    sw.on_closed({
        move || {
            let taken = SETTINGS_WIN.with(|h| h.borrow_mut().take());
            if let Some(w) = taken {
                w.hide().ok();
            }
        }
    });

    // Apply DWM dark mode to the settings window.
    sw.show().unwrap();
    if let Some(sw_hwnd) = get_hwnd(sw.window()) {
        apply_window_appearance(sw_hwnd, &state.borrow().settings);
    }
    SETTINGS_WIN.with(|h| *h.borrow_mut() = Some(sw));
}

// ───────────────────────── Layout / State helpers ─────────────────────────

fn cycle_layout(state: &Rc<RefCell<AppState>>) {
    let mut s = state.borrow_mut();
    s.current_layout = s.current_layout.next();
    s.settings.initial_layout = s.current_layout;
    s.drag_separator = None;
    let _ = s.settings.save(s.profile_name.as_deref());
}

fn set_layout(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>, layout: LayoutType) {
    {
        let mut s = state.borrow_mut();
        if s.current_layout == layout {
            return;
        }
        s.current_layout = layout;
        s.settings.initial_layout = layout;
        s.drag_separator = None;
        let _ = s.settings.save(s.profile_name.as_deref());
    }
    refresh_ui(state, weak);
}

fn update_settings(state: &Rc<RefCell<AppState>>, mutate: impl FnOnce(&mut AppSettings)) {
    let mut s = state.borrow_mut();
    mutate(&mut s.settings);
    s.settings = s.settings.normalized();
    let _ = s.settings.save(s.profile_name.as_deref());
}

fn refresh_ui(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
    }
}

fn advance_animation(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let Ok(mut s) = state.try_borrow_mut() else {
        return;
    };
    let Some(started_at) = s.animation_started_at else {
        return;
    };
    if !unsafe { IsWindowVisible(s.hwnd).as_bool() } {
        s.animation_started_at = None;
        return;
    }

    let elapsed_ms = started_at.elapsed().as_millis() as u32;
    let progress = (elapsed_ms as f32 / ANIMATION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);

    for mw in &mut s.windows {
        mw.display_rect = lerp_rect(mw.animation_from_rect, mw.target_rect, eased);
    }

    if progress >= 1.0 {
        s.animation_started_at = None;
        for mw in &mut s.windows {
            mw.display_rect = mw.target_rect;
        }
    }

    // Update the Slint model with new positions.
    let accent = tag_accent_color(&s.settings);
    let show_footer = s.settings.show_window_info;
    let data: Vec<ThumbnailData> = s
        .windows
        .iter()
        .enumerate()
        .map(|(i, mw)| ThumbnailData {
            x: mw.display_rect.left as f32,
            y: mw.display_rect.top as f32,
            width: (mw.display_rect.right - mw.display_rect.left) as f32,
            height: (mw.display_rect.bottom - mw.display_rect.top) as f32,
            title: SharedString::from(truncate_title(&mw.info.title)),
            app_label: SharedString::from(mw.info.app_label()),
            is_hovered: s.hover_index == Some(i),
            accent_color: accent,
            show_footer,
        })
        .collect();
    drop(s);
    win.set_thumbnails(ModelRc::new(VecModel::from(data)));
}

// ───────────────────────── Visibility ─────────────────────────

fn toggle_visibility(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    let visible = state.borrow().hwnd != HWND::default()
        && unsafe { IsWindowVisible(state.borrow().hwnd).as_bool() };
    if visible {
        release_all_thumbnails(state);
        if let Some(win) = weak.upgrade() {
            win.hide().ok();
        }
    } else {
        restore_from_tray(state, weak);
    }
}

fn restore_from_tray(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    if let Some(win) = weak.upgrade() {
        win.show().ok();
        let hwnd = state.borrow().hwnd;
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        refresh_windows(state);
        recompute_and_update_ui(state, &win);
    }
}

fn request_exit(state: &Rc<RefCell<AppState>>) {
    tracing::info!("exiting Panopticon");
    {
        let mut s = state.borrow_mut();
        if s.is_appbar {
            unregister_appbar(s.hwnd);
            s.is_appbar = false;
        }
        s.windows.clear();
        if let Some(tray) = s.tray_icon.as_mut() {
            tray.remove();
        }
    }
    SETTINGS_WIN.with(|h| {
        h.borrow_mut().take();
    });
    slint::quit_event_loop().ok();
}

// ───────────────────────── Window Appearance ─────────────────────────

fn apply_window_appearance(hwnd: HWND, settings: &AppSettings) {
    let dark_mode: i32 = 1;
    let corner = DWMWCP_ROUND;
    let backdrop = if settings.use_system_backdrop {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::from_ref(&dark_mode).cast::<c_void>(),
            mem::size_of_val(&dark_mode) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corner).cast::<c_void>(),
            mem::size_of_val(&corner) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::from_ref(&backdrop).cast::<c_void>(),
            mem::size_of_val(&backdrop) as u32,
        );
    }
}

fn apply_topmost_mode(hwnd: HWND, always_on_top: bool) {
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

// ───────────────────────── App-bar Docking ─────────────────────────

fn apply_dock_mode(state: &mut AppState) {
    let hwnd = state.hwnd;
    unsafe {
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_VISIBLE).0 as isize);
    }
    if register_appbar(hwnd) {
        state.is_appbar = true;
        reposition_appbar(state);
    }
}

fn register_appbar(hwnd: HWND) -> bool {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: WM_APPBAR_CALLBACK,
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_NEW, &raw mut abd) != 0 }
}

fn unregister_appbar(hwnd: HWND) {
    let mut abd = APPBARDATA {
        cbSize: mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    };
    unsafe {
        let _ = SHAppBarMessage(ABM_REMOVE, &raw mut abd);
    }
}

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

    unsafe {
        let _ = SHAppBarMessage(ABM_QUERYPOS, &raw mut abd);
        match edge {
            DockEdge::Left => abd.rc.right = abd.rc.left + thickness,
            DockEdge::Right => abd.rc.left = abd.rc.right - thickness,
            DockEdge::Top => abd.rc.bottom = abd.rc.top + thickness,
            DockEdge::Bottom => abd.rc.top = abd.rc.bottom - thickness,
        }
        let _ = SHAppBarMessage(ABM_SETPOS, &raw mut abd);
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

fn restore_floating_style(hwnd: HWND) {
    unsafe {
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
    }
}

const fn dock_edge_to_abe(edge: DockEdge) -> u32 {
    match edge {
        DockEdge::Left => ABE_LEFT,
        DockEdge::Right => ABE_RIGHT,
        DockEdge::Top => ABE_TOP,
        DockEdge::Bottom => ABE_BOTTOM,
    }
}

fn get_monitor_rect(hwnd: HWND) -> RECT {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &raw mut info).as_bool() {
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

// ───────────────────────── Utility ─────────────────────────

fn parse_profile_from_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--profile" && i + 1 < args.len() {
            let name = args[i + 1].trim().to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }
        i += 1;
    }
    None
}

fn tag_accent_color(settings: &AppSettings) -> slint::Color {
    settings
        .active_tag_filter
        .as_deref()
        .map_or(slint::Color::from_rgb_u8(0xD2, 0x9A, 0x5C), |tag| {
            hex_to_slint_color(&settings.tag_color_hex(tag))
        })
}

fn hex_to_slint_color(hex: &str) -> slint::Color {
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("D2"), 16).unwrap_or(0xD2);
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("9A"), 16).unwrap_or(0x9A);
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("5C"), 16).unwrap_or(0x5C);
    slint::Color::from_rgb_u8(r, g, b)
}

fn truncate_title(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() > 40 {
        let mut short: String = chars[..37].iter().collect();
        short.push_str("...");
        short
    } else {
        title.to_owned()
    }
}

fn active_filter_summary(settings: &AppSettings) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(m) = &settings.active_monitor_filter {
        parts.push(format!("monitor:{m}"));
    }
    if let Some(g) = settings.active_group_filter_label() {
        parts.push(g);
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn collect_available_monitors(windows: &[WindowInfo]) -> Vec<String> {
    let set: BTreeSet<String> = windows.iter().map(|w| w.monitor_name.clone()).collect();
    set.into_iter().collect()
}

fn collect_available_apps(windows: &[WindowInfo]) -> Vec<AppSelectionEntry> {
    let mut map: HashMap<String, String> = HashMap::new();
    for w in windows {
        map.entry(w.app_id.clone()).or_insert_with(|| w.app_label());
    }
    let mut apps: Vec<AppSelectionEntry> = map
        .into_iter()
        .map(|(app_id, label)| AppSelectionEntry { app_id, label })
        .collect();
    apps.sort_by(|a, b| a.label.cmp(&b.label).then(a.app_id.cmp(&b.app_id)));
    apps
}

fn rect_has_area(rect: RECT) -> bool {
    rect.right > rect.left && rect.bottom > rect.top
}

fn lerp_rect(from: RECT, to: RECT, t: f32) -> RECT {
    RECT {
        left: lerp_i32(from.left, to.left, t),
        top: lerp_i32(from.top, to.top, t),
        right: lerp_i32(from.right, to.right, t),
        bottom: lerp_i32(from.bottom, to.bottom, t),
    }
}

fn lerp_i32(from: i32, to: i32, t: f32) -> i32 {
    (from as f32 + (to - from) as f32 * t).round() as i32
}
