//! Settings-window view-model data shared by the Slint adapter and Settings modules.

use panopticon::settings::{AppSelectionEntry, HiddenAppEntry};

pub(crate) struct RuntimeUiOptions {
    pub(crate) monitors: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) apps: Vec<AppSelectionEntry>,
    pub(crate) hidden_apps: Vec<HiddenAppEntry>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "UI filters need explicit boolean flags to drive quick predicates without extra allocations"
)]
pub(crate) struct AppRuleListEntry {
    pub(crate) option: AppSelectionEntry,
    pub(crate) is_running: bool,
    pub(crate) has_saved_rule: bool,
    pub(crate) is_hidden: bool,
    pub(crate) has_tags: bool,
    pub(crate) has_custom_refresh: bool,
    pub(crate) is_pinned: bool,
    pub(crate) searchable_blob: String,
}
