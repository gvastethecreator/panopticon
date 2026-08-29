# Code map · panopticon

generated: 2026-08-29T02:54:13Z
commit: 228fdfe8cba6
scope: .

counts: 20 nodes · 46 edges · 0 flows · 0 unknown

## Modules

- `external-dependencies` · `src/app/action_execution.rs` · external · External
  callers: other-modules (imports), src (imports), src-app-command-palette (imports), src-app-dock (imports), src-app-dwm (imports), src-app-model-sync (imports), src-app-native-runtime (imports), src-app-presentation (imports), src-app-secondary-windows-dialogs (imports), src-app-secondary-windows-settings-window (imports), src-app-settings (imports), src-app-settings-callbacks-app-rules (imports), src-app-settings-callbacks-runtime (imports), src-app-settings-preset-sync (imports), src-app-settings-ui (imports), src-app-thumbnail-interactions (imports), src-app-tray-actions (imports), src-app-workspace (imports)
  callees: (none)
  tests: (none)
  entry: src/app/action_execution.rs:panopticon

- `other-modules` · `src/app/cli.rs` · module · Other Modules
  callers: src-app (imports), src-app-command-palette (imports), src-app-settings (imports)
  callees: external-dependencies (imports), src-app (imports), src-app-settings-callbacks-app-rules (imports), src-app-settings-callbacks-runtime (imports)
  tests: (none)
  entry: src/app/cli.rs:parse_startup_args_from

- `src` · `src` · module · Src
  callers: (none)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/lib.rs://! # Panopticon

- `src-app` · `src/app` · module · Src
  callers: other-modules (imports), src (imports), src-app-command-palette (imports), src-app-model-sync (imports), src-app-secondary-windows-dialogs (imports), src-app-secondary-windows-settings-window (imports), src-app-settings-callbacks-app-rules (imports), src-app-settings-callbacks-runtime (imports), src-app-settings-preset-sync (imports), src-app-tray-actions (imports), src-app-workspace (imports)
  callees: other-modules (imports), src-app-command-palette (imports), src-app-dock (imports), src-app-dwm (imports), src-app-model-sync (imports), src-app-native-runtime (imports), src-app-presentation (imports), src-app-settings (imports), src-app-thumbnail-interactions (imports), src-app-tray-actions (imports), src-app-workspace (imports)
  tests: (none)
  entry: src/app/mod.rs://! Binary-only application helpers.

- `src-app-command-palette` · `src/app/command_palette` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports), other-modules (imports), src-app (imports)
  tests: (none)
  entry: src/app/command_palette/mod.rs:rebuild_filtered_commands

- `src-app-dock` · `src/app/dock.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/dock.rs:fn

- `src-app-dwm` · `src/app/dwm.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/dwm.rs:enter

- `src-app-model-sync` · `src/app/model_sync.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/model_sync.rs:enter

- `src-app-native-runtime` · `src/app/native_runtime.rs` · service · Src
  callers: src-app (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/native_runtime.rs:configured_floating_window_size

- `src-app-presentation` · `src/app/presentation.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/presentation.rs:derive_empty_state_context

- `src-app-secondary-windows-dialogs` · `src/app/secondary_windows/dialogs.rs` · module · Src
  callers: (none)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/secondary_windows/dialogs.rs:sync_about_window_from_state

- `src-app-secondary-windows-settings-window` · `src/app/secondary_windows/settings_window.rs` · module · Src
  callers: (none)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/secondary_windows/settings_window.rs:enter

- `src-app-settings` · `src/app/settings` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports), other-modules (imports), src-app-settings-preset-sync (imports), src-app-settings-ui (imports)
  tests: (none)
  entry: src/app/settings/mod.rs:parse_rgb_hex

- `src-app-settings-callbacks-app-rules` · `src/app/settings/callbacks/app_rules.rs` · module · Src
  callers: other-modules (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/settings/callbacks/app_rules.rs:register_app_rules_select_app_callback

- `src-app-settings-callbacks-runtime` · `src/app/settings/callbacks/runtime.rs` · service · Src
  callers: other-modules (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/settings/callbacks/runtime.rs:register_open_about_callback

- `src-app-settings-preset-sync` · `src/app/settings/preset_sync.rs` · module · Src
  callers: src-app-settings (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/settings/preset_sync.rs:persistence_feedback

- `src-app-settings-ui` · `src/app/settings/ui.rs` · interface · Src
  callers: src-app-settings (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/settings/ui.rs:MAX_THEME_PREVIEW_CARDS

- `src-app-thumbnail-interactions` · `src/app/thumbnail_interactions.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports)
  tests: (none)
  entry: src/app/thumbnail_interactions.rs:KillProcessError

- `src-app-tray-actions` · `src/app/tray_actions.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/tray_actions.rs:app_action_for_tray_action

- `src-app-workspace` · `src/app/workspace.rs` · module · Src
  callers: src-app (imports)
  callees: external-dependencies (imports), src-app (imports)
  tests: (none)
  entry: src/app/workspace.rs:workspace_status_summary

## Edges

- other-modules -> external-dependencies · imports
- other-modules -> src-app · imports
- other-modules -> src-app-settings-callbacks-app-rules · imports
- other-modules -> src-app-settings-callbacks-runtime · imports
- src -> external-dependencies · imports
- src -> src-app · imports
- src-app -> other-modules · imports
- src-app -> src-app-command-palette · imports
- src-app -> src-app-dock · imports
- src-app -> src-app-dwm · imports
- src-app -> src-app-model-sync · imports
- src-app -> src-app-native-runtime · imports
- src-app -> src-app-presentation · imports
- src-app -> src-app-settings · imports
- src-app -> src-app-thumbnail-interactions · imports
- src-app -> src-app-tray-actions · imports
- src-app -> src-app-workspace · imports
- src-app-command-palette -> external-dependencies · imports
- src-app-command-palette -> other-modules · imports
- src-app-command-palette -> src-app · imports
- src-app-dock -> external-dependencies · imports
- src-app-dwm -> external-dependencies · imports
- src-app-model-sync -> external-dependencies · imports
- src-app-model-sync -> src-app · imports
- src-app-native-runtime -> external-dependencies · imports
- src-app-presentation -> external-dependencies · imports
- src-app-secondary-windows-dialogs -> external-dependencies · imports
- src-app-secondary-windows-dialogs -> src-app · imports
- src-app-secondary-windows-settings-window -> external-dependencies · imports
- src-app-secondary-windows-settings-window -> src-app · imports
- src-app-settings -> external-dependencies · imports
- src-app-settings -> other-modules · imports
- src-app-settings -> src-app-settings-preset-sync · imports
- src-app-settings -> src-app-settings-ui · imports
- src-app-settings-callbacks-app-rules -> external-dependencies · imports
- src-app-settings-callbacks-app-rules -> src-app · imports
- src-app-settings-callbacks-runtime -> external-dependencies · imports
- src-app-settings-callbacks-runtime -> src-app · imports
- src-app-settings-preset-sync -> external-dependencies · imports
- src-app-settings-preset-sync -> src-app · imports
- src-app-settings-ui -> external-dependencies · imports
- src-app-thumbnail-interactions -> external-dependencies · imports
- src-app-tray-actions -> external-dependencies · imports
- src-app-tray-actions -> src-app · imports
- src-app-workspace -> external-dependencies · imports
- src-app-workspace -> src-app · imports

## Unknown

- none

## Flows

- none
