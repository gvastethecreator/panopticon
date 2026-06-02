# ADR-0006: Use Win32 Subclassing to Intercept Native Messages

## Status

Accepted

## Context

Because Slint manages its own window and event loop, several Windows-specific behaviours cannot be implemented declaratively: tray icon messages, dock/appbar callbacks, manual scroll handling, and global hotkeys.

## Decision

Subclass the main Slint window's `HWND` to intercept Win32 messages before Slint processes them.

## Consequences

- **Positive:**
  - Enables tray icon interaction (`WM_TRAYICON`).
  - Supports dock/appbar mode (`WM_APPBAR_CALLBACK`).
  - Allows custom scroll handling (`WM_MOUSEWHEEL`, `WM_MBUTTONDOWN`, etc.).
  - Supports specific hotkeys like `Alt` (`WM_SYSKEYDOWN`).
- **Negative:**
  - Introduces `unsafe` code for the subclass procedure and handle management.
  - Ties the application more tightly to the Win32 message loop.
  - Requires careful cleanup to avoid dangling procedure pointers on exit.

## Related

- `src/main.rs` — window subclassing setup and message handling
- `src/app/tray.rs` — tray-specific message handling
- ADR-0005 (Slint UI) — the reason subclassing is necessary
- `docs/ARCHITECTURE.md` — Win32 subclassing section
