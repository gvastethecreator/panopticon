//! Keyboard shortcuts and theme-cycling actions.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;

use super::dock::apply_topmost_mode;
use super::layout_actions;
use super::secondary_windows;
use super::tray_actions;
use crate::{refresh_ui, refresh_windows, update_settings, AppState, MainWindow};

pub(crate) fn handle_key(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    key: &str,
) -> bool {
    let shortcuts = state.borrow().settings.shortcuts.clone();

    if shortcut_matches(&shortcuts.layout_grid, key) {
        layout_actions::set_layout(state, weak, LayoutType::Grid);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_mosaic, key) {
        layout_actions::set_layout(state, weak, LayoutType::Mosaic);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_bento, key) {
        layout_actions::set_layout(state, weak, LayoutType::Bento);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_fibonacci, key) {
        layout_actions::set_layout(state, weak, LayoutType::Fibonacci);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_columns, key) {
        layout_actions::set_layout(state, weak, LayoutType::Columns);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_row, key) {
        layout_actions::set_layout(state, weak, LayoutType::Row);
        return true;
    }
    if shortcut_matches(&shortcuts.layout_column, key) {
        layout_actions::set_layout(state, weak, LayoutType::Column);
        return true;
    }
    if shortcut_matches(&shortcuts.reset_layout, key) {
        layout_actions::reset_layout_custom(state);
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.toggle_animations, key) {
        update_settings(state, |settings| {
            settings.animate_transitions = !settings.animate_transitions;
        });
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.toggle_toolbar, key) {
        update_settings(state, |settings| {
            settings.show_toolbar = !settings.show_toolbar;
        });
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.toggle_window_info, key) {
        update_settings(state, |settings| {
            settings.show_window_info = !settings.show_window_info;
        });
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.open_menu, key) {
        tray_actions::open_application_context_menu(state, weak, None);
        return true;
    }
    if shortcut_matches(&shortcuts.open_settings, key) {
        secondary_windows::open_settings_window(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.toggle_always_on_top, key) {
        update_settings(state, |settings| {
            settings.always_on_top = !settings.always_on_top;
        });
        let state_ref = state.borrow();
        apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
        drop(state_ref);
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.refresh_now, key) {
        if refresh_windows(state) {
            refresh_ui(state, weak);
        }
        return true;
    }
    if shortcut_matches(&shortcuts.cycle_theme, key) {
        cycle_theme(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.cycle_layout, key) {
        layout_actions::cycle_layout(state);
        refresh_ui(state, weak);
        return true;
    }
    if shortcut_matches(&shortcuts.exit_app, key) {
        crate::queue_exit_request();
        return true;
    }

    false
}

fn shortcut_matches(binding: &str, key: &str) -> bool {
    match binding {
        "Tab" => key == "\t",
        "Esc" => key == "\u{001B}",
        "Enter" => key == "\n" || key == "\r",
        "Space" => key == " ",
        _ => key.eq_ignore_ascii_case(binding),
    }
}

fn cycle_theme(state: &Rc<RefCell<AppState>>, weak: &slint::Weak<MainWindow>) {
    let current_idx = {
        let state = state.borrow();
        panopticon::theme::theme_index(state.settings.theme_id.as_deref())
    };
    let total = panopticon::theme::theme_labels().len() as i32;
    let next_idx = (current_idx + 1) % total;
    let new_id = panopticon::theme::theme_id_by_index(next_idx);

    update_settings(state, |settings| {
        settings.theme_id = new_id;
    });

    let state_ref = state.borrow();
    super::dock::apply_window_appearance(state_ref.hwnd, &state_ref.settings);
    drop(state_ref);

    refresh_ui(state, weak);
}
