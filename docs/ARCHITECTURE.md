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
├── thumbnail   — RAII wrapper around DWM HTHUMBNAIL
└── window_enum — EnumWindows-based discovery and filtering

panopticon (bin)
└── main.rs     — Win32 window, message loop, painting, global state
```

## Key Design Decisions

### DWM Thumbnails (Zero-Copy Rendering)

Instead of capturing bitmaps, Panopticon registers DWM thumbnails that the
Windows compositor renders directly on the GPU. This means:

- **Near-zero CPU** for rendering (the GPU handles composition).
- **Real-time fidelity** including playing videos.
- **No memory overhead** from bitmap buffers.

### Global State Pattern

Win32 window procedures (`WNDPROC`) are `extern "system"` callbacks that cannot
carry user data through their signature. Panopticon uses a
`static UnsafeCell<Option<AppState>>` to hold the application state.

This is safe because:

1. The Win32 message loop is single-threaded.
2. All state access happens within sequential message handlers.
3. The state is guarded by `is_state_ready()` during early window creation
   (before `AppState` is initialised).

### Layout Engine

The layout engine is a **pure function**:

```text
(LayoutType, RECT, count, aspects) → Vec<RECT>
```

It has no side effects and is fully unit-testable. Each layout mode computes
destination rectangles differently:

| Layout    | Strategy |
| --------- | -------- |
| Grid      | Equal-sized cells in a √n × √n grid |
| Mosaic    | Row-based with aspect-ratio-weighted column widths |
| Bento     | Primary window (60 % width) + sidebar stack |
| Fibonacci | Golden-ratio spiral subdivision |
| Columns   | Masonry-style shortest-column-first placement |

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

A 2-second `WM_TIMER` re-enumerates windows, adds/removes thumbnails, and
recomputes the layout. The timer also handles:

- Windows closed externally.
- Windows resized (source size changed).
- New windows opened.

## Dependencies

| Crate | Purpose |
| ----- | ------- |
| `windows` 0.58 | Official Microsoft Win32 bindings |
| `thiserror` | Ergonomic error derives |
| `anyhow` | Top-level error propagation |
| `tracing` | Structured logging facade |
| `tracing-subscriber` | Log subscriber with formatting and filtering |
| `tracing-appender` | Daily-rolling file appender |
