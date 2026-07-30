# ADR-0005: Use Slint for the Declarative UI Layer

## Status

Accepted

## Context

The UI layer needs to display a dynamic grid of window thumbnails with toolbars, overlays, dialogs, and settings. Options considered included raw Win32/GDI, immediate-mode libraries, and declarative UI frameworks.

## Decision

Use Slint 1.17.1 as the declarative UI framework, with the `raw-window-handle-06`, `backend-winit`, and `renderer-skia` features.

## Consequences

- **Positive:**
  - Modern, declarative UI definition in `.slint` files separate from Rust logic.
  - Built-in data binding between Rust models and UI elements.
  - Skia renderer provides high-quality rendering and animation support.
- **Negative:**
  - Slint cannot natively handle certain Win32 messages (tray, dock, scroll, specific hotkeys).
  - Requires Win32 subclassing of the Slint window to intercept these messages.
  - Dependency on a specific set of Slint features locks the crate version tightly.

## Related

- `ui/main.slint` — main dashboard visual definition
- `src/app/ui_callbacks.rs` — callback wiring between Slint and runtime
- ADR-0006 (Win32 Subclassing) — compensates for Slint's Win32 message limitations
