//! Persisted settings snapshot plus runtime-derived settings state.

use std::ops::Deref;

use panopticon::layout::LayoutType;
use panopticon::settings::{parse_global_hotkey_binding, AppSettings, GlobalHotkeyBinding};

use crate::app::settings::apply_effects::SettingsApplyEffects;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeSettings {
    pub(crate) effective_layout: LayoutType,
    pub(crate) global_activate_binding: Option<GlobalHotkeyBinding>,
}

impl RuntimeSettings {
    fn derive(settings: &AppSettings) -> Self {
        Self {
            effective_layout: settings.effective_layout(),
            global_activate_binding: settings
                .shortcuts
                .global_activate
                .as_deref()
                .and_then(parse_global_hotkey_binding),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsChange {
    pub(crate) previous: AppSettings,
    pub(crate) next: AppSettings,
    pub(crate) effects: SettingsApplyEffects,
}

#[derive(Debug, Clone)]
pub(crate) struct SettingsState {
    persisted: AppSettings,
    runtime: RuntimeSettings,
}

impl SettingsState {
    pub(crate) fn new(persisted: &AppSettings) -> Self {
        let persisted = persisted.normalized();
        let runtime = RuntimeSettings::derive(&persisted);
        Self { persisted, runtime }
    }

    pub(crate) fn snapshot(&self) -> AppSettings {
        self.persisted.clone()
    }

    pub(crate) const fn persisted(&self) -> &AppSettings {
        &self.persisted
    }

    pub(crate) const fn runtime(&self) -> &RuntimeSettings {
        &self.runtime
    }

    pub(crate) fn replace_persisted(&mut self, next: &AppSettings) -> Option<SettingsChange> {
        let next = next.normalized();
        if next == self.persisted {
            return None;
        }

        let previous = self.persisted.clone();
        let effects = SettingsApplyEffects::plan(&previous, &next);
        self.persisted = next.clone();
        self.runtime = RuntimeSettings::derive(&self.persisted);

        Some(SettingsChange {
            previous,
            next,
            effects,
        })
    }

    pub(crate) fn update_persisted(
        &mut self,
        mutate: impl FnOnce(&mut AppSettings),
    ) -> Option<SettingsChange> {
        let mut next = self.persisted.clone();
        mutate(&mut next);
        self.replace_persisted(&next)
    }

    pub(crate) fn save(&self, workspace_name: Option<&str>) -> panopticon::error::Result<()> {
        self.persisted.save(workspace_name)
    }
}

impl Deref for SettingsState {
    type Target = AppSettings;

    fn deref(&self) -> &Self::Target {
        &self.persisted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use panopticon::layout::LayoutType;
    use panopticon::settings::DockEdge;

    #[test]
    fn settings_state_derives_runtime_layout_and_hotkey_binding() {
        let settings = AppSettings {
            dock_edge: Some(DockEdge::Left),
            shortcuts: panopticon::settings::ShortcutBindings {
                global_activate: Some("Ctrl+Alt+K".to_owned()),
                ..Default::default()
            },
            ..AppSettings::default()
        };

        let state = SettingsState::new(&settings);

        assert_eq!(state.runtime().effective_layout, LayoutType::Column);
        assert!(state.runtime().global_activate_binding.is_some());
    }

    #[test]
    fn replace_persisted_reports_changes_and_refreshes_runtime_projection() {
        let defaults = AppSettings::default();
        let mut state = SettingsState::new(&defaults);

        let next = AppSettings {
            dock_edge: Some(DockEdge::Top),
            ..AppSettings::default()
        };
        let change = state.replace_persisted(&next);

        assert!(change.is_some());
        assert_eq!(state.runtime().effective_layout, LayoutType::Row);
    }

    #[test]
    fn update_persisted_returns_none_when_mutation_is_effectively_noop() {
        let defaults = AppSettings::default();
        let mut state = SettingsState::new(&defaults);

        let change = state.update_persisted(|settings| {
            settings.refresh_interval_ms = 2_000;
        });

        assert!(change.is_none());
    }
}
