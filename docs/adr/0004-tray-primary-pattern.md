# ADR-0004: Tray as the Primary Operating Pattern

## Status

Accepted

## Context

Panopticon could behave like a traditional application (open window, close window) or like a persistent desktop utility.

## Decision

Design Panopticon as a persistent tray utility: it starts, registers a system tray icon, and remains available until explicitly exited. Closing the main window hides it rather than terminating the process (configurable).

## Consequences

- **Positive:**
  - Matches the mental model of a workspace dashboard — always available, never "closed".
  - Enables global hotkey activation (`Ctrl + Alt + P`) without requiring the window to be open.
  - Tray menu provides quick access to common actions.
- **Negative:**
  - Requires handling Explorer restart (`TaskbarCreated`) to re-register the tray icon.
  - Adds complexity to the application lifecycle (close-to-tray vs. exit).
  - Some users may expect a traditional close-button behaviour.

## Related

- `src/app/tray.rs` — tray icon registration and menu
- `src/app/window_menu.rs` — per-window context menu
- `src/main.rs` — subclassing for `WM_TRAYICON` and `TaskbarCreated`
