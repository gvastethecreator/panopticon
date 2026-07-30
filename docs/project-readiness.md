# Project Readiness

Last updated: 2026-07-30

## Baseline

- Platform: Windows 10/11 only.
- Crate: single Rust binary/library crate, edition 2021.
- Toolchain: Rust 1.96.0 pinned in `rust-toolchain.toml`; dependency MSRV is 1.92.
- UI: Slint 1.17.1 with `raw-window-handle-06`, `backend-winit`, and `renderer-skia`.
- CI: Windows GitHub Actions run check, format, clippy, tests, release build, rustdoc, and dependency audit.

## Maintenance status

- Direct dependencies now use their latest compatible releases; `Cargo.lock` was refreshed with 25 compatible package updates.
- Dead-code suppression is limited to Slint-generated resource code. Two unused Win32 helpers were removed from project source.
- `.gitignore` now preserves the tracked VS Code task file while ignoring local editor state, release artifacts, installer output, and common certificate/backup files.
- `.vscode/tasks.json` provides short tasks for development, checks, build, release, tests, lint, formatting, docs, audit, cleanup, and the full CI sequence.
- Documentation, agent guidance, task references, and dependency versions match the current tree.

## Validation record

Commands ran from repository root on Windows with the pinned toolchain.

| Command | Status | Notes |
| --- | --- | --- |
| `cargo check --all-targets --locked` | Passed | Compiles project and all test targets without warnings. |
| `cargo fmt -- --check` | Passed | Rust formatting is clean. |
| `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic` | Passed | No warnings accepted. |
| `cargo test --all-targets --locked` | Passed | 158 tests passed. |
| `cargo build --release --locked` | Passed | Optimised Windows binary built successfully. |
| `cargo doc --no-deps --locked` | Passed | Generated `target/doc/panopticon/index.html`. |
| `cargo audit` | Not run locally | `cargo-audit` is not installed on this host; CI installs and runs it. |

## Remaining runtime validation

Native integrations still need a real desktop session to exercise tray behavior, DWM thumbnail registration, appbar/dock mode, Win32 menus, and Explorer restart recovery. The automated suite covers the pure logic and compile-time integration paths.
