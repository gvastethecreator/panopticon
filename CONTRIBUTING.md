# Contributing to Panopticon

Thanks for taking the time to contribute to Panopticon.

## Before you start

- Read `README.md`.
- Review `docs/ARCHITECTURE.md` if you plan to touch Win32, DWM, or persistence.
- If the change affects settings, also update `docs/CONFIGURATION.md`.

## Local setup

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo check --locked
cargo test --all-targets --locked
```

## Project rules

### Minimum quality

Before opening a PR, run:

```bash
cargo fmt -- --check
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic
cargo test --all-targets --locked
cargo build --release --locked
cargo doc --no-deps --locked
```

The repository pins Rust in `rust-toolchain.toml`; use that toolchain for local checks. VS Code users can run the same workflow from `.vscode/tasks.json`, especially `♻️ ci`.

### `unsafe`

Every `unsafe` block must document its invariant with a `// SAFETY:` comment.

### Visible changes

If you change a user-visible feature:

- update `README.md`,
- document the new or changed behaviour,
- add tests if the change touches pure logic or persistence.

### PR style

- keep changes small and coherent;
- explain the **why**, not just the **what**;
- avoid mixing unrelated refactors with a feature or fix.

## Important code areas

- `src/main.rs`: window loop, input, repaint, runtime state.
- `src/window_enum.rs`: window discovery and filtering.
- `src/settings.rs`: TOML persistence, per-app rules, tags, and filters.
- `src/app/tray.rs`: tray integration and menus.
- `src/layout.rs`: pure, testable layouts.

## Useful contribution ideas

- improve the tag editor inside the UI,
- packaging and installer,
- fully opt-in telemetry for diagnostics,
- screenshots or GIFs for documentation,
- more tests for settings and filter flows.

## Bug reports

Use the corresponding issue template and include:

- Windows version,
- exact reproduction steps,
- relevant logs from `%TEMP%/panopticon/logs/`.
