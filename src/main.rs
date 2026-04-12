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

use app::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, docked_mode_active,
    is_blocked_dock_syscommand, reposition_appbar, restore_floating_style, sync_dock_system_menu,
    unregister_appbar,
};
use app::dwm::{
    ensure_thumbnail, query_source_size, release_all_thumbnails, release_thumbnail,
    update_dwm_thumbnails,
};
use app::icon::populate_cached_icon;
use app::theme_ui::{
    advance_theme_animation, apply_main_window_theme_snapshot, sync_theme_target,
    thumbnail_accent_color,
};
use app::tray::{
    apply_window_icons, handle_tray_message, show_application_context_menu_at, AppIcons,
    TrayAction, TrayIcon, TrayMenuState, INSTANCE_ACCENT_PALETTE, WM_TRAYICON,
};
use panopticon::constants::{ANIMATION_DURATION_MS, TOOLBAR_HEIGHT};
use panopticon::layout::{
    apply_separator_drag, apply_separator_drag_grouped, compute_layout_custom, default_ratios,
    AspectHint, LayoutType, ScrollDirection, Separator,
};
use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{
    active_filter_summary, apply_pinned_positions, collect_available_apps,
    collect_available_monitors, sort_windows_for_grouping, truncate_title,
};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::mem;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{
    CloseRequestResponse, ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel,
};

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};

use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Shell::ABN_POSCHANGED;
use windows::Win32::UI::WindowsAndMessaging::*;

slint::include_modules!();

// ───────────────────────── Constants ─────────────────────────

/// Callback message posted by the shell when the app-bar needs repositioning.
pub(crate) const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

const OPTION_SEPARATOR: &str = " — ";
pub(crate) const THUMBNAIL_INFO_STRIP_HEIGHT: i32 = 26;
pub(crate) const THUMBNAIL_CONTENT_PADDING: i32 = 6;
pub(crate) const THEME_TRANSITION_DURATION_MS: u32 = 220;
pub(crate) const HIDDEN_THUMBNAIL_RECT: RECT = RECT {
    left: 0,
    top: 0,
    right: 1,
    bottom: 1,
};

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

// ───────────────────────── Thread-local subclass state ─────────────────────────

