//! Workspace file-store for persisted Panopticon settings snapshots.

use std::path::PathBuf;

use crate::error::{PanopticonError, Result};
use crate::i18n;
use crate::settings::{validate_workspace_name_input, AppSettings, WorkspaceNameValidation};

const WORKSPACE_METADATA_SCHEMA_VERSION: u32 = 1;

pub const DEFAULT_WORKSPACE_LABEL: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkspaceTarget {
    Default,
    Named(String),
}

impl WorkspaceTarget {
    #[must_use]
    pub fn from_optional_name(workspace: Option<&str>) -> Self {
        workspace
            .filter(|name| !name.trim().is_empty())
            .map_or(Self::Default, |name| Self::Named(name.trim().to_owned()))
    }

    #[must_use]
    pub const fn as_name(&self) -> Option<&str> {
        match self {
            Self::Default => None,
            Self::Named(name) => Some(name.as_str()),
        }
    }

    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Default => DEFAULT_WORKSPACE_LABEL.to_owned(),
            Self::Named(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceStore {
    base_dir: PathBuf,
}

impl WorkspaceStore {
    #[must_use]
    pub fn from_appdata() -> Self {
        let base_dir = std::env::var_os("APPDATA")
            .map_or_else(|| std::env::temp_dir().join("Panopticon"), PathBuf::from)
            .join("Panopticon");
        Self { base_dir }
    }

    #[must_use]
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    #[must_use]
    pub fn settings_path(&self, target: &WorkspaceTarget) -> PathBuf {
        match target {
            WorkspaceTarget::Default => self.base_dir.join("settings.toml"),
            WorkspaceTarget::Named(name) => self
                .base_dir
                .join("workspaces")
                .join(format!("{name}.toml")),
        }
    }

    #[must_use]
    pub fn path_for(&self, workspace: Option<&str>) -> PathBuf {
        self.settings_path(&WorkspaceTarget::from_optional_name(workspace))
    }

    /// Return known saved named workspaces discovered on disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace directory exists but cannot be enumerated.
    pub fn list_named(&self) -> Result<Vec<String>> {
        let workspaces_dir = self.base_dir.join("workspaces");
        if !workspaces_dir.exists() {
            return Ok(Vec::new());
        }

        let mut workspaces = Vec::new();
        for entry in std::fs::read_dir(workspaces_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                if let Some(workspace) = crate::settings::normalize_workspace_name(stem) {
                    workspaces.push(workspace);
                }
            }
        }
        workspaces.sort();
        workspaces.dedup();
        Ok(workspaces)
    }

    /// Return all workspace labels, always including the implicit default one.
    ///
    /// # Errors
    ///
    /// Returns an error when the named-workspace directory cannot be enumerated.
    pub fn list_with_default(&self) -> Result<Vec<String>> {
        let mut workspaces = self.list_named()?;
        workspaces.insert(0, DEFAULT_WORKSPACE_LABEL.to_owned());
        workspaces.dedup();
        Ok(workspaces)
    }

    #[must_use]
    pub fn exists(&self, workspace: Option<&str>) -> bool {
        self.path_for(workspace).exists()
    }

    /// Load settings from disk, returning defaults if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load_settings(&self, workspace: Option<&str>) -> Result<AppSettings> {
        let path = self.path_for(workspace);
        if !path.exists() {
            return Ok(AppSettings::default());
        }

        let contents = std::fs::read_to_string(path)?;
        let settings: AppSettings = toml::from_str(&contents)
            .map_err(|error| PanopticonError::SettingsParse(error.to_string()))?;
        Ok(settings.normalized())
    }

