# Changelog

## Unreleased

## [0.1.2] - 2026-04-13

### Fixed

- invalid profile names entered via CLI or the settings window are now rejected instead of being silently rewritten or falling back to the current profile when launching an extra instance;
- forced process termination now waits briefly for the target to exit so stale windows disappear more reliably after a kill action.

### Changed

- the GitHub release workflow now verifies that the pushed tag matches `Cargo.toml` and builds the release artifacts with `--locked`;
- user-facing docs now document the Windows-safe profile naming rules used by Panopticon.

## Earlier project notes

### Earlier additions

- persistent per-monitor filters;
- tag and application filters from the tray;
- persistent manual tags per application;
- tag creation from each app's context menu;
- open-source documentation and collaboration templates;
- project SVG icon;
- documentation and `cargo audit` jobs in GitHub Actions;
- i18n system with English (default) and Spanish support.

### Earlier changes

- README updated with real repository links;
- architecture documented with filters and grouping flow;
- version set to 0.1.0 for initial public release.