thread_local! {
    static ORIGINAL_WNDPROC: Cell<isize> = const { Cell::new(0) };
    static UI_STATE: RefCell<Option<Rc<RefCell<AppState>>>> = const { RefCell::new(None) };
    static UI_WINDOW: RefCell<Option<slint::Weak<MainWindow>>> = const { RefCell::new(None) };
    static PENDING_ACTIONS: RefCell<Vec<PendingAction>> = const { RefCell::new(Vec::new()) };
    static SETTINGS_WIN: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
    static TAG_DIALOG_WIN: RefCell<Option<TagDialogWindow>> = const { RefCell::new(None) };
    static PAN_STATE: RefCell<MiddlePanState> = const { RefCell::new(MiddlePanState { active: false, last_x: 0, last_y: 0 }) };
    /// Instant when the last scroll event occurred; used by the scrollbar
    /// auto-hide timer to determine when to fade out.
    static SCROLL_LAST_ACTIVITY: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Populate the `Tr` global on any Slint window with the current locale strings.
macro_rules! populate_tr_global {
    ($window:expr) => {{
        use panopticon::i18n;
        let tr = $window.global::<Tr>();
        tr.set_minimized(SharedString::from(i18n::t("ui.minimized")));
        tr.set_last_seen(SharedString::from(i18n::t("ui.last_seen")));
        tr.set_visible_label(SharedString::from(i18n::t("ui.visible")));
        tr.set_hidden_label(SharedString::from(i18n::t("ui.hidden")));
        tr.set_always_on_top_label(SharedString::from(i18n::t("ui.always_on_top")));
        tr.set_normal_window_label(SharedString::from(i18n::t("ui.normal_window")));
        tr.set_toolbar_hint(SharedString::from(i18n::t("ui.toolbar_hint")));
        tr.set_anim_on(SharedString::from(i18n::t("ui.anim_on")));
        tr.set_anim_off(SharedString::from(i18n::t("ui.anim_off")));
        tr.set_empty_message(SharedString::from(i18n::t("ui.empty_message")));
        tr.set_empty_helper(SharedString::from(i18n::t("ui.empty_helper")));
        tr.set_dock_mode_hint(SharedString::from(i18n::t("settings.dock_hint")));
        tr.set_filters_hint(SharedString::from(i18n::t("settings.filters_hint")));
        tr.set_current_profile_prefix(SharedString::from(i18n::t("settings.current_profile")));
        tr.set_profile_input_label(SharedString::from(i18n::t("settings.profile_label")));
        tr.set_save_profile_btn(SharedString::from(i18n::t("settings.save_profile")));
        tr.set_open_instance_btn(SharedString::from(i18n::t("settings.open_instance")));
        tr.set_no_hidden_hint(SharedString::from(i18n::t("settings.no_hidden_hint")));
        tr.set_tag_title(SharedString::from(i18n::t("tag.title")));
        tr.set_tag_app_label(SharedString::from(i18n::t("tag.application")));
        tr.set_tag_name_label(SharedString::from(i18n::t("tag.name_label")));
        tr.set_tag_preset_colour(SharedString::from(i18n::t("tag.preset_colour")));
        tr.set_tag_create_assign(SharedString::from(i18n::t("tag.create_assign")));
    }};
}

pub(crate) fn populate_tr_global<Component>(window: &Component)
where
    Component: ComponentHandle,
    for<'a> Tr<'a>: slint::Global<'a, Component>,
{
    use panopticon::i18n;

    let tr = window.global::<Tr>();
    tr.set_minimized(SharedString::from(i18n::t("ui.minimized")));
    tr.set_last_seen(SharedString::from(i18n::t("ui.last_seen")));
    tr.set_visible_label(SharedString::from(i18n::t("ui.visible")));
    tr.set_hidden_label(SharedString::from(i18n::t("ui.hidden")));
    tr.set_always_on_top_label(SharedString::from(i18n::t("ui.always_on_top")));
    tr.set_normal_window_label(SharedString::from(i18n::t("ui.normal_window")));
    tr.set_toolbar_hint(SharedString::from(i18n::t("ui.toolbar_hint")));
    tr.set_anim_on(SharedString::from(i18n::t("ui.anim_on")));
    tr.set_anim_off(SharedString::from(i18n::t("ui.anim_off")));
    tr.set_empty_message(SharedString::from(i18n::t("ui.empty_message")));
    tr.set_empty_helper(SharedString::from(i18n::t("ui.empty_helper")));
    tr.set_dock_mode_hint(SharedString::from(i18n::t("settings.dock_hint")));
    tr.set_filters_hint(SharedString::from(i18n::t("settings.filters_hint")));
    tr.set_current_profile_prefix(SharedString::from(i18n::t("settings.current_profile")));
    tr.set_profile_input_label(SharedString::from(i18n::t("settings.profile_label")));
    tr.set_save_profile_btn(SharedString::from(i18n::t("settings.save_profile")));
    tr.set_open_instance_btn(SharedString::from(i18n::t("settings.open_instance")));
    tr.set_no_hidden_hint(SharedString::from(i18n::t("settings.no_hidden_hint")));
    tr.set_tag_title(SharedString::from(i18n::t("tag.title")));
    tr.set_tag_app_label(SharedString::from(i18n::t("tag.application")));
    tr.set_tag_name_label(SharedString::from(i18n::t("tag.name_label")));
    tr.set_tag_preset_colour(SharedString::from(i18n::t("tag.preset_colour")));
    tr.set_tag_create_assign(SharedString::from(i18n::t("tag.create_assign")));
}

/// Tracks middle-button pan drag state.
struct MiddlePanState {
    active: bool,
    last_x: i32,
    last_y: i32,
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
    /// Last pointer offset inside the handle, used for incremental movement.
    last_pointer_offset: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeAnimation {
    pub(crate) from: theme_catalog::UiTheme,
    pub(crate) to: theme_catalog::UiTheme,
    pub(crate) started_at: Instant,
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

// ───────────────────────── Entry Point ─────────────────────────

#[allow(clippy::too_many_lines)]
fn main() {
    let _log_guard = panopticon::logging::init().ok();
    panopticon::i18n::init();
    let profile = parse_profile_from_args();
    tracing::info!(profile = ?profile, "Panopticon starting (Slint UI)");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::Relaxed);
    }

    let icons = match profile.as_deref() {
        Some(name) => {
            let idx = name.bytes().fold(0u32, |a, b| a.wrapping_add(u32::from(b))) as usize
                % INSTANCE_ACCENT_PALETTE.len();
            let [r, g, b] = INSTANCE_ACCENT_PALETTE[idx];
            AppIcons::with_accent(r, g, b).unwrap_or_else(|_| {
                AppIcons::new().unwrap_or_else(|error| {
                    tracing::error!(%error, "icon generation failed; falling back");
                    AppIcons::fallback_system()
                })
            })
        }
        None => AppIcons::new().unwrap_or_else(|error| {
            tracing::error!(%error, "icon generation failed; falling back");
            AppIcons::fallback_system()
        }),
    };
    let settings = AppSettings::load_or_default(profile.as_deref()).unwrap_or_else(|error| {
        tracing::error!(%error, "settings load failed; using defaults");
        AppSettings::default()
    });
    app::secondary_windows::ensure_default_profiles_exist(&settings);

    let initial_theme = theme_catalog::resolve_ui_theme(
        settings.theme_id.as_deref(),
        &settings.background_color_hex,
    );

    let main_window = match MainWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create main window");
            return;
        }
    };
    populate_tr_global!(main_window);
    apply_main_window_theme_snapshot(&main_window, &initial_theme);

    // Apply initial property values from settings.
    sync_settings_to_ui(&main_window, &settings);

    let state = Rc::new(RefCell::new(AppState {
        hwnd: HWND::default(),
        windows: Vec::new(),
        current_layout: settings.initial_layout,
        active_hwnd: None,
        tray_icon: None,
        icons,
        settings,
        animation_started_at: None,
        content_extent: 0,
        is_appbar: false,
        profile_name: profile,
        last_size: (0, 0),
        separators: Vec::new(),
        drag_separator: None,
        loaded_background_path: None,
        current_theme: initial_theme,
        theme_animation: None,
    }));

    // Show the window so the native HWND exists on next event-loop iteration.
    if let Err(error) = main_window.show() {
        tracing::error!(%error, "failed to show main window");
        return;
    }

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
            let _ = try_initialize_native_runtime(&state, &win);
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
            if state.borrow().hwnd.0.is_null() && !try_initialize_native_runtime(&state, &win) {
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

            // Smoothly interpolate theme changes.
            advance_theme_animation(&state, &win);

            // Re-sync DWM thumbnails (scroll changes, animation frames, etc.).
            update_dwm_thumbnails(&state, &win);
        }
    });

    // Slow refresh timer: window enumeration.
    let refresh_timer = Timer::default();
    refresh_timer.start(
        TimerMode::Repeated,
        Duration::from_millis((state.borrow().settings.refresh_interval_ms as u64).max(50)),
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

    // Scrollbar auto-hide timer: checks every 200 ms and hides after inactivity.
    let scrollbar_timer = Timer::default();
    scrollbar_timer.start(TimerMode::Repeated, Duration::from_millis(200), {
        let weak = main_window.as_weak();
        move || {
            let should_hide = SCROLL_LAST_ACTIVITY
                .with(|c| c.get().is_some_and(|t| t.elapsed() >= SCROLLBAR_HIDE_DELAY));
            if should_hide {
                SCROLL_LAST_ACTIVITY.with(|c| c.set(None));
                if let Some(win) = weak.upgrade() {
                    win.set_scroll_active(false);
                }
            }
        }
    });

    tracing::info!("entering Slint event loop");
    if let Err(error) = slint::run_event_loop_until_quit() {
        tracing::error!(%error, "Slint event loop failed");
    }
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

fn try_initialize_native_runtime(state: &Rc<RefCell<AppState>>, win: &MainWindow) -> bool {
    if !state.borrow().hwnd.0.is_null() {
        return true;
    }

    let Some(hwnd) = get_hwnd(win.window()) else {
        tracing::debug!("HWND not ready yet; deferring native initialization");
        return false;
    };

    {
        let mut s = state.borrow_mut();
        if !s.hwnd.0.is_null() {
            return true;
        }
        s.hwnd = hwnd;
    }

    let settings_snapshot = state.borrow().settings.clone();
    tracing::info!(hwnd = ?hwnd, "native HWND acquired");

    {
        let state = state.borrow();
        apply_window_icons(hwnd, &state.icons);
    }

    // DWM appearance.
    apply_window_appearance(hwnd, &settings_snapshot);
    apply_topmost_mode(hwnd, settings_snapshot.always_on_top);
    sync_dock_system_menu(hwnd, settings_snapshot.dock_edge.is_some());

    // System tray.
    {
        let mut s = state.borrow_mut();
        match TrayIcon::add(hwnd, s.icons.small) {
            Ok(tray) => {
                tracing::info!("tray icon registered");
                s.tray_icon = Some(tray);
            }
            Err(error) => tracing::error!(%error, "tray icon registration failed"),
        }
    }

    // Subclass the Slint HWND to intercept tray / appbar / minimize messages.
    setup_subclass(hwnd, state, win);

    let refreshed = refresh_windows(state);
    let tracked = state.borrow().windows.len();
    tracing::info!(
        refreshed,
        tracked_windows = tracked,
        "initial window refresh completed"
    );
    recompute_and_update_ui(state, win);

    // App-bar registration (if dock edge is set).
    if settings_snapshot.dock_edge.is_some() {
        let mut s = state.borrow_mut();
        apply_dock_mode(&mut s);
    }

    // Start minimized to tray when the setting is active.
    if settings_snapshot.start_in_tray {
        tracing::info!("start_in_tray is active — hiding main window");
        for mw in &mut state.borrow_mut().windows {
            release_thumbnail(mw);
        }
        win.hide().ok();
    }

    true
}

// ───────────────────────── HWND Subclass ─────────────────────────

fn setup_subclass(hwnd: HWND, state: &Rc<RefCell<AppState>>, main_window: &MainWindow) {
    UI_STATE.with(|s| *s.borrow_mut() = Some(state.clone()));
    UI_WINDOW.with(|w| *w.borrow_mut() = Some(main_window.as_weak()));

    // SAFETY: hwnd is a live window created by winit; we read then replace
    // the WndProc pointer on the same UI thread that owns the window.
    let original = unsafe { GetWindowLongPtrW(hwnd, GWL_WNDPROC) };
    ORIGINAL_WNDPROC.with(|p| p.set(original));

    // SAFETY: same UI thread; we install our subclass proc and keep the
    // original pointer in ORIGINAL_WNDPROC for forwarding.
    unsafe {
        let subclass_proc_ptr = subclass_proc as *const () as isize;
        let _ = SetWindowLongPtrW(hwnd, GWL_WNDPROC, subclass_proc_ptr);
    }
}

fn teardown_subclass(hwnd: HWND) {
    let original = ORIGINAL_WNDPROC.with(Cell::get);
    if original != 0 {
        // SAFETY: restoring the original WndProc saved during setup_subclass;
        // called on the same UI thread that owns the window.
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
    if original == 0 {
        // SAFETY: DefWindowProcW is always valid as a fallback.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: `original` points to winit's WndProc set during window creation.
    unsafe { CallWindowProcW(mem::transmute(original), hwnd, msg, wparam, lparam) }
}

#[allow(clippy::too_many_lines)]
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
            handle_tray_subclass(hwnd, lparam);
            LRESULT(0)
        }
        WM_SYSKEYDOWN => {
            if wparam.0 as u32 == 0x12 && (lparam.0 & 0x4000_0000) == 0 {
                toggle_toolbar_from_alt_hotkey();
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        WM_APPBAR_CALLBACK => {
            if wparam.0 as u32 == ABN_POSCHANGED {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Reposition));
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGED | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            if docked_mode_active() {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Reposition));
            }
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_SYSCOMMAND => {
            if docked_mode_active() && is_blocked_dock_syscommand(wparam.0) {
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
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
            handle_show_window(wparam);
            forward_to_original(hwnd, msg, wparam, lparam)
        }
        WM_MBUTTONDOWN => {
            let x = (lparam.0 & 0xFFFF) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
            PAN_STATE.with(|p| {
                let mut pan = p.borrow_mut();
                pan.active = true;
                pan.last_x = x;
                pan.last_y = y;
            });
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let handled = handle_wheel_scroll(wparam);
            if handled {
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        WM_MBUTTONUP => {
            PAN_STATE.with(|p| p.borrow_mut().active = false);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pan_active = PAN_STATE.with(|p| p.borrow().active);
            if pan_active {
                // MK_MBUTTON (0x0010) — reset stale pan if middle button was
                // released outside the window (WM_MBUTTONUP never received).
                if wparam.0 & 0x0010 == 0 {
                    PAN_STATE.with(|p| p.borrow_mut().active = false);
                    return forward_to_original(hwnd, msg, wparam, lparam);
                }
                handle_middle_pan_move(lparam);
                LRESULT(0)
            } else {
                forward_to_original(hwnd, msg, wparam, lparam)
            }
        }
        _ => forward_to_original(hwnd, msg, wparam, lparam),
    }
}

/// Handles `WM_SHOWWINDOW` — enqueue refresh on show, release thumbnails on hide.
fn handle_show_window(wparam: WPARAM) {
    if wparam.0 != 0 {
        PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Refresh));
    } else {
        UI_STATE.with(|s| {
            if let Some(rc) = s.borrow().as_ref() {
                if let Ok(mut st) = rc.try_borrow_mut() {
                    for mw in &mut st.windows {
                        release_thumbnail(mw);
                    }
                }
            }
        });
    }
}

/// Handles tray icon messages (left/right click).
fn handle_tray_subclass(hwnd: HWND, lparam: LPARAM) {
    let mouse_msg = lparam.0 as u32;
    if mouse_msg == WM_LBUTTONUP {
        PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Tray(TrayAction::Toggle)));
    } else if mouse_msg == WM_RBUTTONUP {
        let menu_state = UI_STATE.with(|s| {
            s.borrow().as_ref().and_then(|rc| {
                rc.try_borrow_mut()
                    .ok()
                    .map(|mut st| build_tray_menu_state(&mut st))
            })
        });
        if let Some(menu_state) = menu_state {
            if let Some(action) = handle_tray_message(hwnd, lparam, &menu_state) {
                PENDING_ACTIONS.with(|q| q.borrow_mut().push(PendingAction::Tray(action)));
            }
        }
    }
}

