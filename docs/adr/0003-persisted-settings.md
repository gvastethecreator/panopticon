# ADR-0003: Persist Settings and Per-App Rules as Source of Truth

## Status

Accepted

## Context

Application state (filters, groups, themes, per-app rules) could be kept only in memory or persisted to disk.

## Decision

Persist all user configuration and per-app rules in TOML files under `%APPDATA%\Panopticon\`, with support for named workspaces as separate files.

## Consequences

- **Positive:**
  - User context survives application restarts.
  - Workspaces allow completely separate configurations for different workflows.
  - Human-readable, version-control-friendly format.
- **Negative:**
  - Requires careful normalisation of invalid inputs before they reach the runtime.
  - File I/O errors must be handled gracefully with sensible fallbacks.
  - Workspace names are constrained by Windows filename rules.

## Related

- `src/settings.rs` — persistence, normalisation, and per-app rules
- `src/main.rs` — workspace loading via `--workspace <name>`
