//! Visual theme and animation state.
//!
//! Groups the fields that describe the current UI theme, any in-progress
//! theme transition, layout animation timing, and the loaded background image.

use std::time::Instant;

use panopticon::theme as theme_catalog;

use crate::ThemeAnimation;

/// The subset of [`AppState`] that deals with visual appearance.
pub(crate) struct ThemeState {
    pub(crate) current_theme: theme_catalog::UiTheme,
    pub(crate) theme_animation: Option<ThemeAnimation>,
    pub(crate) animation_started_at: Option<Instant>,
    pub(crate) loaded_background_path: Option<String>,
}

impl ThemeState {
    pub(crate) fn new(initial_theme: theme_catalog::UiTheme) -> Self {
        Self {
            current_theme: initial_theme,
            theme_animation: None,
            animation_started_at: None,
            loaded_background_path: None,
        }
    }
}