/// Handles mouse wheel scroll — maps vertical wheel to horizontal for Row mode.
/// Returns `true` if the event was consumed.
fn handle_wheel_scroll(wparam: WPARAM) -> bool {
    let delta = (wparam.0 >> 16) as i16 as f32; // high word = wheel delta
    let scroll_px = delta / 120.0 * 48.0; // 48 logical px per wheel tick
    UI_WINDOW.with(|w| {
        let Some(win) = w.borrow().as_ref().and_then(slint::Weak::upgrade) else {
            return false;
        };
        let scroll_h = win.get_scroll_horizontal();
        let scroll_v = win.get_scroll_vertical();
        if scroll_h {
            let scale = win.window().scale_factor();
            let phys = win.window().size();
            let cw = win.get_content_width();
            let visible = phys.width as f32 / scale;
            let max_scroll = (cw - visible).max(0.0);
            let new_vx = (win.get_viewport_x() + scroll_px).clamp(-max_scroll, 0.0);
            win.set_viewport_x(new_vx);
            flash_scrollbar(&win);
            true
        } else if scroll_v {
            let scale = win.window().scale_factor();
            let phys = win.window().size();
            let toolbar_h = if win.get_show_toolbar() {
                TOOLBAR_HEIGHT as f32
            } else {
                0.0
            };
            let ch = win.get_content_height();
            let visible = phys.height as f32 / scale - toolbar_h;
            let max_scroll = (ch - visible).max(0.0);
            let new_vy = (win.get_viewport_y() + scroll_px).clamp(-max_scroll, 0.0);
            win.set_viewport_y(new_vy);
            flash_scrollbar(&win);
            true
        } else {
            false
        }
    })
}

