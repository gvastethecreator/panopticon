#![windows_subsystem = "windows"]
#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::wildcard_imports
)]

//! Binary entry point for Panopticon — Slint UI with DWM thumbnail overlays.

mod app;
mod state;

// Re-export all public state types and thread-locals so that `crate::AppState`,
// `crate::UI_STATE`, etc. continue to resolve without changing every consumer.
pub(crate) use state::*;

use app::cli::{cli_usage, parse_startup_args};
use app::model_sync::sync_settings_to_ui;
use app::runtime_support::request_update_check;
use app::ui_callbacks::setup_callbacks;
use app::ui_translations::populate_tr_global;
use app::theme_ui::apply_main_window_theme_snapshot;
use app::tray::{AppIcons, INSTANCE_ACCENT_PALETTE};
use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::w;


use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

slint::include_modules!();

// ───────────────────────── Constants ─────────────────────────

/// Callback message posted by the shell when the app-bar needs repositioning.
pub(crate) const WM_APPBAR_CALLBACK: u32 = WM_APP + 2;

static TASKBAR_CREATED_MSG: AtomicU32 = AtomicU32::new(0);

// ───────────────────────── Entry Point ─────────────────────────

#[cfg(target_os = "windows")]
fn select_text_friendly_renderer() {
    let renderer_selection = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("skia".into())
        .select();

    match renderer_selection {
        Ok(()) => {
            tracing::info!(
                "selected Slint winit backend with Skia renderer for sharper Windows text"
            );
        }
        Err(error) => {
            tracing::warn!(
                %error,
                "failed to select Slint Skia renderer; falling back to default backend selection"
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn select_text_friendly_renderer() {}

#[allow(clippy::too_many_lines)]
fn main() {
    let _ = panopticon::i18n::set_locale(panopticon::i18n::Locale::English);
    let startup_args = match parse_startup_args() {
        Ok(startup_args) => startup_args,
        Err(error) => StartupArgs::PrintAndExit {
            message: format!("{error}\n\n{}", cli_usage()),
            stderr: true,
        },
    };

    match startup_args {
        StartupArgs::Run { workspace } => run_app(workspace),
        StartupArgs::PrintAndExit { message, stderr } => {
            if stderr {
                eprintln!("{message}");
            } else {
                println!("{message}");
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run_app(workspace: Option<String>) {
    let _log_guard = panopticon::logging::init().ok();
    select_text_friendly_renderer();

    tracing::info!(workspace = ?workspace, "Panopticon starting (Slint UI)");

    // SAFETY: FFI call with no preconditions; failure is non-fatal.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
        TASKBAR_CREATED_MSG.store(taskbar_msg, Ordering::Relaxed);
    }

    let icons = match workspace.as_deref() {
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
    let settings = AppSettings::load_or_default(workspace.as_deref()).unwrap_or_else(|error| {
        tracing::error!(%error, "settings load failed; using defaults");
        AppSettings::default()
    });
    app::startup::sync_run_at_startup(settings.run_at_startup, workspace.as_deref());
    panopticon::i18n::init(settings.language);
    app::secondary_windows::ensure_default_workspaces_exist(&settings);

    let initial_theme = theme_catalog::resolve_ui_theme(
        settings.theme_id.as_deref(),
        &settings.background_color_hex,
        &settings.theme_color_overrides,
    );

    let main_window = match MainWindow::new() {
        Ok(window) => window,
        Err(error) => {
            tracing::error!(%error, "failed to create main window");
            return;
        }
    };
    populate_tr_global(&main_window);
    apply_main_window_theme_snapshot(&main_window, &initial_theme);

    // Apply initial property values from settings.
    sync_settings_to_ui(&main_window, &settings);

    let state = Rc::new(RefCell::new(AppState {
        shell: app::shell_state::ShellState::new(icons),
        window_collection: app::window_collection::WindowCollection::new(
            settings.effective_layout(),
        ),
        theme: app::theme_state::ThemeState::new(initial_theme),
        settings,
        workspace_name: workspace,
        app_version: format!("v{}", env!("CARGO_PKG_VERSION")),
        update_status: UpdateStatus::Idle,
    }));

    // Show the window so the native HWND exists on next event-loop iteration.
    if let Err(error) = main_window.show() {
        tracing::error!(%error, "failed to show main window");
        return;
    }

    // Slint callbacks (don't need HWND — they use state internally).
    setup_callbacks(&main_window, &state);

    let _ = request_update_check(&state, false);

    app::runtime_loop::install_close_behavior(&main_window, &state);
    let _runtime_loop = app::runtime_loop::start(&main_window, &state);

    tracing::info!("entering Slint event loop");
    if let Err(error) = slint::run_event_loop_until_quit() {
        tracing::error!(%error, "Slint event loop failed");
    }
    let hwnd = state.borrow().shell.hwnd;
    if !hwnd.0.is_null() {
        app::window_subclass::teardown_subclass(hwnd);
    }
    tracing::info!("Panopticon exiting");
}


