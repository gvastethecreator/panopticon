# Project Structure

This document describes how the repository is organised, what each folder is responsible for, and which pieces should be treated as source code, assets, documentation, or generated artefacts.

## General view

```text
panopticon/
+-- .agents/
+-- .github/
+-- assets/
+-- docs/
+-- src/
+-- tests/
+-- ui/
+-- build.rs
+-- Cargo.toml
+-- README.md
+-- PRD.md
+-- Justfile
+-- ...other support files
```

## Important root files

| File | Purpose |
| --- | --- |
| `Cargo.toml` | crate manifest, dependencies, profiles, and lints |
| `build.rs` | compiles `ui/main.slint` during the build |
| `README.md` | main project entry point |
| `PRD.md` | updated product definition |
| `Justfile` | auxiliary project commands |
| `CHANGELOG.md` | visible change history |
| `CONTRIBUTING.md` | contribution guide |
| `SECURITY.md` | security policy |
| `SUPPORT.md` | support channels and scope |
| `rustfmt.toml` | Rust format style |
| `LICENSE` | project licence |

## Repository folders

### `src/`

The main source of the crate and binary.

#### Files in `src/`

| Path | Role |
| --- | --- |
| `src/lib.rs` | crate library index; re-exports base modules |
| `src/main.rs` | main binary; startup, event loop, Win32 interop, timer orchestration, and top-level synchronisation |
| `src/constants.rs` | UI, animation, and truncation constants |
| `src/error.rs` | typed crate errors |
| `src/i18n.rs` | internationalisation (English / Spanish) |
| `src/layout.rs` | pure, testable layout engine |
| `src/logging.rs` | `tracing` initialisation with rolling files |
| `src/settings.rs` | TOML persistence and configuration normalisation |
| `src/theme.rs` | theme catalogue and visual interpolation |
| `src/thumbnail.rs` | RAII wrapper for DWM thumbnails |
| `src/window_enum.rs` | Win32 window enumeration and metadata extraction |

#### Subfolder `src/app/`

Groups helpers oriented towards the binary UX.

| Path | Role |
| --- | --- |
| `src/app/mod.rs` | binary helper index |
| `src/app/actions.rs` | shared dispatcher for runtime actions triggered by keyboard, tray, and command palette |
| `src/app/command_palette.rs` | searchable command launcher wired to shared actions |
| `src/app/keyboard_actions.rs` | keyboard shortcut resolution routed through the shared dispatcher |
| `src/app/model_sync.rs` | derives Slint-facing view models and empty-state context |
| `src/app/secondary_windows.rs` | facade for settings/about/tag/workspace secondary windows |
| `src/app/secondary_windows/settings_callbacks.rs` | callback wiring for `SettingsWindow`, including workspace/app-rules/shortcut/background actions |
| `src/app/secondary_windows/placement.rs` | owner resolution, centering, and z-order helpers for secondary windows |
| `src/app/secondary_windows/dialogs.rs` | About/Tag dialog lifecycle and callbacks |
| `src/app/secondary_windows/workspace.rs` | workspace CRUD/load/switch/new-instance helpers |
| `src/app/settings_ui.rs` | bridge between `AppSettings` and `SettingsWindow` |
| `src/app/tray_runtime.rs` | tray runtime facade re-exported as `app::tray` for callers |
| `src/app/tray_runtime/icons.rs` | icon loading/generation/resolution for the main window and tray |
| `src/app/tray_runtime/menu.rs` | native popup-menu construction and `TrayAction` decoding |
| `src/app/tray_runtime/notify.rs` | `Shell_NotifyIconW` registration/update/remove wrapper |
| `src/app/tray_actions.rs` | tray action handling routed through shared runtime dispatch |
| `src/app/ui_callbacks.rs` | extracted `MainWindow` callback wiring |
| `src/app/ui_translations.rs` | translation/global text population extracted from `main.rs` |
| `src/app/window_menu.rs` | per-window context menu |

### `ui/`

Contains the declarative Slint UI.

| Path | Role |
| --- | --- |
| `ui/main.slint` | definition of `MainWindow`, `SettingsWindow`, `TagDialogWindow`, cards, toolbar, scrollbars, and visual components |