/// Applies a middle-button pan delta to the scroll viewport.
fn handle_middle_pan_move(lparam: LPARAM) {
    let x = (lparam.0 & 0xFFFF) as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
    PAN_STATE.with(|p| {
        let mut pan = p.borrow_mut();
        let dx = x - pan.last_x;
        let dy = y - pan.last_y;
        pan.last_x = x;
        pan.last_y = y;
        UI_WINDOW.with(|w| {
            if let Some(win) = w.borrow().as_ref().and_then(slint::Weak::upgrade) {
                let scale = win.window().scale_factor();
                let scroll_h = win.get_scroll_horizontal();
                let scroll_v = win.get_scroll_vertical();
                let mut scrolled = false;
                if scroll_h {
                    let phys = win.window().size();
                    let cw = win.get_content_width();
                    let visible = phys.width as f32 / scale;
                    let max_scroll = (cw - visible).max(0.0);
                    let new_vx = (win.get_viewport_x() + dx as f32 / scale).clamp(-max_scroll, 0.0);
                    win.set_viewport_x(new_vx);
                    scrolled = true;
                }
                if scroll_v {
                    let phys = win.window().size();
                    let toolbar_h = if win.get_show_toolbar() {
                        TOOLBAR_HEIGHT as f32
                    } else {
                        0.0
                    };
                    let ch = win.get_content_height();
                    let visible = phys.height as f32 / scale - toolbar_h;
                    let max_scroll = (ch - visible).max(0.0);
                    let new_vy = (win.get_viewport_y() + dy as f32 / scale).clamp(-max_scroll, 0.0);
                    win.set_viewport_y(new_vy);
                    scrolled = true;
                }
                if scrolled {
                    flash_scrollbar(&win);
                }
            }
        });
    });
}

/// Duration after which the scrollbar overlay auto-hides.
const SCROLLBAR_HIDE_DELAY: Duration = Duration::from_millis(1500);

/// Mark the scrollbar as visible and record activity time for auto-hide.
fn flash_scrollbar(win: &MainWindow) {
    win.set_scroll_active(true);
    SCROLL_LAST_ACTIVITY.with(|c| c.set(Some(Instant::now())));
}

// ───────────────────────── Slint Callbacks ─────────────────────────

