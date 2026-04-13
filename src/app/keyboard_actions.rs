//! Keyboard shortcuts and theme-cycling actions.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;

use super::dock::apply_topmost_mode;
use super::layout_actions;
use super::secondary_windows;
use super::tray_actions;
use crate::{refresh_ui, refresh_windows, update_settings, AppState, MainWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    SetLayout(LayoutType),
    ResetLayout,
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    OpenMenu,
    OpenSettings,
    ToggleAlwaysOnTop,
    RefreshNow,
    CycleTheme,
    CycleLayout,
    Exit,
}

pub(crate) fn handle_key(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    key: &str,
) -> bool {
    let Some(action) = ({
        let state = state.borrow();
        matched_shortcut_action(&state.settings, key)
    }) else {
        return false;
    };

    match action {
        ShortcutAction::SetLayout(layout) => layout_actions::set_layout(state, weak, layout),
        ShortcutAction::ResetLayout => {
            layout_actions::reset_layout_custom(state);
            refresh_ui(state, weak);
        }
        ShortcutAction::ToggleAnimations => {
            update_settings(state, |settings| {
                settings.animate_transitions = !settings.animate_transitions;
            });
            refresh_ui(state, weak);
        }
        ShortcutAction::ToggleToolbar => {
            update_settings(state, |settings| {
                settings.show_toolbar = !settings.show_toolbar;
            });
            refresh_ui(state, weak);
        }
        ShortcutAction::ToggleWindowInfo => {
            update_settings(state, |settings| {
                settings.show_window_info = !settings.show_window_info;
            });
            refresh_ui(state, weak);
        }
        ShortcutAction::OpenMenu => tray_actions::open_application_context_menu(state, weak, None),
        ShortcutAction::OpenSettings => secondary_windows::open_settings_window(state, weak),
        ShortcutAction::ToggleAlwaysOnTop => {
            update_settings(state, |settings| {
                settings.always_on_top = !settings.always_on_top;
            });
            let state_ref = state.borrow();
            apply_topmost_mode(state_ref.hwnd, state_ref.settings.always_on_top);
            drop(state_ref);
            refresh_ui(state, weak);
        }
        ShortcutAction::RefreshNow => {
            if refresh_windows(state) {
                refresh_ui(state, weak);
            }
        }
        ShortcutAction::CycleTheme => cycle_theme(state, weak),
        ShortcutAction::CycleLayout => {
            layout_actions::cycle_layout(state);
            refresh_ui(state, weak);
        }
        ShortcutAction::Exit => crate::queue_exit_request(),
    }

    true
}

fn matched_shortcut_action(settings: &AppSettings, key: &str) -> Option<ShortcutAction> {
    let shortcuts = &settings.shortcuts;

    if shortcut_matches(&shortcuts.layout_grid, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Grid))
    } else if shortcut_matches(&shortcuts.layout_mosaic, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Mosaic))
    } else if shortcut_matches(&shortcuts.layout_bento, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Bento))
    } else if shortcut_matches(&shortcuts.layout_fibonacci, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Fibonacci))
    } else if shortcut_matches(&shortcuts.layout_columns, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Columns))
    } else if shortcut_matches(&shortcuts.layout_row, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Row))
    } else if shortcut_matches(&shortcuts.layout_column, key) {
        Some(ShortcutAction::SetLayout(LayoutType::Column))
    } else if shortcut_matches(&shortcuts.reset_layout, key) {
        Some(ShortcutAction::ResetLayout)
    } else if shortcut_matches(&shortcuts.toggle_animations, key) {
        Some(ShortcutAction::ToggleAnimations)
    } else if shortcut_matches(&shortcuts.toggle_toolbar, key) {
        Some(ShortcutAction::ToggleToolbar)
    } else if shortcut_matches(&shortcuts.toggle_window_info, key) {
        Some(ShortcutAction::ToggleWindowInfo)
    } else if shortcut_matches(&shortcuts.open_menu, key) {
        Some(ShortcutAction::OpenMenu)
    } else if shortcut_matches(&shortcuts.open_settings, key) {
        Some(ShortcutAction::OpenSettings)
    } else if shortcut_matches(&shortcuts.toggle_always_on_top, key) {
        Some(ShortcutAction::ToggleAlwaysOnTop)
    } else if shortcut_matches(&shortcuts.refresh_now, key) {
        Some(ShortcutAction::RefreshNow)
    } else if shortcut_matches(&shortcuts.cycle_theme, key) {
        Some(ShortcutAction::CycleTheme)
    } else if shortcut_matches(&shortcuts.cycle_layout, key) {
        Some(ShortcutAction::CycleLayout)
    } else if shortcut_matches(&shortcuts.exit_app, key) {
        Some(ShortcutAction::Exit)
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::{matched_shortcut_action, shortcut_matches, ShortcutAction};
    use panopticon::layout::LayoutType;
    use panopticon::settings::AppSettings;

    #[test]
    fn shortcut_matches_supports_named_special_keys() {
        assert!(shortcut_matches("Tab", "\t"));
        assert!(shortcut_matches("Esc", "\u{001B}"));
        assert!(shortcut_matches("Enter", "\n"));
        assert!(shortcut_matches("Space", " "));
        assert!(!shortcut_matches("Tab", "x"));
    }

    #[test]
    fn matched_shortcut_action_uses_current_bindings_without_cloning() {
        let mut settings = AppSettings::default();
        settings.shortcuts.layout_grid = "G".to_owned();
        settings.shortcuts.open_menu = "Q".to_owned();

        assert_eq!(
            matched_shortcut_action(&settings, "g"),
            Some(ShortcutAction::SetLayout(LayoutType::Grid))
        );
        assert_eq!(
            matched_shortcut_action(&settings, "Q"),
            Some(ShortcutAction::OpenMenu)
        );
        assert_eq!(matched_shortcut_action(&settings, "1"), None);
    }
}