    /// Persist settings to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the settings directory cannot be created or if the
    /// TOML payload cannot be serialized.
    pub fn save_settings(&self, workspace: Option<&str>, settings: &AppSettings) -> Result<()> {
        let path = self.path_for(workspace);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut normalized = settings.normalized();
        normalized.workspace.touch_for_save();

        let toml = toml::to_string_pretty(&normalized)
            .map_err(|error| PanopticonError::SettingsParse(error.to_string()))?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    /// Duplicate an existing workspace snapshot into a new named workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if source loading fails, the target name is invalid,
    /// or the target workspace already exists.
    pub fn duplicate(&self, source: Option<&str>, target: &str) -> Result<()> {
        let target = validate_named_workspace(
            target,
            "cannot duplicate into reserved workspace name 'default'",
        )?;
        if self.exists(Some(&target)) {
            return Err(PanopticonError::SettingsParse(format!(
                "workspace '{target}' already exists"
            )));
        }

        let mut settings = self.load_settings(source)?;
        settings.workspace.created_unix_ms = None;
        settings.workspace.updated_unix_ms = None;
        settings.workspace.schema_version = Some(WORKSPACE_METADATA_SCHEMA_VERSION);
        if settings.workspace.display_name.trim().is_empty() {
            settings.workspace.display_name.clone_from(&target);
        }
        self.save_settings(Some(&target), &settings)
    }

    /// Rename a named workspace file.
    ///
    /// # Errors
    ///
    /// Returns an error if source/target names are invalid, if source does not
    /// exist, or if target already exists.
    pub fn rename(&self, source: &str, target: &str) -> Result<()> {
        let source = validate_named_workspace(source, "the default workspace cannot be renamed")?;
        let target =
            validate_named_workspace(target, "cannot rename to reserved workspace name 'default'")?;

        let source_path = self.path_for(Some(&source));
        if !source_path.exists() {
            return Err(PanopticonError::SettingsParse(format!(
                "workspace '{source}' does not exist"
            )));
        }

        let target_path = self.path_for(Some(&target));
        if target_path.exists() {
            return Err(PanopticonError::SettingsParse(format!(
                "workspace '{target}' already exists"
            )));
        }

        std::fs::rename(source_path, &target_path)?;

        let mut updated = self.load_settings(Some(&target))?;
        if updated.workspace.display_name.trim().is_empty()
            || updated.workspace.display_name.eq_ignore_ascii_case(&source)
        {
            updated.workspace.display_name.clone_from(&target);
        }
        updated.workspace.schema_version = Some(WORKSPACE_METADATA_SCHEMA_VERSION);
        self.save_settings(Some(&target), &updated)
    }

    /// Delete a named workspace file.
    ///
    /// # Errors
    ///
    /// Returns an error if the workspace name is invalid/reserved or when file
    /// removal fails.
    pub fn delete(&self, workspace: &str) -> Result<()> {
        let workspace =
            validate_named_workspace(workspace, "the default workspace cannot be deleted")?;
        let path = self.path_for(Some(&workspace));
        if !path.exists() {
            return Err(PanopticonError::SettingsParse(format!(
                "workspace '{workspace}' does not exist"
            )));
        }

        std::fs::remove_file(path)?;
        Ok(())
    }
}

fn validate_named_workspace(input: &str, reserved_message: &str) -> Result<String> {
    let workspace = match validate_workspace_name_input(input) {
        WorkspaceNameValidation::Valid(workspace_name) => workspace_name,
        WorkspaceNameValidation::Empty => {
            return Err(PanopticonError::SettingsParse(
                i18n::t("settings.workspace_empty_name").to_owned(),
            ));
        }
        WorkspaceNameValidation::Invalid(reason) => {
            return Err(PanopticonError::SettingsParse(reason));
        }
    };

    if workspace.eq_ignore_ascii_case(DEFAULT_WORKSPACE_LABEL) {
        return Err(PanopticonError::SettingsParse(reserved_message.to_owned()));
    }
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> WorkspaceStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        WorkspaceStore::with_base_dir(
            std::env::temp_dir().join(format!("panopticon-workspace-store-test-{unique}")),
        )
    }

    #[test]
    fn workspace_paths_match_persisted_layout() {
        let store = WorkspaceStore::with_base_dir("C:/Users/demo/AppData/Roaming/Panopticon");

        assert_eq!(
            store.path_for(None),
            PathBuf::from("C:/Users/demo/AppData/Roaming/Panopticon/settings.toml")
        );
        assert_eq!(
            store.path_for(Some("studio")),
            PathBuf::from("C:/Users/demo/AppData/Roaming/Panopticon/workspaces/studio.toml")
        );
    }

    #[test]
    fn list_named_ignores_non_toml_and_sorts_unique_normalized_names() {
        let store = temp_store();
        let workspaces_dir = store.base_dir.join("workspaces");
        std::fs::create_dir_all(&workspaces_dir).expect("workspace dir should be creatable");
        std::fs::write(workspaces_dir.join("Beta.toml"), "").expect("toml file should write");
        std::fs::write(workspaces_dir.join("alpha.toml"), "").expect("toml file should write");
        std::fs::write(workspaces_dir.join("ignored.txt"), "").expect("txt file should write");

        assert_eq!(
            store.list_named().expect("workspaces should list"),
            vec!["Beta".to_owned(), "alpha".to_owned()]
        );

        let _ = std::fs::remove_dir_all(store.base_dir);
    }

    #[test]
    fn save_load_roundtrip_preserves_settings_schema() {
        let store = temp_store();
        let settings = AppSettings {
            show_toolbar: false,
            workspace: crate::settings::WorkspaceMetadata {
                display_name: "Studio".to_owned(),
                description: "Primary workspace".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        };

        store
            .save_settings(Some("studio"), &settings)
            .expect("settings should save");
        let loaded = store
            .load_settings(Some("studio"))
            .expect("settings should load");

        assert!(!loaded.show_toolbar);
        assert_eq!(loaded.workspace.display_name, "Studio");
        assert_eq!(loaded.workspace.description, "Primary workspace");
        assert!(loaded.workspace.updated_unix_ms.is_some());

        let _ = std::fs::remove_dir_all(store.base_dir);
    }

    #[test]
    fn duplicate_rename_and_delete_named_workspace() {
        let store = temp_store();
        let settings = AppSettings {
            workspace: crate::settings::WorkspaceMetadata {
                display_name: "Source".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        };
        store
            .save_settings(Some("source"), &settings)
            .expect("source should save");

        store
            .duplicate(Some("source"), "copy")
            .expect("workspace should duplicate");
        assert!(store.exists(Some("copy")));

        store
            .rename("copy", "renamed")
            .expect("workspace should rename");
        assert!(!store.exists(Some("copy")));
        assert!(store.exists(Some("renamed")));

        store.delete("renamed").expect("workspace should delete");
        assert!(!store.exists(Some("renamed")));

        let _ = std::fs::remove_dir_all(store.base_dir);
    }

    #[test]
    fn named_workspace_operations_reject_default() {
        let store = temp_store();

        assert!(store.duplicate(None, "default").is_err());
        assert!(store.rename("default", "next").is_err());
        assert!(store.delete("default").is_err());
    }
}