#[allow(clippy::too_many_lines)]
fn setup_callbacks(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) {
    main_window.on_thumbnail_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            app::thumbnail_interactions::handle_thumbnail_click(&state, &weak, index as usize);
        }
    });

    main_window.on_thumbnail_right_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            app::thumbnail_interactions::handle_thumbnail_right_click(
                &state,
                &weak,
                index as usize,
                x,
                y,
            );
        }
    });

    main_window.on_thumbnail_drag_ended({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |src_idx, drop_x, drop_y| {
            app::thumbnail_interactions::handle_thumbnail_drag_ended(
                &state,
                &weak,
                src_idx as usize,
                drop_x as f64,
                drop_y as f64,
            );
        }
    });

    main_window.on_thumbnail_close_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            app::thumbnail_interactions::handle_thumbnail_close(&state, &weak, index as usize);
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

    main_window.on_app_context_menu_requested({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |x, y| open_application_context_menu(&state, &weak, Some((x, y)))
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

    let mut discovered: Vec<WindowInfo> = discovered_all
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

    sort_windows_for_grouping(&mut discovered, &s.settings);
    apply_pinned_positions(&mut discovered, &s.settings);

    let discovered_map: HashMap<isize, &WindowInfo> =
        discovered.iter().map(|w| (w.hwnd.0 as isize, w)).collect();
    let discovered_hwnds: HashSet<isize> = discovered_map.keys().copied().collect();
    let discovered_order: HashMap<isize, usize> = discovered
        .iter()
        .enumerate()
        .map(|(index, window)| (window.hwnd.0 as isize, index))
        .collect();

    let prev_len = s.windows.len();
    s.windows
        .retain(|mw| discovered_hwnds.contains(&(mw.info.hwnd.0 as isize)));
    let mut changed = s.windows.len() != prev_len;

    changed |= update_existing_windows(&mut s.windows, &discovered_map, host_hwnd, host_visible);

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
            last_thumb_update: None,
            last_thumb_dest: None,
            last_thumb_visible: false,
            cached_icon: None,
        };
        if host_visible {
            let _ = ensure_thumbnail(host_hwnd, &mut mw);
        }
        s.windows.push(mw);
        changed = true;
    }

    let order_before: Vec<isize> = s.windows.iter().map(|mw| mw.info.hwnd.0 as isize).collect();
    s.windows.sort_by_key(|mw| {
        discovered_order
            .get(&(mw.info.hwnd.0 as isize))
            .copied()
            .unwrap_or(usize::MAX)
    });
    let order_after: Vec<isize> = s.windows.iter().map(|mw| mw.info.hwnd.0 as isize).collect();
    if order_before != order_after {
        changed = true;
    }

    changed
}

