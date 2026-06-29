# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Changed

- Updated the Rust dependency baseline to Slint 1.17.0 and refreshed the Cargo lockfile.
- Added a pinned Rust 1.96.0 toolchain with clippy/rustfmt components for reproducible local and CI validation.
- Aligned `Justfile`, CI, and contributor docs around locked builds, full-target tests, release builds, rustdoc, and dependency audit.

### Fixed

- Reconciled stale documentation references to Slint 1.15.1, a missing docs preview asset, and untracked VS Code task wrappers.

## [0.1.0] - 2026-06-02

The "definitive public launch" milestone. This release re-aligns the on-disk version number with the `0.1.0` tag for public consumption; the codebase is functionally identical to the preceding `0.1.21` private iteration, with a small set of polish and hygiene changes layered on top.

> If you arrived here from `0.1.21`, `0.1.2`, or `0.1.1`: nothing in your saved data or settings has changed. The version number on this release is a rebrand, not a regression. The four earlier `v0.1.x` GitHub releases remain published for historical reference.

### Added in 0.1.0

- A bilingual UI with English as the default locale and Spanish as a bundled alternative, including localised CLI help, error messages, native dialog titles, and a language selector in settings.
- A `src/app/actions.rs` `AppAction` enum (36 variants) plus `dispatch_action` as the single shared entry point used by shortcuts, tray, command palette, and Slint callbacks; the action/effect layer is now the canonical wiring seam.
- A command palette window (`ui/windows/command_palette_window.slint`, `src/app/command_palette/`) with a static catalog, dynamic command resolution, search-with-recent-boost, and category-prefixed list rendering.
- An App Rules Manager in settings with search, filters, tag chips, suggestions, and per-app controls for hide, aspect ratio, refresh mode, and pinning.
- Performance / refresh mode UX with `realtime` / `balanced` / `battery-saver` / `manual` presets, a status-bar indicator, and per-app interval override.
- A persisted application language setting with `PANOPTICON_LANG` override and live tray tooltip refresh on locale change.

### Changed in 0.1.0

- On-disk version string is `0.1.0` (was `0.1.21`); the `v0.1.21` / `v0.1.2` / `v0.1.1` GitHub tags remain in history and are not modified.
- `Cargo.toml` package version, `docs/PRD.md` documented version, and `CHANGELOG.md` now point to `0.1.0`; the version-bump helper script was removed, so future bumps are made by editing these three files plus tagging the commit manually.
- `cargo clippy` invocations in `AGENTS.md`, `README.md`, `Justfile`, `docs/GETTING_STARTED.md`, and `docs/README.md` now pass `--all-targets` so local checks match the CI workflow.
- `docs/PROJECT_STRUCTURE.md` rewritten to reflect the actual current `src/`, `ui/`, `tests/`, `assets/`, and `docs/` tree (was substantially behind the code).

### Fixed in 0.1.0

- Tray tooltip now refreshes immediately on locale change instead of staying in the previous language until restart.
- Hidden-app fallback labels, saved-profile summaries, tag colour names, and runtime layout labels stay aligned with the selected language.

### Removed in 0.1.0

- `docs/assets/app-showcase.png` and `docs/assets/Diskete.png` (orphan assets; the README no longer references a dashboard screenshot and `Diskete.png` was never referenced).
- `download_icons.ps1` and `gui_smoke_settings.ps1` (host-specific one-off dev scripts; their functionality is already realised in the repo or was never useful outside the maintainer's machine).
- `scripts/bump-version.ps1` (PowerShell release helper; replaced by manual edits of `Cargo.toml` + `docs/PRD.md` + `CHANGELOG.md` plus a `git tag` on the release commit).
- `docs/panopticon_improvement_prd/` (private PRD planning pack; was already excluded by `.gitignore` and is superseded by the implementation it described).

## [0.1.21] - 2026-04-13

> Historical release. The on-disk version was `0.1.21`; functionally superseded by `0.1.0` (2026-06-02), which carries the same code under a re-aligned version number. The `v0.1.21` GitHub release remains published for reference.

### Added in 0.1.21

- a persisted application language setting with English as the default locale and Spanish as a bundled alternative;
- a language selector in the settings window so the UI locale can be changed without editing TOML files by hand;
- localized CLI help and error messages, plus translated titles for the main window, settings window, tag dialog, and native background-image picker.

### Changed in 0.1.21

- user-facing Slint copy now flows through the shared translation layer, including settings navigation, filters, theme/background tools, profile management, keyboard shortcuts, and advanced options;
- layout persistence now uses stable internal storage keys while user-visible layout names come from translations, protecting saved custom ratios when the locale changes;
- quick-start and configuration docs now describe the `language` setting and highlight language selection alongside the rest of the dashboard controls.

### Fixed in 0.1.21

- the tray tooltip now refreshes immediately after changing the active locale instead of staying in the previous language until restart;
- hidden-app fallback labels, saved-profile summaries, tag colour names, and runtime layout labels now stay aligned with the selected language.

## [0.1.2] - 2026-04-13

> Historical release. Superseded by `0.1.0` (2026-06-02). The `v0.1.2` GitHub release remains published for reference.

### Added in 0.1.2

- a broader desktop UI foundation with richer app state, model synchronization, secondary windows, and expanded settings workflows;
- improved window-management plumbing around thumbnails, tray actions, keyboard helpers, renderer selection, and icon handling for the Windows desktop stack.

### Fixed in 0.1.2

- invalid profile names entered via CLI or the settings window are now rejected instead of being silently rewritten or falling back to the current profile when launching an extra instance;
- forced process termination now waits briefly for the target to exit so stale windows disappear more reliably after a kill action.

### Changed in 0.1.2

- the GitHub release workflow now verifies that the pushed tag matches `Cargo.toml` and builds release artifacts with `--locked`;
- user-facing docs now document the Windows-safe profile naming rules used by Panopticon;
- dependencies, fonts, icon handling, and renderer selection were refreshed as part of the UI and infrastructure expansion.

## [0.1.1] - 2026-04-07

> Historical release. Superseded by `0.1.0` (2026-06-02). The `v0.1.1` GitHub release remains published for reference.

### Added in 0.1.1

- application icon support in the system tray and the main window for a more native Windows presentation.

### Changed in 0.1.1

- release tooling, installer assets, and repository ignores were tightened up ahead of the first public maintenance release.

## [0.1.0] - 2026-04-07

> Historical release. The first public tag. Re-used as the version number for the 2026-06-02 rebrand; the `v0.1.0` GitHub release for the 2026-04-07 build remains published for reference.

### Added in 0.1.0

- the initial public release of Panopticon with live DWM thumbnails, multiple layouts, tray integration, local profiles, filters, and on-disk TOML configuration.
