# ADR-0002: Isolate the Layout Engine as Pure, Testable Logic

## Status

Accepted

## Context

Window layout computation is a core concern that could either be embedded in the UI/runtime layer or extracted as a standalone module.

## Decision

Extract layout computation into `layout.rs` as a pure function with no dependencies on Win32, Slint, or the runtime state.

## Consequences

- **Positive:**
  - Fully unit-testable without mocking OS APIs or UI frameworks.
  - Clear contract: `(layout, area, count, aspect_hints, custom_ratios) -> LayoutResult`.
  - Easy to add new layout algorithms without touching runtime code.
- **Negative:**
  - Requires explicit synchronisation between the pure output (`LayoutResult`) and the actual DWM thumbnail rectangles.
  - The separation adds a small translation layer in `main.rs` between geometry and visual state.

## Related

- `src/layout.rs` — the pure layout engine
- `src/main.rs` — consumes `LayoutResult` and updates DWM rectangles
- `docs/ARCHITECTURE.md` — runtime flow section
