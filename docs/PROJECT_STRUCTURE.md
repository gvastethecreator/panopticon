# Project Structure

This document describes how the repository is organised, what each folder is responsible for, and which pieces should be treated as source code, assets, documentation, or generated artefacts.

## General view

```text
panopticon/
+-- .github/
+-- assets/
+-- docs/
+-- installer/
+-- src/
+-- tests/
+-- ui/
+-- build.rs
+-- Cargo.toml
+-- README.md
+-- rust-toolchain.toml
+-- Justfile
+-- ...other root metadata files
```

Folders such as `target/`, `.opencode/`, `.agents/`, `.vscode/`, `logs/`, `temp/`, `docs/book/`, and `docs/panopticon_improvement_prd/` are not part of the public source tree; they are ignored by `.gitignore` (or are local-only working areas) and are not described below.

## Important root files

| File | Purpose |
| --- | --- |
| `Cargo.toml` | crate manifest, dependencies, profiles, and lints |
| `Cargo.lock` | locked dependency graph (committed) |
| `rust-toolchain.toml` | pinned Rust toolchain and required components |
| `build.rs` | compiles `ui/main.slint`, embeds the Windows icon, raises the main-thread stack |
| `README.md` | public-facing landing page |
| `docs/PRD.md` | product definition |
| `Justfile` | wrappers over local build, lint, test, doc, audit, and CI-equivalent commands |
| `CHANGELOG.md` | visible change history |
| `CONTRIBUTING.md` | contribution guide |
| `SECURITY.md` | security policy |
| `SUPPORT.md` | support channels and scope |
| `CODE_OF_CONDUCT.md` | collaboration rules |
| `AGENTS.md` | agent guidance (this repo's AI context block) |
| `CONTEXT.md` | canonical domain vocabulary |
| `rustfmt.toml` | Rust format style |
| `LICENSE` | project licence |
| `installer/panopticon.iss` | Inno Setup script for the Windows installer |

## Repository folders

### `src/`

The main source of the crate and binary. The `lib.rs` modules are the canonical public surface; the binary-only `main.rs` adds `mod state;` and the `app` helper tree.

#### Library modules (`src/`)

| Path | Role |
| --- | --- |
| `src/lib.rs` | crate library index; declares the public modules |
| `src/main.rs` | binary entry point; window loop, input, repaint, runtime state, top-level synchronisation |
| `src/constants.rs` | UI, animation, and truncation constants |
| `src/error.rs` | typed crate errors |
| `src/i18n.rs` | internationalisation (English / Spanish) |
| `src/input_ops.rs` | pure operations for keyboard and pointer input handling |
| `src/layout.rs` | pure, testable layout engine |
| `src/logging.rs` | `tracing` initialisation with rolling files |
| `src/settings.rs` | TOML persistence and configuration normalisation |
| `src/state.rs` | binary-only `AppState` and runtime state containers |
| `src/theme.rs` | theme catalogue and visual interpolation |
| `src/thumbnail.rs` | RAII wrapper for DWM thumbnails |
| `src/ui_option_ops.rs` | pure operations for Slint option models |
| `src/window_enum.rs` | Win32 window enumeration and metadata extraction |
| `src/window_ops.rs` | pure operations for window collection transformations |
| `src/workspace.rs` | workspace CRUD, loading, and selection logic |

#### Subfolder `src/app/`

Groups binary-side helpers oriented towards the UX. The folder is flat in `mod.rs` and re-exports both leaf modules and a few facade submodules.

| Path | Role |
| --- | --- |
| `src/app/mod.rs` | binary helper index (declares the submodules below) |
| `src/app/actions.rs` | shared dispatcher for runtime actions triggered by keyboard, tray, and command palette |
| `src/app/action_execution.rs` | executes a dispatched `Action` against the current `AppState` |
| `src/app/action_handlers.rs` | per-category action handlers used by the dispatcher |
| `src/app/animation_engine.rs` | layout/thumbnail animation ticker and easing helpers |
| `src/app/cache.rs` | icon and metadata caches |
| `src/app/cli.rs` | command-line argument parsing, help text, and version output |
| `src/app/command_palette/` | searchable command launcher (catalog + module wiring) |
| `src/app/dock.rs` | appbar/dock mode behaviour and resize rules |
| `src/app/dwm.rs` | DWM attribute helpers shared with `thumbnail.rs` |
| `src/app/global_hotkey.rs` | `RegisterHotKey` registration and routing |
| `src/app/icon.rs` | icon resolution glue shared by tray and main window |
| `src/app/keyboard_actions.rs` | keyboard shortcut resolution routed through the shared dispatcher |
| `src/app/layout_actions.rs` | layout-switch and separator-reset actions |
| `src/app/layout_pipeline.rs` | sequencing between refresh, layout, model sync, and animation |
| `src/app/managed_window_lifecycle.rs` | preview and DWM thumbnail lifecycle for `ManagedWindow` |
| `src/app/managed_window_reconcile.rs` | reconcile `WindowInfo` enumeration against existing `ManagedWindow`s |
| `src/app/menu_utils.rs` | shared helpers for native popup menus |
| `src/app/model_sync.rs` | derives Slint-facing view models and empty-state context |
| `src/app/native_events.rs` | Win32 message routing for the main and tray HWNDs |
| `src/app/native_runtime.rs` | shared native runtime handles (DWM, hotkey, tray, subclass) |
| `src/app/presentation.rs` | Slint presentation: background, backdrop, theme transition |
| `src/app/runtime_effects.rs` | settings-backed runtime effect seam |
| `src/app/runtime_loop.rs` | close-request wiring plus recurring Slint timers for native init retries, UI refresh, and scrollbar idle handling |
| `src/app/runtime_support.rs` | shared runtime helpers re-exported by `main.rs` |
| `src/app/secondary_windows.rs` | facade for settings/about/tag secondary windows; re-exports `dialogs`, `placement`, and `settings_window` |
| `src/app/settings/` | `SettingsWindow` data binding, callback modules, and pure helpers |
| `src/app/settings_state.rs` | persisted snapshot plus runtime projections for `SettingsWindow` |
| `src/app/shell_state.rs` | shell-level runtime state shared with `main.rs` |
| `src/app/startup.rs` | deferred native initialisation after the Slint window appears |
| `src/app/theme_state.rs` | theme catalogue state and transitions |
| `src/app/theme_ui.rs` | theme application helpers for Slint surfaces |
| `src/app/thumbnail_interactions.rs` | activation, click, and drag handling for thumbnail cards |
| `src/app/thumbnail_model_builder.rs` | builds the Slint thumbnail model from `ManagedWindow`s |
| `src/app/tick_phases.rs` | per-tick phase ordering used by `runtime_loop` |
| `src/app/tray.rs` | tray runtime facade re-exported as `app::tray`; aggregates `icons`, `menu`, `notify` |
| `src/app/tray/icons.rs` | icon loading, generation, and resolution |
| `src/app/tray/menu.rs` | native popup-menu construction and `TrayAction` decoding |
| `src/app/tray/notify.rs` | `Shell_NotifyIconW` registration/update/remove wrapper |
| `src/app/tray_actions.rs` | tray action handling routed through shared runtime dispatch |
| `src/app/ui_callbacks.rs` | extracted `MainWindow` callback wiring |
| `src/app/ui_translations.rs` | translation/global text population extracted from `main.rs` |
| `src/app/updates.rs` | update check facade and integration |
| `src/app/viewport_manager.rs` | overlay scrollbar, viewport, and overflow state |
| `src/app/window_collection.rs` | `ManagedWindow` collection helpers |
| `src/app/window_menu.rs` | per-window context menu |
| `src/app/window_subclass.rs` | `SetWindowLongPtrW`-based subclass for tray, hotkey, and scroll messages |
| `src/app/window_sync.rs` | keeps `AppState` and Slint thumbnail model in sync |
| `src/app/workspace.rs` | workspace CRUD, loading, switching, and new-instance helpers |

### `ui/`

Contains the declarative Slint UI. The file tree is split by surface so each window/component can be edited in isolation.

| Path | Role |
| --- | --- |
| `ui/main.slint` | root Slint entry compiled by `build.rs`; declares `MainWindow` and shared theme tokens |
| `ui/common.slint` | shared structs, palettes, and helpers reused by all surfaces |
| `ui/components/` | reusable widgets: `custom_themed_widgets`, `empty_state`, `overlay_scrollbar`, `resize_handle`, `thumbnail_card`, `toolbar` |
| `ui/windows/` | secondary windows: `about_window`, `command_palette_window`, `settings_window`, `tag_dialog_window` |

This is a key folder: any visual structure or declarative binding change goes through here.

### `tests/`

Project integration tests. Each file exercises a pure operation surface so it can run without Win32.

| Path | Role |
| --- | --- |
| `tests/input_ops_tests.rs` | pure keyboard and pointer input operations |
| `tests/layout_tests.rs` | layout engine: overflow, counts, ratios, separators |
| `tests/ui_option_ops_tests.rs` | Slint option model helpers |
| `tests/window_ops_tests.rs` | `WindowInfo` / `ManagedWindow` collection transforms |

Additionally, there are unit tests embedded in `src/settings.rs` and `src/theme.rs`.

### `assets/`

Assets consumed at runtime, bundled in the installer, or referenced from the docs. Files in `assets/fonts/` are the bundled **Miranda Sans** family and ship with their `LICENSE-OFL.txt`; see `assets/fonts/README.md`.

| Path | Role |
| --- | --- |
| `assets/themes.json` | base theme catalogue used by `src/theme.rs` |
| `assets/icon.ico`, `assets/icon.svg`, `assets/icon-{xs,m,xl}.png` | application icons (ICO, SVG, PNG fallbacks) |
| `assets/mascot.svg`, `assets/pano-black.svg` | first-party illustrations used by the UI and docs |
| `assets/disk.webp` | decorative image used in the README and docs |
| `assets/logos/` | third-party logo marks referenced from the docs (Slint, Rust, Windows) |
| `assets/ui-icons/` | UI iconography bundle (HugeIcons, see README credits) |
| `assets/fonts/` | bundled Miranda Sans family with the OFL 1.1 license text |

### `docs/`

Technical and product documentation for the project.

| Path | Focus |
| --- | --- |
| `docs/README.md` | documentation hub and reading guide |
| `docs/project-readiness.md` | latest maintenance/readiness baseline and validation record |
| `docs/technical-debt.md` | active technical-debt register and deferred follow-ups |
| `docs/GETTING_STARTED.md` | install, launch, first-run flow, common issues |
| `docs/CONFIGURATION.md` | settings, workspaces, and TOML schema |
| `docs/ARCHITECTURE.md` | architecture, runtime layers, and diagrams |
| `docs/IMPLEMENTATION.md` | module-level implementation details |
| `docs/PROJECT_STRUCTURE.md` | this document |
| `docs/SYSTEM_INTEGRATIONS.md` | Win32/DWM/Shell/GDI usage and operational constraints |
| `docs/UX_DESIGN.md` | user-facing surfaces, interactions, and visual language |
| `docs/PRD.md` | product goals, scope, users, constraints, acceptance criteria |
| `docs/adr/` | architectural decision records (ADRs) |
| `docs/agents/` | agent-skill configuration (backlog, domain, triage labels) |
| `docs/assets/` | graphic resources used by the documentation |

The folders `docs/book/` and `docs/panopticon_improvement_prd/` are local-only planning material and are excluded by `.gitignore`.

### Maintainer scripts

There is no tracked `scripts/` directory at the moment. The release flow is driven by `.github/workflows/release.yml` on tag push, and version bumps are made directly in `Cargo.toml` + `docs/PRD.md` + `CHANGELOG.md`.

### `installer/`

| Path | Role |
| --- | --- |
| `installer/panopticon.iss` | Inno Setup script used by the release workflow to build the Windows installer |

### `.github/`

Project conventions for collaboration, CI, and automation.

- CI and release workflows;
- dependabot config;
- issue templates and PR template;
- Copilot instructions for the in-browser coding agent;
- funding configuration.

## Conceptual code organisation

The project can be understood in five groups:

1. **Technical domain core**  
   `src/layout.rs`, `src/settings.rs`, `src/theme.rs`, `src/workspace.rs`, plus the `*_ops` pure-operation modules (`input_ops`, `ui_option_ops`, `window_ops`).
2. **Win32 / DWM interop**  
   `src/window_enum.rs`, `src/thumbnail.rs`, large parts of `src/main.rs`, plus `src/app/dwm.rs`, `src/app/native_events.rs`, `src/app/native_runtime.rs`, `src/app/window_subclass.rs`, `src/app/tray.rs`, and `src/app/window_menu.rs`.
3. **UI layer**  
   `ui/main.slint`, `ui/common.slint`, `ui/components/*`, `ui/windows/*`, `src/app/model_sync.rs`, `src/app/presentation.rs`, `src/app/ui_callbacks.rs`, `src/app/ui_translations.rs`, and the `src/app/settings/` data-binding tree.
4. **Runtime orchestration**  
   `src/main.rs`, `src/state.rs`, `src/app/actions.rs`, `src/app/action_execution.rs`, `src/app/action_handlers.rs`, `src/app/runtime_loop.rs`, `src/app/tick_phases.rs`, `src/app/runtime_support.rs`, `src/app/runtime_effects.rs`, `src/app/secondary_windows.rs`, `src/app/workspace.rs`.
5. **Quality and support**  
   `tests/`, `docs/`, `src/logging.rs`, `src/error.rs`, `src/i18n.rs`.

## Which folders are sources of truth

### Yes: edit normally

- `src/`
- `ui/`
- `tests/`
- `docs/`
- `assets/`
- `installer/`
- root files like `Cargo.toml`, `README.md`, `AGENTS.md`, `CONTEXT.md`, and `rust-toolchain.toml`

### No: generated, transient, or local-only

- `target/` — Cargo build outputs.
- `.vscode/`, `.idea/`, `.opencode/`, `.agents/`, `.local/` — local IDE or agent state.
- `docs/book/`, `docs/panopticon_improvement_prd/` — private planning material (ignored).
- Root `*.log` files, `temp/`, `smoke_stdout.txt`, `smoke_stderr.txt` — ignored runtime/test artefacts.

## Key paths by type of change

| If you want to change... | Start with... |
| --- | --- |
| layout behaviour | `src/layout.rs` + `tests/layout_tests.rs` |
| persistence or settings | `src/settings.rs` + `src/app/settings/` + `src/app/settings_state.rs` |
| main visual UX | `ui/main.slint` + `ui/components/*` + `src/app/model_sync.rs` |
| secondary windows (settings/about/tag/command palette) | `src/app/secondary_windows.rs` + `ui/windows/*` |
| window enumeration | `src/window_enum.rs` + `src/app/managed_window_reconcile.rs` |
| DWM thumbnails | `src/thumbnail.rs` + `src/app/dwm.rs` + `src/app/managed_window_lifecycle.rs` |
| tray and quick menus | `src/app/tray.rs` + `src/app/tray_actions.rs` |
| per-window menu | `src/app/window_menu.rs` |
| theming | `src/theme.rs` + `src/app/theme_state.rs` + `assets/themes.json` |
| internationalisation | `src/i18n.rs` + `src/app/ui_translations.rs` + `src/app/settings/translations.rs` |
| documentation | `README.md`, `PRD.md`, `docs/*.md` |

## Important structural observations

- `main.rs` is still the file with the most concentrated responsibility in the project, although callback wiring, action dispatch, settings data binding, and tray handling have already been extracted into `src/app/*` helpers.
- `src/layout.rs`, `src/settings.rs`, and `src/theme.rs` are the cleanest and most decoupled pieces and are the best entry points for new contributors.
- The declarative UI is now split across `ui/main.slint`, `ui/common.slint`, `ui/components/`, and `ui/windows/` to keep each surface focused.
- The `src/app/` tree is large and intentionally flat at the top level: the `mod.rs` re-exports leaf modules and a few facades (`tray`, `secondary_windows`, `settings`) so callers do not need to know the internal split.
- `target/`, `temp/`, and root `*.log` files can be voluminous; they should not be confused with maintainable product code.
