# Project Readiness

Last updated: 2026-06-29

## Baseline

- Platform: Windows 10/11 only.
- Crate: single Rust binary/library crate, edition 2021.
- Toolchain: Rust 1.96.0 pinned in `rust-toolchain.toml`; dependency MSRV is 1.92 because Slint 1.17.0 requires it.
- UI: Slint 1.17.0 with `raw-window-handle-06`, `backend-winit`, and `renderer-skia`.
- CI: Windows GitHub Actions run check, fmt, clippy, tests, release build, docs, and cargo-audit.

## Findings

| Area | Finding | Action |
| --- | --- | --- |
| Dependencies | `Cargo.lock` could advance to Slint 1.17.0 and newer compatible transitives, but local stable Rust was older than the new MSRV. | Refreshed the lockfile, made direct dependency versions explicit, and pinned Rust 1.96.0. |
| Tooling | `Justfile` said "full CI" but only ran fmt/lint/test. | Expanded `just ci` to match the real local gate: check, fmt, lint, tests, release build, docs, audit. |
| CI reproducibility | CI commands resolved dependencies without `--locked` in several steps. | Added `--locked` to build/check/test/clippy/doc commands. |
| Docs | Docs still referenced Slint 1.15.1, a missing docs image, and VS Code tasks that are not tracked. | Updated docs to match current source tree and tooling. |
| Unsafe policy | `unsafe` usage is broad because of Win32/DWM/Shell/GDI, but the scan showed local `SAFETY` comments around the FFI boundaries. | No code change needed in this pass; keep reviewing new unsafe blocks tightly. |
| Audit warnings | `cargo audit` exits successfully but reports allowed unmaintained warnings for transitive `bincode` and `paste` through `i-slint-compiler`. | Documented as external dependency debt to watch with Slint updates. |

## Completed Maintenance

- Updated direct Rust dependencies in `Cargo.toml`.
- Refreshed `Cargo.lock` through `cargo update`.
- Added `rust-toolchain.toml`.
- Updated `.github/workflows/ci.yml` to use locked Cargo commands.
- Added GitHub Actions updates to Dependabot.
- Reconciled `Justfile`, `AGENTS.md`, `CONTRIBUTING.md`, docs, and changelog with the new baseline.

## Validation Record

Commands are run from the repository root on Windows.

| Command | Status | Notes |
| --- | --- | --- |
| `cargo check --locked` | Passed | Confirms Slint 1.17.0 compiles with the pinned toolchain. |
| `cargo fmt -- --check` | Passed | Clean after applying rustfmt to the clippy fix. |
| `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic` | Passed | Fixed new Rust 1.96 `map_unwrap_or` findings in native-state helpers. |
| `cargo test --all-targets --locked` | Passed | 158 tests passed across library, binary, and integration targets. |
| `cargo build --release --locked` | Passed | First run hit a long compile timeout; rerun completed after waiting on the build lock. |
| `cargo doc --no-deps --locked` | Passed | Generated docs under `target/doc/panopticon/index.html`. |
| `cargo audit` | Passed with warnings | Exit 0; warns about unmaintained transitive `bincode` and `paste` from Slint compiler dependencies. |
| `just --dry-run ci` | Not run | `just` is not installed on this machine; each command wrapped by `just ci` was run directly. |

## Operational Notes

- The global `stable` Rust toolchain on this machine was corrupt during update; `rustup toolchain install 1.96.0-x86_64-pc-windows-msvc --profile minimal --component clippy,rustfmt` succeeded and avoids relying on that broken local alias.
- Native integrations still need manual/runtime checks for tray, DWM thumbnails, appbar mode, Win32 menus, and Explorer restart behavior.
