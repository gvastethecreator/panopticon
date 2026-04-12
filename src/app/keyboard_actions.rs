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
    match key {
        "1" => {
            layout_actions::set_layout(state, weak, LayoutType::Grid);
            true
        }
        "2" => {
            layout_actions::set_layout(state, weak, LayoutType::Mosaic);
            true
        }
        "3" => {
            layout_actions::set_layout(state, weak, LayoutType::Bento);
            true
        }
        "4" => {
            layout_actions::set_layout(state, weak, LayoutType::Fibonacci);
            true
        }
        "5" => {
            layout_actions::set_layout(state, weak, LayoutType::Columns);
            true
        }
        "6" => {
            layout_actions::set_layout(state, weak, LayoutType::Row);
            true
        }
        "7" => {
            layout_actions::set_layout(state, weak, LayoutType::Column);
            true
        }
        "0" => {
            layout_actions::reset_layout_custom(state);
            refresh_ui(state, weak);
            true
        }
        "a" | "A" => {
            update_settings(state, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
            refresh_ui(state, weak);
            true
        }
        "h" | "H" => {
            update_settings(state, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            refresh_ui(state, weak);
            true
        }
        "i" | "I" => {
            update_settings(state, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
            refresh_ui(state, weak);
            true
        }
        "m" | "M" => {
            tray_actions::open_application_context_menu(state, weak, None);
            true
        }
        "o" | "O" => {
            secondary_windows::open_settings_window(state, weak);
            true
        }
        "p" | "P" => {
            update_settings(state, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            let state_ref = state.borrow();
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
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
            layout_actions::cycle_layout(state);
            refresh_ui(state, weak);
            true
        }
        "\u{001B}" => {
            crate::queue_exit_request();
            true
        }
        _ => false,
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
