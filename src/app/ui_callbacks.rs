//! Main-window callback wiring extracted from `main.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use super::actions::{dispatch_action, AppAction};
use crate::{AppState, MainWindow};

#[allow(clippy::too_many_lines)]
pub(crate) fn setup_callbacks(main_window: &MainWindow, state: &Rc<RefCell<AppState>>) {
    main_window.on_thumbnail_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index| {
            super::thumbnail_interactions::handle_thumbnail_click(&state, &weak, index as usize);
        }
    });

    main_window.on_thumbnail_right_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            super::thumbnail_interactions::handle_thumbnail_right_click(
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
            super::thumbnail_interactions::handle_thumbnail_drag_ended(
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
            super::thumbnail_interactions::handle_thumbnail_close(&state, &weak, index as usize);
        }
    });

    main_window.on_toolbar_clicked({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::CycleLayout);
        }
    });

    main_window.on_app_context_menu_requested({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |x, y, prefer_below| {
            super::tray_actions::open_application_context_menu(
                &state,
                &weak,
                Some((x, y)),
                prefer_below,
            );
        }
    });

    main_window.on_empty_open_settings({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::OpenSettingsWindowAt(None));
        }
    });

    main_window.on_empty_refresh_now({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::RefreshNow);
        }
    });

    main_window.on_empty_open_menu({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::OpenContextMenu);
        }
    });

    main_window.on_empty_clear_filters({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::ClearAllFilters);
        }
    });

    main_window.on_empty_show_hidden_apps({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::RestoreAllHidden);
        }
    });

    main_window.on_empty_dismiss_welcome({
        let state = state.clone();
        let weak = main_window.as_weak();
        move || {
            dispatch_action(&state, &weak, AppAction::DismissEmptyStateWelcome);
        }
    });

    main_window.on_resize_drag_started({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            super::layout_actions::handle_resize_drag_start(
                &state,
                &weak,
                index as usize,
                x as f64,
                y as f64,
            );
        }
    });

    main_window.on_resize_drag_moved({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |index, x, y| {
            super::layout_actions::handle_resize_drag_move(
                &state,
                &weak,
                index as usize,
                x as f64,
                y as f64,
            );
        }
    });

    main_window.on_resize_drag_ended({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |_index| {
            super::layout_actions::handle_resize_drag_end(&state, &weak);
        }
    });

    main_window.on_key_pressed({
        let state = state.clone();
        let weak = main_window.as_weak();
        move |key_text, shift_pressed| {
            super::keyboard_actions::handle_key(&state, &weak, &key_text, shift_pressed)
        }
    });
}
