//! Effect planning for applying SettingsWindow changes to the runtime.

use panopticon::settings::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettingsApplyEffects {
    pub(crate) dock_changed: bool,
    pub(crate) locale_changed: bool,
    pub(crate) startup_changed: bool,
    pub(crate) hotkey_changed: bool,
    pub(crate) refresh_windows: bool,
    pub(crate) window_appearance: bool,
    pub(crate) recompute_ui: bool,
}

impl SettingsApplyEffects {
    #[must_use]
    pub(crate) fn plan(previous: &AppSettings, next: &AppSettings) -> Self {
        Self {
            dock_changed: previous.dock_edge != next.dock_edge,
            locale_changed: previous.language != next.language,
            startup_changed: previous.run_at_startup != next.run_at_startup,
            hotkey_changed: previous.shortcuts.global_activate != next.shortcuts.global_activate,
            refresh_windows: refresh_affecting_settings_changed(previous, next),
            window_appearance: appearance_affecting_settings_changed(previous, next),
            recompute_ui: previous != next,
        }
    }
}

fn refresh_affecting_settings_changed(previous: &AppSettings, next: &AppSettings) -> bool {
    previous.dock_edge != next.dock_edge
        || previous.group_windows_by != next.group_windows_by
        || previous.active_monitor_filter != next.active_monitor_filter
        || previous.active_tag_filter != next.active_tag_filter
        || previous.active_app_filter != next.active_app_filter
        || previous.app_rules != next.app_rules
}

fn appearance_affecting_settings_changed(previous: &AppSettings, next: &AppSettings) -> bool {
    previous.always_on_top != next.always_on_top
        || previous.theme_id != next.theme_id
        || previous.background_color_hex != next.background_color_hex
        || previous.theme_color_overrides != next.theme_color_overrides
        || previous.dock_edge != next.dock_edge
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon::settings::DockEdge;

    #[test]
    fn effects_flag_locale_dock_and_hotkey_changes_independently() {
        let previous = AppSettings::default();
        let mut next = previous.clone();
        next.language = panopticon::i18n::Locale::Spanish;
        next.dock_edge = Some(DockEdge::Left);
        next.shortcuts.global_activate = Some("Ctrl+Alt+K".to_owned());

        let effects = SettingsApplyEffects::plan(&previous, &next);

        assert!(effects.locale_changed);
        assert!(effects.dock_changed);
        assert!(effects.hotkey_changed);
        assert!(effects.refresh_windows);
        assert!(effects.window_appearance);
        assert!(effects.recompute_ui);
        assert!(!effects.startup_changed);
    }

    #[test]
    fn effects_do_not_refresh_windows_for_pure_colour_changes() {
        let previous = AppSettings::default();
        let mut next = previous.clone();
        next.background_color_hex = "112233".to_owned();

        let effects = SettingsApplyEffects::plan(&previous, &next);

        assert!(!effects.refresh_windows);
        assert!(effects.window_appearance);
        assert!(effects.recompute_ui);
    }
}
