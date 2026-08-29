# Panopticon — Agent Context

Project rules for this repository. Host catalogs such as `.github/copilot-instructions.md` must point here.

## Tracker

GitHub Issues and Project 9 hold live state. Local mirrors live in `.scratch/panopticon/`. Read `docs/agents/issue-tracker.md` before creating or triaging work. Read `docs/agents/triage-labels.md` for the five canonical labels.

## Domain

Use the glossary in root `CONTEXT.md`. Decisions live in `docs/adr/`. Runtime layers live in `docs/ARCHITECTURE.md`. Load `docs/agents/domain.md` before exploring a new area.

## Stack

- Language: Rust edition 2021. Pin: `rust-toolchain.toml` (1.96.0). Dependency MSRV: 1.92.
- UI: Slint 1.17.1 with `accessibility`, `compat-1-2`, `raw-window-handle-06`, `backend-winit`, `renderer-skia`.
- Platform: Windows 10/11 only (Win32, DWM, Shell).
- Direct crates: `windows` 0.62.2, `rfd` 0.17.2, `toml` 1.1.4.
- In `windows` 0.62.x many APIs take `Option<HWND>`, `Option<WPARAM>`, or `Option<LPARAM>`. `EnumWindows` callbacks return `windows::core::BOOL`.
- Product runtime is English-only. Legacy Spanish settings deserialize and normalize to English.

## Build

- `cargo check --locked`
- `cargo test --all-targets --locked`
- `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic`
- `cargo fmt -- --check`
- `cargo build --release --locked`

`Justfile` and `.vscode/tasks.json` wrap the same commands. Prefer `♻️ CI` for the full local gate.

## Implementation

- Keep `unsafe` blocks small. Every block needs a `SAFETY` comment. Encapsulate handles in wrappers.
- Pure or testable logic: `src/layout.rs`, `src/settings.rs`, `src/theme.rs`.
- OS integration: `src/app/*`, `src/window_enum.rs`, `src/thumbnail.rs`.
- Native tray and popup menus: `src/app/tray.rs`, `src/app/window_menu.rs`. Do not rebuild those menus in Slint.
- Fonts: static Miranda Sans TTF files in `assets/fonts/`. Do not reintroduce variable fonts without a Windows rendering check.
- Settings schema change: update `docs/CONFIGURATION.md` and shortcut copy in `README.md` and `docs/GETTING_STARTED.md`.
- OS integration or direct-dependency change: update `docs/SYSTEM_INTEGRATIONS.md`.

## Docs map

- Users: `docs/GETTING_STARTED.md`, `docs/CONFIGURATION.md`
- Contributors: `docs/ARCHITECTURE.md`, `docs/IMPLEMENTATION.md`, `docs/PROJECT_STRUCTURE.md`
- Product intent: `docs/PRD.md`
- Gates: `docs/QUALITY_AUDIT.md`, `docs/project-readiness.md`
