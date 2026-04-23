//! Theme snapshot application, interpolation, and Slint globals sync.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, Model};

use panopticon::settings::AppSettings;
use panopticon::theme as theme_catalog;

use crate::{
    AppState, MainWindow, SettingsWindow, TagDialogWindow, Theme, ABOUT_WIN, SETTINGS_WIN,
    TAG_DIALOG_WIN, THEME_TRANSITION_DURATION_MS,
};

// ───────────────────────── Macro ─────────────────────────

macro_rules! apply_runtime_theme {
    ($window:expr, $resolved:expr) => {{
        let globals = $window.global::<Theme>();
        globals.set_bg(hex_to_slint_color(&$resolved.bg_hex));
        globals.set_toolbar_bg(hex_to_slint_color(&$resolved.toolbar_bg_hex));
        globals.set_panel_bg(hex_to_slint_color(&$resolved.panel_bg_hex));
        globals.set_card_bg(hex_to_slint_color(&$resolved.card_bg_hex));
        globals.set_border(hex_to_slint_color(&$resolved.border_hex));
        globals.set_accent(hex_to_slint_color(&$resolved.accent_hex));
        globals.set_accent_soft(hex_to_slint_color(&$resolved.accent_soft_hex));
        globals.set_text(hex_to_slint_color(&$resolved.text_hex));
        globals.set_label(hex_to_slint_color(&$resolved.label_hex));
        globals.set_muted(hex_to_slint_color(&$resolved.muted_hex));
        globals.set_hover_border(hex_to_slint_color(&$resolved.hover_border_hex));
        globals.set_placeholder(hex_to_slint_color(&$resolved.placeholder_hex));
        globals.set_footer_bg(hex_to_slint_color(&$resolved.footer_bg_hex));
        globals.set_surface(hex_to_slint_color(&$resolved.surface_hex));
    }};
}

// ───────────────────────── Snapshot apply ─────────────────────────

pub(crate) fn apply_main_window_theme_snapshot(
    window: &MainWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_settings_window_theme_snapshot(
    window: &SettingsWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_tag_dialog_theme_snapshot(
    window: &TagDialogWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_about_window_theme_snapshot(
    window: &crate::AboutWindow,
    resolved: &theme_catalog::UiTheme,
) {
    apply_runtime_theme!(window, resolved);
}

pub(crate) fn apply_theme_snapshot_everywhere(win: &MainWindow, resolved: &theme_catalog::UiTheme) {
    apply_main_window_theme_snapshot(win, resolved);
    SETTINGS_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            apply_settings_window_theme_snapshot(window, resolved);
        }
    });
    TAG_DIALOG_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            apply_tag_dialog_theme_snapshot(window, resolved);
        }
    });
    ABOUT_WIN.with(|handle| {
        if let Some(window) = handle.borrow().as_ref() {
            apply_about_window_theme_snapshot(window, resolved);
        }
    });
}

// ───────────────────────── Theme target + animation ─────────────────────────

pub(crate) fn sync_theme_target(state: &mut AppState) {
    let desired = theme_catalog::resolve_ui_theme(
        state.settings.theme_id.as_deref(),
        &state.settings.background_color_hex,
        &state.settings.theme_color_overrides,
    );
    let already_targeting = state
        .theme_animation
        .as_ref()
        .is_some_and(|animation| animation.to == desired);

    if already_targeting || state.current_theme == desired {
        return;
    }

    state.theme_animation = Some(crate::ThemeAnimation {
        from_rgb: theme_catalog::RgbThemeSnapshot::from_ui_theme(&state.current_theme),
        to_rgb: theme_catalog::RgbThemeSnapshot::from_ui_theme(&desired),
        to: desired,
        started_at: Instant::now(),
    });
}

pub(crate) fn advance_theme_animation(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let mut s = state.borrow_mut();
    let Some(ref animation) = s.theme_animation else {
        let current = s.current_theme.clone();
        drop(s);
        apply_theme_snapshot_everywhere(win, &current);
        refresh_thumbnail_accent_rows(state, win);
        return;
    };

    let elapsed_ms = animation.started_at.elapsed().as_millis() as u32;
    let progress = (elapsed_ms as f32 / THEME_TRANSITION_DURATION_MS as f32).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(3);
    let completed_theme = animation.to.clone();
    let resolved = animation
        .from_rgb
        .interpolate(&animation.to_rgb, eased, &animation.to);
    s.current_theme = resolved;
    if progress >= 1.0 {
        s.current_theme = completed_theme;
        s.theme_animation = None;
    }
    let current = s.current_theme.clone();
    drop(s);
    apply_theme_snapshot_everywhere(win, &current);
    refresh_thumbnail_accent_rows(state, win);
}

// ───────────────────────── Accent / color helpers ─────────────────────────

pub(crate) fn default_thumbnail_accent_color(
    settings: &AppSettings,
    theme: &theme_catalog::UiTheme,
) -> slint::Color {
    settings.active_tag_filter.as_deref().map_or_else(
        || hex_to_slint_color(&theme.accent_hex),
        |tag| hex_to_slint_color(&settings.tag_color_hex(tag)),
    )
}

pub(crate) fn thumbnail_accent_color(
    settings: &AppSettings,
    theme: &theme_catalog::UiTheme,
    app_id: &str,
) -> slint::Color {
    settings.app_color_hex(app_id).map_or_else(
        || default_thumbnail_accent_color(settings, theme),
        hex_to_slint_color,
    )
}

pub(crate) fn refresh_thumbnail_accent_rows(state: &Rc<RefCell<AppState>>, win: &MainWindow) {
    let s = state.borrow();
    let model = win.get_thumbnails();
    if model.row_count() != s.windows.len() {
        return;
    }

    for (index, managed_window) in s.windows.iter().enumerate() {
        if let Some(mut item) = model.row_data(index) {
            item.accent_color =
                thumbnail_accent_color(&s.settings, &s.current_theme, &managed_window.info.app_id);
            model.set_row_data(index, item);
        }
    }
}

pub(crate) fn hex_to_slint_color(hex: &str) -> slint::Color {
    let bytes = hex.as_bytes();
    let r = if bytes.len() >= 2 {
        hex_byte(bytes[0], bytes[1])
    } else {
        0xD2
    };
    let g = if bytes.len() >= 4 {
        hex_byte(bytes[2], bytes[3])
    } else {
        0x9A
    };
    let b = if bytes.len() >= 6 {
        hex_byte(bytes[4], bytes[5])
    } else {
        0x5C
    };
    slint::Color::from_rgb_u8(r, g, b)
}

#[inline]
fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

#[inline]
fn hex_byte(hi: u8, lo: u8) -> u8 {
    hex_nibble(hi) << 4 | hex_nibble(lo)
}