This is a key folder: any visual structure or declarative binding change goes through here.

### `tests/`

Project integration tests.

| Path | Role |
| --- | --- |
| `tests/layout_tests.rs` | validates the layout engine, overflow, counts, ratios, and separators |

Additionally, there are unit tests embedded in `src/settings.rs` and `src/theme.rs`.

### `assets/`

Assets consumed at runtime or during visual documentation.

| Path | Role |
| --- | --- |
| `assets/themes.json` | base theme catalogue used by `src/theme.rs` |

### `docs/`

Technical and product documentation for the project.

| Path | Focus |
| --- | --- |
| `docs/GETTING_STARTED.md` | installation and initial flow |
| `docs/ARCHITECTURE.md` | architecture, layers, and diagrams |
| `docs/CONFIGURATION.md` | persistent configuration and profiles |
| `docs/PROJECT_STRUCTURE.md` | repository map |
| `docs/IMPLEMENTATION.md` | internal details per module and runtime |
| `docs/SYSTEM_INTEGRATIONS.md` | APIs, dependencies, and system services |
| `docs/UX_DESIGN.md` | experience design and visual components |
| `docs/assets/` | graphic resources used by the documentation |

### `.github/`

Project conventions for collaboration, CI, and automation.

- CI workflows;
- issue/PR templates if present;
- agent-specific instructions.

### `.agents/`

Repository-specific instructions and skills for assistants/automation. They are not part of the final product, but rather part of its assisted maintenance layer.

### `target/`

Artefacts generated by Cargo.

Includes:

- debug/release binaries;
- compiled dependencies;
- incremental builds;
- coverage if `tarpaulin` is run;
- test and doc artefacts.

**Must not be edited manually.**

### `doc/`

HTML documentation generated by `cargo doc`. This is a generated artefact, not a primary maintenance source.

### `temp/`

Auxiliary workspace folder. Currently contains temporary files and operational notes, so it should be treated as support material rather than core source code.

## Conceptual code organisation

The project can be understood in five groups:

1. **Technical domain core**  
   `layout.rs`, `settings.rs`, `theme.rs`
2. **Win32/DWM interop**  
   `window_enum.rs`, `thumbnail.rs`, large parts of `main.rs`, `tray_runtime.rs`, `window_menu.rs`
3. **UI layer**  
   `ui/main.slint`, `settings_ui.rs`
4. **Runtime orchestration**  
   `main.rs`, `app/actions.rs`, `app/ui_callbacks.rs`, `app/secondary_windows.rs`
5. **Quality and support**  
   `tests/`, `docs/`, `logging.rs`, `error.rs`

## Which folders are sources of truth

### Yes: edit normally

- `src/`
- `ui/`
- `tests/`
- `docs/`
- `assets/`
- root files like `Cargo.toml`, `README.md`, `PRD.md`

### No: generated or transient

- `target/`
- `doc/`
- some `temp/` content if only outputs or work notes

## Key paths by type of change

| If you want to change... | Start with... |
| --- | --- |
| layout behaviour | `src/layout.rs` + `tests/layout_tests.rs` |
| persistence or settings | `src/settings.rs` + `src/app/settings_ui.rs` |
| main visual UX | `ui/main.slint` |
| window enumeration | `src/window_enum.rs` |
| DWM thumbnails | `src/thumbnail.rs` + `src/main.rs` |
| tray and quick menus | `src/app/tray_runtime.rs` + `src/app/tray_actions.rs` |
| per-window menu | `src/app/window_menu.rs` |
| theming | `src/theme.rs` + `assets/themes.json` |
| internationalisation | `src/i18n.rs` |
| documentation | `README.md`, `PRD.md`, `docs/*.md` |

## Important structural observations

- `main.rs` is still the file with the most concentrated responsibility in the project, although callback wiring and action dispatch have started moving into `src/app/*` helpers.
- `layout.rs` is the cleanest and most decoupled piece.
- the declarative UI is centralised in a single Slint file, which simplifies searching but can grow significantly over time.
- `target/` and `doc/` can be voluminous; they should not be confused with maintainable product code.
