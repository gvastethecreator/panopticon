# Project Readiness

Last updated: 2026-08-14

## Baseline

- Platform: Windows 10/11 only.
- Crate: single Rust binary/library crate, edition 2021.
- Toolchain: Rust 1.96.0 pinned in `rust-toolchain.toml`; dependency MSRV is 1.92.
- UI: Slint 1.17.1 sin features por defecto, con `accessibility`, `compat-1-2`,
  `raw-window-handle-06`, `backend-winit` y `renderer-skia` explícitas.
- CI: Windows GitHub Actions run check, format, clippy, tests, release build, rustdoc, and dependency audit.

## Maintenance status

- Direct dependencies now use their latest compatible releases. Este lote añadió cuatro updates
  transitivos compatibles (`cc`, `find-msvc-tools`, `libredox`, `pkg-config`) al lock ya mantenido. See
  [`DEPENDENCIES.md`](DEPENDENCIES.md) for changelogs and rationale.
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
| `cargo test --all-targets --locked` | Passed | 170 tests passed. |
| `cargo build --release --locked` | Passed | Optimised Windows binary built successfully. |
| `cargo doc --no-deps --locked` | Passed | Generated `target/doc/panopticon/index.html`. |
| `cargo audit` | Passed | 0 vulnerabilidades; 4 avisos `unmaintained` permitidos en dependencias transitivas de Slint. |

## Runtime validation

Una sesión Windows real verificó el dashboard release, thumbnails DWM, Settings, las seis páginas de
configuración, paleta de comandos, UI Automation, navegación por teclado y confirmación segura de
reset. El grid excluyó Panopticon y TextInputHost. Siguen como gates manuales el tray completo,
appbar/dock, menús nativos, reinicio de Explorer y una terminación real de proceso.