fn update_existing_windows(
    windows: &mut [ManagedWindow],
    discovered_map: &HashMap<isize, &WindowInfo>,
    host_hwnd: HWND,
    host_visible: bool,
) -> bool {
    let mut changed = false;
    for mw in windows.iter_mut() {
        if let Some(fresh) = discovered_map.get(&(mw.info.hwnd.0 as isize)) {
            let metadata_changed = fresh.title != mw.info.title
                || fresh.app_id != mw.info.app_id
                || fresh.process_name != mw.info.process_name
                || fresh.process_path != mw.info.process_path
                || fresh.class_name != mw.info.class_name
                || fresh.monitor_name != mw.info.monitor_name;
            if metadata_changed {
                let icon_changed =
                    fresh.app_id != mw.info.app_id || fresh.process_path != mw.info.process_path;
                mw.info = (*fresh).clone();
                if icon_changed {
                    mw.cached_icon = None;
                }
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
    changed
}

// ───────────────────────── Layout + UI Sync ─────────────────────────

fn recompute_and_update_ui(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let mut s = state.borrow_mut();
    if s.windows.is_empty() {
        s.animation_started_at = None;
        sync_theme_target(&mut s);
        sync_settings_to_ui(win, &s.settings);
        sync_background_image(&mut s, win);
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
        && s.drag_separator.is_none()
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
    clamp_viewport_offsets(
        win,
        scroll_dir,
        s.content_extent,
        logical_w,
        content_area.bottom,
    );

    sync_theme_target(&mut s);
    sync_settings_to_ui(win, &s.settings);
    sync_background_image(&mut s, win);

    drop(s);
    sync_model_to_slint(state, win);
}

fn sync_settings_to_ui(win: &MainWindow, settings: &AppSettings) {
    win.set_show_toolbar(settings.show_toolbar);
    win.set_show_window_info(settings.show_window_info);
    win.set_is_always_on_top(settings.always_on_top);
    win.set_animate_transitions(settings.animate_transitions);
    win.set_resize_locked(settings.locked_layout || settings.lock_cell_resize);
    win.set_refresh_label(SharedString::from(settings.refresh_interval_label()));
    win.set_filters_label(SharedString::from(
        active_filter_summary(settings).unwrap_or_default(),
    ));
}

fn sync_background_image(state: &mut AppState, win: &MainWindow) {
    let desired = state.settings.background_image_path.clone();
    if state.loaded_background_path == desired {
        return;
    }

    if let Some(path) = desired.as_deref() {
        match slint::Image::load_from_path(std::path::Path::new(path)) {
            Ok(image) => {
                win.set_background_image(image);
                state.loaded_background_path = desired;
            }
            Err(error) => {
                tracing::warn!(%error, path, "failed to load background image");
                win.set_background_image(slint::Image::default());
                state.loaded_background_path = None;
            }
        }
    } else {
        win.set_background_image(slint::Image::default());
        state.loaded_background_path = None;
    }
}

fn clamp_viewport_offsets(
    win: &MainWindow,
    scroll_dir: ScrollDirection,
    content_extent: i32,
    visible_width: i32,
    visible_height: i32,
) {
    match scroll_dir {
        ScrollDirection::Horizontal => {
            let max_scroll = (content_extent - visible_width).max(0) as f32;
            win.set_viewport_x(win.get_viewport_x().clamp(-max_scroll, 0.0));
            win.set_viewport_y(0.0);
        }
        ScrollDirection::Vertical => {
            let max_scroll = (content_extent - visible_height).max(0) as f32;
            win.set_viewport_y(win.get_viewport_y().clamp(-max_scroll, 0.0));
            win.set_viewport_x(0.0);
        }
        ScrollDirection::None => {
            win.set_viewport_x(0.0);
            win.set_viewport_y(0.0);
        }
    }
}

fn sync_model_to_slint(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let mut s = state.borrow_mut();
    let show_footer = s.settings.show_window_info;
    let show_icons = s.settings.show_app_icons;
    let resize_locked = s.settings.locked_layout || s.settings.lock_cell_resize;

    // Populate cached icons lazily.
    if show_icons {
        for mw in &mut s.windows {
            populate_cached_icon(mw);
        }
    } else {
        for mw in &mut s.windows {
            mw.cached_icon = None;
        }
    }

    let data: Vec<ThumbnailData> = s
        .windows
        .iter()
        .map(|mw| {
            let accent = thumbnail_accent_color(&s.settings, &s.current_theme, &mw.info.app_id);
            let is_minimized = unsafe { IsIconic(mw.info.hwnd).as_bool() };
            ThumbnailData {
                x: mw.display_rect.left as f32,
                y: mw.display_rect.top as f32,
                width: (mw.display_rect.right - mw.display_rect.left) as f32,
                height: (mw.display_rect.bottom - mw.display_rect.top) as f32,
                title: SharedString::from(truncate_title(&mw.info.title)),
                app_label: SharedString::from(mw.info.app_label()),
                is_active: s.active_hwnd == Some(mw.info.hwnd),
                accent_color: accent,
                show_footer,
                is_minimized,
                icon: mw.cached_icon.clone().unwrap_or_default(),
                show_icon: show_icons,
            }
        })
        .collect();

    // Build resize handle data from cached separators.
    let handle_thickness: f32 = 14.0;
    let handles: Vec<ResizeHandleData> = if resize_locked {
        Vec::new()
    } else {
        s.separators
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
            .collect()
    };

    let dragging = s.drag_separator.is_some();
    let active_drag = s
        .drag_separator
        .as_ref()
        .map_or(-1, |d| d.separator_index as i32);

    win.set_layout_label(SharedString::from(s.current_layout.label()));
    win.set_window_count(s.windows.len() as i32);
    win.set_hidden_count(s.settings.hidden_app_entries().len() as i32);

    drop(s);
    win.set_thumbnails(ModelRc::new(VecModel::from(data)));

    // During a drag, update handle positions in-place via set_row_data so that
    // the Slint component instances (and their pointer capture) survive.
    if dragging {
        let model = win.get_resize_handles();
        let existing = model.row_count();
        for (idx, handle_data) in handles.into_iter().enumerate() {
            if idx < existing {
                model.set_row_data(idx, handle_data);
            }
        }
    } else {
        win.set_resize_handles(ModelRc::new(VecModel::from(handles)));
    }
    win.set_active_drag_index(active_drag);
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
        "m" | "M" => {
            open_application_context_menu(state, weak, None);
            true
        }
        "o" | "O" => {
            app::secondary_windows::open_settings_window(state, weak);
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
        "t" | "T" => {
            cycle_theme(state, weak);
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
    x: f64,
    y: f64,
) {
    let mut s = state.borrow_mut();
    if s.settings.lock_cell_resize || s.settings.locked_layout {
        s.drag_separator = None;
        return;
    }
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
        match s.current_layout.scroll_direction() {
            ScrollDirection::Vertical => f64::from(s.content_extent.max(logical_h)),
            _ => logical_h as f64,
        }
    } else {
        match s.current_layout.scroll_direction() {
            ScrollDirection::Horizontal => f64::from(s.content_extent.max(logical_w)),
            _ => logical_w as f64,
        }
    };

    // x, y are now absolute coordinates in content-frame space.
    let initial_offset = if sep.horizontal { y } else { x };

    s.drag_separator = Some(DragState {
        separator_index,
        horizontal: sep.horizontal,
        ratio_index: sep.ratio_index,
        axis_extent,
        last_pointer_offset: initial_offset,
    });
}

fn handle_resize_drag_move(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    separator_index: usize,
    x: f64,
    y: f64,
) {
    let (horizontal, ratio_index, axis_extent, last_pointer_offset) = {
        let s = state.borrow();
        if s.settings.lock_cell_resize || s.settings.locked_layout {
            return;
        }
        let Some(drag) = s.drag_separator.as_ref() else {
            return;
        };
        if drag.separator_index != separator_index || drag.axis_extent <= 0.0 {
            return;
        }
        (
            drag.horizontal,
            drag.ratio_index,
            drag.axis_extent,
            drag.last_pointer_offset,
        )
    };

    let handle_center = 7.0; // half of handle_thickness
    let _ = handle_center; // coordinates are now absolute in content-frame space
    let pointer_offset = if horizontal { y } else { x };
    let delta_frac = (pointer_offset - last_pointer_offset) / axis_extent;

    let mut s = state.borrow_mut();
    let layout = s.current_layout;
    ensure_custom_ratios(&mut s, layout);

    let min_frac = 0.03;
    if let Some(custom) = s.settings.layout_customizations.get_mut(layout.label()) {
        let ratios = if horizontal {
            &mut custom.row_ratios
        } else {
            &mut custom.col_ratios
        };
        if ratio_index + 1 < ratios.len() {
            match layout {
                LayoutType::Columns | LayoutType::Row | LayoutType::Column => {
                    apply_separator_drag_grouped(ratios, ratio_index, delta_frac, min_frac);
                }
                _ => apply_separator_drag(ratios, ratio_index, delta_frac, min_frac),
            }
        }
    }

    if let Some(drag) = s.drag_separator.as_mut() {
        if drag.separator_index == separator_index {
            drag.last_pointer_offset = pointer_offset;
        }
    }

    s.settings = s.settings.normalized();
    drop(s);

    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
    }
}

fn handle_resize_drag_end(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
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
        is_docked: state.is_appbar || state.settings.dock_edge.is_some(),
        show_toolbar: state.settings.show_toolbar,
        show_window_info: state.settings.show_window_info,
        show_app_icons: state.settings.show_app_icons,
        start_in_tray: state.settings.start_in_tray,
        locked_layout: state.settings.locked_layout,
        lock_cell_resize: state.settings.lock_cell_resize,
        group_windows_by: state.settings.group_windows_by,
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
            refresh_ui(state, weak);
        }
        TrayAction::ToggleDefaultAspectRatio => {
            update_settings(state, |s| {
                s.preserve_aspect_ratio = !s.preserve_aspect_ratio;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleDefaultHideOnSelect => {
            if state.borrow().settings.dock_edge.is_none() {
                update_settings(state, |s| {
                    s.hide_on_select = !s.hide_on_select;
                });
                refresh_ui(state, weak);
            }
        }
        TrayAction::ToggleAlwaysOnTop => {
            update_settings(state, |s| {
                s.always_on_top = !s.always_on_top;
            });
            let s = state.borrow();
            apply_topmost_mode(s.hwnd, s.settings.always_on_top);
            drop(s);
            refresh_ui(state, weak);
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
        TrayAction::SetWindowGrouping(grouping) => {
            update_settings(state, |s| {
                s.group_windows_by = grouping;
            });
            refresh_windows(state);
            refresh_ui(state, weak);
        }
        TrayAction::ToggleToolbar => {
            update_settings(state, |s| {
                s.show_toolbar = !s.show_toolbar;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleWindowInfo => {
            update_settings(state, |s| {
                s.show_window_info = !s.show_window_info;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleAppIcons => {
            update_settings(state, |s| {
                s.show_app_icons = !s.show_app_icons;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleStartInTray => {
            update_settings(state, |s| {
                s.start_in_tray = !s.start_in_tray;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleLockedLayout => {
            update_settings(state, |s| {
                s.locked_layout = !s.locked_layout;
            });
            refresh_ui(state, weak);
        }
        TrayAction::ToggleLockCellResize => {
            update_settings(state, |s| {
                s.lock_cell_resize = !s.lock_cell_resize;
            });
            refresh_ui(state, weak);
        }
        TrayAction::OpenSettingsWindow => {
            app::secondary_windows::open_settings_window(state, weak);
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

// ───────────────────────── Layout / State helpers ─────────────────────────

fn cycle_layout(state: &Rc<RefCell<AppState>>) {
    let mut s = state.borrow_mut();
    if s.settings.locked_layout {
        return;
    }
    s.current_layout = s.current_layout.next();
    s.settings.initial_layout = s.current_layout;
    s.drag_separator = None;
    let _ = s.settings.save(s.profile_name.as_deref());
}

fn cycle_theme(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    let current_idx = {
        let s = state.borrow();
        theme_catalog::theme_index(s.settings.theme_id.as_deref())
    };
    let total = theme_catalog::theme_labels().len() as i32;
    let next_idx = (current_idx + 1) % total;
    let new_id = theme_catalog::theme_id_by_index(next_idx);

    update_settings(state, |s| {
        s.theme_id = new_id;
    });

    let s = state.borrow();
    apply_window_appearance(s.hwnd, &s.settings);
    drop(s);

    refresh_ui(state, weak);
}

fn set_layout(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>, layout: LayoutType) {
    {
        let mut s = state.borrow_mut();
        if s.settings.locked_layout {
            return;
        }
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
        advance_theme_animation(state, &win);
    }
    app::secondary_windows::refresh_open_settings_window(state);
}

/// Schedule an immediate refresh + a deferred one (300 ms) so that
/// closed/killed windows disappear promptly even if the process takes
/// a moment to terminate.
fn schedule_deferred_refresh(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    let _ = refresh_windows(state);
    refresh_ui(state, weak);

    let state2 = state.clone();
    let weak2 = weak.clone();
    let timer = Timer::default();
    timer.start(
        TimerMode::SingleShot,
        Duration::from_millis(300),
        move || {
            if refresh_windows(&state2) {
                if let Some(win) = weak2.upgrade() {
                    recompute_and_update_ui(&state2, &win);
                }
            }
        },
    );
    // Intentional: the Slint event loop owns the timer until it fires;
    // dropping it here would cancel the callback. `forget` transfers
    // ownership to the event loop (no real leak for SingleShot timers).
    std::mem::forget(timer);
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
    let show_footer = s.settings.show_window_info;
    let show_icons = s.settings.show_app_icons;
    let model = win.get_thumbnails();
    let row_count = model.row_count();

    if row_count == s.windows.len() {
        for (i, mw) in s.windows.iter().enumerate() {
            if let Some(mut item) = model.row_data(i) {
                let accent = thumbnail_accent_color(&s.settings, &s.current_theme, &mw.info.app_id);
                let is_minimized = unsafe { IsIconic(mw.info.hwnd).as_bool() };
                item.x = mw.display_rect.left as f32;
                item.y = mw.display_rect.top as f32;
                item.width = (mw.display_rect.right - mw.display_rect.left) as f32;
                item.height = (mw.display_rect.bottom - mw.display_rect.top) as f32;
                item.title = SharedString::from(truncate_title(&mw.info.title));
                item.app_label = SharedString::from(mw.info.app_label());
                item.is_active = s.active_hwnd == Some(mw.info.hwnd);
                item.accent_color = accent;
                item.show_footer = show_footer;
                item.is_minimized = is_minimized;
                item.icon = mw.cached_icon.clone().unwrap_or_default();
                item.show_icon = show_icons;
                model.set_row_data(i, item);
            }
        }
        drop(s);
    } else {
        let data: Vec<ThumbnailData> = s
            .windows
            .iter()
            .map(|mw| {
                let accent = thumbnail_accent_color(&s.settings, &s.current_theme, &mw.info.app_id);
                let is_minimized = unsafe { IsIconic(mw.info.hwnd).as_bool() };
                ThumbnailData {
                    x: mw.display_rect.left as f32,
                    y: mw.display_rect.top as f32,
                    width: (mw.display_rect.right - mw.display_rect.left) as f32,
                    height: (mw.display_rect.bottom - mw.display_rect.top) as f32,
                    title: SharedString::from(truncate_title(&mw.info.title)),
                    app_label: SharedString::from(mw.info.app_label()),
                    is_active: s.active_hwnd == Some(mw.info.hwnd),
                    accent_color: accent,
                    show_footer,
                    is_minimized,
                    icon: mw.cached_icon.clone().unwrap_or_default(),
                    show_icon: show_icons,
                }
            })
            .collect();
        drop(s);
        win.set_thumbnails(ModelRc::new(VecModel::from(data)));
    }
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
        // SAFETY: hwnd is our live main window; bringing it to the foreground.
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
        for mw in &mut s.windows {
            release_thumbnail(mw);
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

fn toggle_toolbar_from_alt_hotkey() {
    UI_STATE.with(|state| {
        UI_WINDOW.with(|window| {
            let Some(state) = state.borrow().as_ref().cloned() else {
                return;
            };
            let Some(window) = window.borrow().as_ref().and_then(slint::Weak::upgrade) else {
                return;
            };
            {
                let mut guard = state.borrow_mut();
                guard.settings.show_toolbar = !guard.settings.show_toolbar;
                let _ = guard.settings.save(guard.profile_name.as_deref());
            }
            recompute_and_update_ui(&state, &window);
        });
    });
}

fn open_application_context_menu(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    coords: Option<(f32, f32)>,
) {
    let Some(win) = weak.upgrade() else { return };

    let (hwnd, anchor, menu_state) = {
        let mut guard = state.borrow_mut();
        if guard.hwnd.0.is_null() {
            return;
        }
        let anchor = coords.map(|(x, y)| {
            logical_to_screen_point(
                guard.hwnd,
                x * win.window().scale_factor(),
                y * win.window().scale_factor(),
            )
        });
        (guard.hwnd, anchor, build_tray_menu_state(&mut guard))
    };

    if let Some(action) = show_application_context_menu_at(hwnd, &menu_state, anchor) {
        handle_tray_action(state, weak, action);
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

fn logical_to_screen_point(hwnd: HWND, logical_x: f32, logical_y: f32) -> POINT {
    let mut window_rect = RECT::default();
    // SAFETY: hwnd is our live window; window_rect is stack-allocated and valid.
    unsafe {
        let _ = GetWindowRect(hwnd, &raw mut window_rect);
    }

    POINT {
        x: window_rect.left + logical_x.round() as i32,
        y: window_rect.top + logical_y.round() as i32,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dwm::sanitize_thumbnail_rect;
    use crate::app::icon::bilinear_sample_rgba;

    #[test]
    fn sanitize_thumbnail_rect_clips_to_client_bounds() {
        let (rect, visible) = sanitize_thumbnail_rect(
            RECT {
                left: -12,
                top: 10,
                right: 180,
                bottom: 140,
            },
            120,
            90,
        );

        assert!(visible);
        assert_eq!(rect.left, 0);
        assert_eq!(rect.top, 10);
        assert_eq!(rect.right, 120);
        assert_eq!(rect.bottom, 90);
    }

    #[test]
    fn sanitize_thumbnail_rect_hides_rects_outside_client() {
        let (rect, visible) = sanitize_thumbnail_rect(
            RECT {
                left: 300,
                top: 50,
                right: 360,
                bottom: 110,
            },
            200,
            120,
        );

        assert!(!visible);
        assert_eq!(rect, HIDDEN_THUMBNAIL_RECT);
    }

    #[test]
    fn bilinear_sample_rgba_preserves_transparent_edges() {
        let size = 4usize;
        let mut source = vec![0u8; size * size * 4];
        let center = (size + 1) * 4;
        source[center..center + 4].copy_from_slice(&[255, 128, 64, 255]);

        let sample = bilinear_sample_rgba(&source, size, 1.0, 1.0);

        assert_eq!(sample, [255, 128, 64, 255]);
        let transparent = bilinear_sample_rgba(&source, size, 0.0, 0.0);
        assert_eq!(transparent[3], 0);
    }

    #[test]
    fn apply_pinned_positions_keeps_pinned_app_in_reserved_slot() {
        let mut settings = AppSettings::default();
        let _ = settings.toggle_app_pinned_position("app:b", "B", 1);

        let mut windows = vec![
            WindowInfo {
                hwnd: HWND(std::ptr::dangling_mut::<c_void>()),
                title: "Alpha".to_owned(),
                app_id: "app:a".to_owned(),
                process_name: "A".to_owned(),
                process_path: None,
                class_name: "A".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
            WindowInfo {
                hwnd: HWND(2usize as *mut c_void),
                title: "Bravo".to_owned(),
                app_id: "app:b".to_owned(),
                process_name: "B".to_owned(),
                process_path: None,
                class_name: "B".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
            WindowInfo {
                hwnd: HWND(3usize as *mut c_void),
                title: "Charlie".to_owned(),
                app_id: "app:c".to_owned(),
                process_name: "C".to_owned(),
                process_path: None,
                class_name: "C".to_owned(),
                monitor_name: "DISPLAY1".to_owned(),
            },
        ];

        windows.swap(0, 1);
        apply_pinned_positions(&mut windows, &settings);

        assert_eq!(windows[1].app_id, "app:b");
    }
}
