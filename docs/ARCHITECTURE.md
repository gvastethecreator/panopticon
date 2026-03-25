# Panopticon — Architecture

## Overview

Panopticon is a native Windows desktop application written in Rust. It displays
real-time thumbnails of all open windows using the Desktop Window Manager (DWM)
API, organised via mathematical layout algorithms.

## Crate Structure

```text
panopticon (lib)
├── constants   — Colours, timers, key codes, UI geometry
├── error       — Typed errors (thiserror)
├── layout      — Layout algorithms: Grid, Mosaic, Bento, Fibonacci, Columns
├── logging     — tracing + daily-rolling file appender
├── settings    — Persistent user preferences saved as TOML
├── thumbnail   — RAII wrapper around DWM HTHUMBNAIL
└── window_enum — EnumWindows-based discovery and filtering

panopticon (bin)
├── main.rs     — Win32 window, message loop, painting, HWND-attached state
└── app/
        ├── mod.rs  — Binary-only helpers
        └── tray.rs — Tray icon, popup menu, icon generation, window-icon helpers
```

## Key Design Decisions

### DWM Thumbnails (Zero-Copy Rendering)

Instead of capturing bitmaps, Panopticon registers DWM thumbnails that the
Windows compositor renders directly on the GPU. This means:

- **Near-zero CPU** for rendering (the GPU handles composition).
- **Real-time fidelity** including playing videos.
- **No memory overhead** from bitmap buffers.

### HWND-Attached State Pattern

Win32 window procedures (`WNDPROC`) are `extern "system"` callbacks that cannot
carry user data through their signature. Panopticon attaches a boxed
`AppState` directly to the window using `GWLP_USERDATA` during `WM_NCCREATE`.

Benefits:

1. Avoids a process-global singleton.
2. Uses the canonical Win32 ownership model.
3. Allows deterministic cleanup in `WM_NCDESTROY`.
4. Keeps all mutable UI state scoped to the lifetime of the main window.

### Tray Integration

Panopticon registers a persistent tray icon at startup. The tray system offers:

- Left-click toggle (show / hide).
- Right-click quick actions (show, refresh, next layout, restore hidden apps, exit).
- A generated custom icon used both by the window class and by the tray icon.

The tray icon lets the app behave more like a native desktop utility while
keeping the visual viewer out of the way when not needed.

### Persistent Settings

Panopticon persists a small set of user preferences in
`%APPDATA%\Panopticon\settings.toml`:

- initial layout
- refresh interval
- minimize-to-tray behavior
- close-to-tray behavior
- default aspect-ratio behavior
- default hide-on-select behavior
- always-on-top state
- animation enablement
- per-application rules (hidden / aspect ratio / hide-on-select)

Preferences are loaded at startup, normalized to safe values, and saved again
whenever the user changes them from the tray menu or a thumbnail context menu.

### Per-Application Identity and Rules

Each discovered window is enriched with:

- executable-path-derived app id when available,
- process name,
- native class name.

This lets Panopticon remember preferences per application rather than per transient window title.
The same identity is used to hide apps from the layout, restore them later from the tray,
preserve aspect ratio selectively, and remember whether Panopticon should hide after activation.

### Animated Layout Transitions

Panopticon keeps both a target rectangle and a display rectangle for each managed window.
When layouts change, a lightweight timer interpolates between them over a short duration,
updating thumbnail destinations on each frame without introducing heavyweight rendering dependencies.

### Layout Engine

The layout engine is a **pure function**:

```text
(LayoutType, RECT, count, aspects) → Vec<RECT>
```

It has no side effects and is fully unit-testable. Each layout mode computes
destination rectangles differently:

- **Grid** — equal-sized cells in a √n × √n grid.
- **Mosaic** — row-based with aspect-ratio-weighted column widths.
- **Bento** — primary window (60 %) plus sidebar stack.
- **Fibonacci** — golden-ratio spiral subdivision.
- **Columns** — masonry-style shortest-column-first placement.

### Error Handling

- **Library code** uses `thiserror` for typed, ergonomic errors.
- **Binary code** uses `anyhow` for top-level propagation.
- DWM failures are handled gracefully (thumbnails are dropped, not panicked on).

### Logging

`tracing` with a daily-rolling file appender writes structured logs to
`%TEMP%/panopticon/logs/`. This is critical because
`#![windows_subsystem = "windows"]` suppresses console output.

## Data Flow

```text
EnumWindows → filter → WindowInfo[]
                           ↓
                   DwmRegisterThumbnail → Thumbnail (RAII)
                           ↓
             compute_layout(mode, area, count, aspects) → RECT[]
                           ↓
                   DwmUpdateThumbnailProperties(rect)
                           ↓
                       WM_PAINT → GDI labels, borders, toolbar
```

## Periodic Refresh

A configurable `WM_TIMER` re-enumerates windows, adds/removes thumbnails, and
recomputes the layout. The timer also handles:

- Windows closed externally.
- Windows resized (source size changed).
- New windows opened.

When the Panopticon window is hidden to the tray, it also releases DWM thumbnails,
backs off the refresh cadence, and recreates previews on demand when the window is shown again.

## Dependencies

| Crate | Purpose |
| ----- | ------- |
| `windows` 0.58 | Official Microsoft Win32 bindings |
| `thiserror` | Ergonomic error derives |
| `anyhow` | Top-level error propagation |
| `tracing` | Structured logging facade |
| `tracing-subscriber` | Log subscriber with formatting and filtering |
| `tracing-appender` | Daily-rolling file appender |
