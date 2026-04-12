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

pub(crate) use app::model_sync::{
    advance_animation, recompute_and_update_ui, sync_model_to_slint, sync_settings_to_ui,
};

use app::dock::{
    apply_dock_mode, apply_topmost_mode, apply_window_appearance, reposition_appbar,
    sync_dock_system_menu, unregister_appbar,
};
use app::dwm::{
    ensure_thumbnail, query_source_size, release_all_thumbnails, release_thumbnail,
    update_dwm_thumbnails,
};
use app::theme_ui::{advance_theme_animation, apply_main_window_theme_snapshot};
use app::tray::{apply_window_icons, AppIcons, TrayAction, TrayIcon, INSTANCE_ACCENT_PALETTE};
use panopticon::constants::TOOLBAR_HEIGHT;
use panopticon::layout::{
    apply_separator_drag, apply_separator_drag_grouped, default_ratios, LayoutType,
    ScrollDirection, Separator,
};
use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;
use panopticon::thumbnail::Thumbnail;
use panopticon::window_enum::{enumerate_windows, WindowInfo};
use panopticon::window_ops::{apply_pinned_positions, sort_windows_for_grouping};

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::{CloseRequestResponse, ComponentHandle, SharedString, Timer, TimerMode};

use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT, RECT, SIZE};

use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

slint::include_modules!();

// ───────────────────────── Constants ─────────────────────────

/// Callback message posted by the shell when the app-bar needs repositioning.
pub(crate) const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

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
            app::window_subclass::hide_scrollbar_if_idle(&weak);
        }
    });

    tracing::info!("entering Slint event loop");
    if let Err(error) = slint::run_event_loop_until_quit() {
        tracing::error!(%error, "Slint event loop failed");
    }
    let hwnd = state.borrow().hwnd;
    if !hwnd.0.is_null() {
        app::window_subclass::teardown_subclass(hwnd);
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
    app::window_subclass::setup_subclass(hwnd, state, win);

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
        move |x, y| app::tray_actions::open_application_context_menu(&state, &weak, Some((x, y)))
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
            app::tray_actions::open_application_context_menu(state, weak, None);
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

fn handle_pending_action(state: &Rc<RefCell<AppState>>, win: &MainWindow, action: PendingAction) {
    let weak = win.as_weak();
    match action {
        PendingAction::Tray(ta) => app::tray_actions::handle_tray_action(state, &weak, ta),
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

pub(crate) fn cycle_layout(state: &Rc<RefCell<AppState>>) {
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

pub(crate) fn update_settings(
    state: &Rc<RefCell<AppState>>,
    mutate: impl FnOnce(&mut AppSettings),
) {
    let mut s = state.borrow_mut();
    mutate(&mut s.settings);
    s.settings = s.settings.normalized();
    let _ = s.settings.save(s.profile_name.as_deref());
}

pub(crate) fn refresh_ui(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    if let Some(win) = weak.upgrade() {
        recompute_and_update_ui(state, &win);
        advance_theme_animation(state, &win);
    }
    app::secondary_windows::refresh_open_settings_window(state);
}

/// Schedule an immediate refresh + a deferred one (300 ms) so that
/// closed/killed windows disappear promptly even if the process takes
/// a moment to terminate.
pub(crate) fn schedule_deferred_refresh(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
) {
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

pub(crate) fn logical_to_screen_point(hwnd: HWND, logical_x: f32, logical_y: f32) -> POINT {
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

pub(crate) fn queue_exit_request() {
    PENDING_ACTIONS.with(|queue| queue.borrow_mut().push(PendingAction::Exit));
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
