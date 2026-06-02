# ADR-0007: Introduce Explicit Runtime Seams for Settings, Actions, Managed Windows, and Presentation

## Status

Accepted

## Context

Panopticon already isolates some strong modules well:

- `layout.rs` keeps `Layout` computation pure and testable;
- `settings.rs` persists `Workspace` snapshots in TOML;
- `thumbnail.rs` encapsulates the raw `DWM Thumbnail` handle lifecycle.

The main architectural friction now sits in runtime orchestration. In practice, a caller often needs to know too much about:

1. mutating persisted `AppSettings` and then normalising/saving them;
2. deciding which runtime side effects follow an `AppAction`;
3. reconciling `ManagedWindow` values with `Enumeration` and DWM state;
4. materialising dashboard presentation state into Slint models;
5. advancing `Theme` transitions in lock-step with presentation refresh.

This reduces locality. The same behavioural knowledge is spread across callbacks, action dispatch, settings apply flows, thumbnail sync, and UI recompute paths.

## Decision

Introduce explicit runtime seams in the following rollout order:

1. **Settings seam** — split the persisted `Workspace` snapshot from runtime-derived settings state.
   - `AppSettings` remains the persisted source of truth on disk.
   - runtime code consumes a derived settings state instead of repeatedly re-parsing and re-normalising persisted values ad hoc.
2. **Action seam** — route `AppAction` execution through a single runtime mutation/effects path.
3. **ManagedWindow seam** — concentrate `ManagedWindow` reconcile, thumbnail hydration, and cleanup behind one lifecycle-oriented module.
4. **Presentation seam** — materialise dashboard presentation state from runtime state in one place.
5. **Theme transition scope** — keep theme animation inside the presentation seam rather than as a separate top-level runtime seam.

## Consequences

- **Positive**
  - Higher locality for settings mutation, runtime effects, and presentation refresh.
  - Smaller interfaces for callers that only want to change behaviour, not orchestrate follow-up work.
  - Better testability around the actual runtime seams instead of isolated helper functions.
  - `ManagedWindow` internal mutation can be encapsulated more aggressively without leaking DWM bookkeeping to unrelated callers.
- **Negative**
  - The migration is incremental and will temporarily coexist with some legacy call paths.
  - Several modules in `src/app/*` will be touched together, so refactors must be kept small and validated frequently.
  - Documentation must be updated in lock-step so architecture notes stay aligned with the active runtime.

## Non-decisions

- This ADR does **not** replace DWM thumbnails with manual capture. ADR-0001 still stands.
- This ADR does **not** move `Layout` computation out of `layout.rs`. ADR-0002 still stands.
- This ADR does **not** introduce hypothetical abstraction layers for tray, DWM, or workspace persistence where there is only one concrete adapter.

## Related

- ADR-0001 — DWM thumbnails
- ADR-0002 — pure layout engine
- ADR-0003 — persisted settings and workspaces
- ADR-0004 — tray as primary operating pattern
- ADR-0005 — Slint UI
- ADR-0006 — Win32 subclassing