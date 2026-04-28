//! Keyboard shortcuts and theme-cycling actions.

use std::cell::RefCell;
use std::rc::Rc;

use panopticon::layout::LayoutType;
use panopticon::settings::AppSettings;

use super::actions::{dispatch_action, AppAction};
use crate::{AppState, MainWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    SetLayout(LayoutType),
    ResetLayout,
    ToggleAnimations,
    ToggleToolbar,
    ToggleWindowInfo,
    OpenMenu,
    OpenSettings,
    OpenCommandPalette,
    ToggleAlwaysOnTop,
    RefreshNow,
    CycleTheme,
    CycleThemePrevious,
    CycleLayout,
    Exit,
}

pub(crate) fn handle_key(
    state: &Rc<RefCell<AppState>>,
    weak: &slint::Weak<MainWindow>,
    key: &str,
    shift_pressed: bool,
) -> bool {
    let Some(action) = ({
        let state = state.borrow();
        matched_shortcut_action(&state.settings, key, shift_pressed)
    }) else {
        return false;
    };

    match action {
        ShortcutAction::SetLayout(layout) => {
            dispatch_action(state, weak, AppAction::SetLayout(layout));
        }
        ShortcutAction::ResetLayout => dispatch_action(state, weak, AppAction::ResetLayoutRatios),
        ShortcutAction::ToggleAnimations => {
            dispatch_action(state, weak, AppAction::ToggleAnimations);
        }
        ShortcutAction::ToggleToolbar => dispatch_action(state, weak, AppAction::ToggleToolbar),
        ShortcutAction::ToggleWindowInfo => {
            dispatch_action(state, weak, AppAction::ToggleWindowInfo);
        }
        ShortcutAction::OpenMenu => dispatch_action(state, weak, AppAction::OpenContextMenu),
        ShortcutAction::OpenSettings => {
            dispatch_action(state, weak, AppAction::OpenSettingsWindowAt(None));
        }
        ShortcutAction::OpenCommandPalette => {
            dispatch_action(state, weak, AppAction::OpenCommandPalette);
        }
        ShortcutAction::ToggleAlwaysOnTop => {
            dispatch_action(state, weak, AppAction::ToggleAlwaysOnTop);
        }
        ShortcutAction::RefreshNow => dispatch_action(state, weak, AppAction::RefreshNow),
        ShortcutAction::CycleTheme => {
            dispatch_action(state, weak, AppAction::CycleTheme { direction: 1 });
        }
        ShortcutAction::CycleThemePrevious => {
            dispatch_action(state, weak, AppAction::CycleTheme { direction: -1 });
        }
        ShortcutAction::CycleLayout => dispatch_action(state, weak, AppAction::CycleLayout),
        ShortcutAction::Exit => dispatch_action(state, weak, AppAction::Exit),
    }

    true
}

fn matched_shortcut_action(
    settings: &AppSettings,
    key: &str,
    shift_pressed: bool,
) -> Option<ShortcutAction> {
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
    } else if shortcut_matches(&shortcuts.toggle_always_on_top, key) {
        Some(ShortcutAction::ToggleAlwaysOnTop)
    } else if shortcut_matches(&shortcuts.open_settings, key) {
        Some(ShortcutAction::OpenSettings)
    } else if shortcut_matches(&shortcuts.open_command_palette, key) {
        Some(ShortcutAction::OpenCommandPalette)
    } else if shortcut_matches(&shortcuts.refresh_now, key) {
        Some(ShortcutAction::RefreshNow)
    } else if shortcut_matches(&shortcuts.cycle_theme, key) {
        Some(if shift_pressed {
            ShortcutAction::CycleThemePrevious
        } else {
            ShortcutAction::CycleTheme
        })
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
            matched_shortcut_action(&settings, "g", false),
            Some(ShortcutAction::SetLayout(LayoutType::Grid))
        );
        assert_eq!(
            matched_shortcut_action(&settings, "Q", false),
            Some(ShortcutAction::OpenMenu)
        );
        assert_eq!(matched_shortcut_action(&settings, "1", false), None);
    }

    #[test]
    fn shift_plus_cycle_theme_goes_backwards() {
        let settings = AppSettings::default();

        assert_eq!(
            matched_shortcut_action(&settings, "T", true),
            Some(ShortcutAction::CycleThemePrevious)
        );
    }
}
