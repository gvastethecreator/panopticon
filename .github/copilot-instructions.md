# Panopticon Agent Guidelines

## Workflow
- Prefer completing work in large, connected batches instead of stopping after each micro-change.
- After understanding the task, continue through the next logical implementation, verification, and cleanup steps without asking for confirmation unless blocked or a requirement is genuinely ambiguous.
- Keep progress updates short and incremental; report what changed next instead of repeating the full plan.

## Validation
- Before finishing, run the most relevant checks for the files or behavior you changed.
- End with a concise summary of what changed and how it was verified.

## Project References
- Prefer the existing workspace tasks or the equivalent Cargo commands for formatting, linting, checking, building, and testing.
- Link to existing project documentation such as `README.md`, `docs/ARCHITECTURE.md`, and `docs/CONFIGURATION.md` instead of duplicating long explanations.

## Platform and Stack
- Panopticon is a **Windows-only desktop utility** built around `Slint`, `windows-rs`, DWM thumbnails, native tray menus, and local TOML persistence.
- The current direct stack is `slint 1.15.1`, `windows 0.62.2`, `rfd 0.17.2`, and `toml 1.1.2`.
- Keep the UI build flow aligned with `build.rs` + `slint_build::compile("ui/main.slint")`.

## Implementation Rules
- Keep Win32/DWM `unsafe` blocks as small as practical and document the invariants with `SAFETY` comments.
- Prefer pure/testable logic in modules such as `src/layout.rs`, `src/settings.rs`, and `src/theme.rs`; keep OS integration in `src/app/*`, `src/window_enum.rs`, and `src/thumbnail.rs`.
- When updating Win32 calls, remember that `windows 0.62.x` uses `Option<HWND>` / `Option<WPARAM>` / `Option<LPARAM>` in several APIs and places `BOOL` in `windows::core` for callback signatures such as `EnumWindows`.
- Prefer the existing native tray and popup-menu flow in `src/app/tray.rs` and `src/app/window_menu.rs` instead of recreating those menus in Slint.
- Keep using the static Miranda Sans TTF files from `assets/fonts/`; do not reintroduce the variable-font assets until the Slint dependency is upgraded to a release that fixes the current rendering/alignment issues.

## Documentation Expectations
- When changing persisted settings, refresh `docs/CONFIGURATION.md` and any user-facing shortcut descriptions in `README.md` or `docs/GETTING_STARTED.md`.
- When changing OS integrations or direct dependencies, refresh `docs/SYSTEM_INTEGRATIONS.md` and keep architecture notes aligned with the active runtime.
