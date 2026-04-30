# Panopticon — Agent Context

This file contains project-specific guidance for AI agents working on the Panopticon codebase.

## Agent skills

### Backlog

Issues and PRDs live as GitHub issues in `gvastethecreator/panopticon`. See `docs/agents/backlog.md`.

### Triage labels

Five canonical roles mapped to identical label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context repo — one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Quick reference

- **Language:** Rust (edition 2021)
- **UI framework:** Slint 1.15.1
- **Platform:** Windows 10/11 only (Win32, DWM, Shell APIs)
- **Build:** `cargo check`, `cargo test`, `cargo clippy -- -D warnings -W clippy::pedantic`, `cargo fmt -- --check`
- **Architecture docs:** `docs/ARCHITECTURE.md`, `docs/IMPLEMENTATION.md`, `docs/PRD.md`
- **Unsafe policy:** Keep blocks minimal, always add `SAFETY` comments, encapsulate handles in wrappers
