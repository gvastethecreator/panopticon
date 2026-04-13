# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## [0.1.21] - 2026-04-13

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

### Added in 0.1.1

- application icon support in the system tray and the main window for a more native Windows presentation.

### Changed in 0.1.1

- release tooling, installer assets, and repository ignores were tightened up ahead of the first public maintenance release.

## [0.1.0] - 2026-04-07

### Added in 0.1.0

- the initial public release of Panopticon with live DWM thumbnails, multiple layouts, tray integration, local profiles, filters, and on-disk TOML configuration.
