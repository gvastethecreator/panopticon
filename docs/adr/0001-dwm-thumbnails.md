# ADR-0001: Use DWM Thumbnails Instead of Manual Captures

## Status

Accepted

## Context

Panopticon needs to display live previews of open windows. Two approaches were considered:

1. **Manual bitmap captures** — use GDI or similar APIs to copy window contents into bitmaps and render them in the UI.
2. **DWM thumbnails** — leverage the Windows Desktop Window Manager's built-in thumbnail API (`DwmRegisterThumbnail`, `DwmUpdateThumbnailProperties`).

## Decision

Use DWM thumbnails as the sole visual preview mechanism.

## Consequences

- **Positive:**
  - Minimal CPU overhead — the system compositor handles rendering.
  - Always live — thumbnails reflect the actual window state in real time.
  - No need to manage bitmap memory or copying pipelines.
- **Negative:**
  - Thumbnails become invalid when a window is minimised; requires fallback to icon-based visuals.
  - Tightly coupled to Windows DWM behaviour; no cross-platform path.
  - Some DWM capabilities depend on the specific Windows version and GPU configuration.

## Related

- `src/thumbnail.rs` — RAII wrapper for `HTHUMBNAIL`
- `src/window_enum.rs` — window discovery
- ADR-0005 (Slint UI) — the UI layer consumes thumbnails via model updates
