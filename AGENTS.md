# Panopticon — Agent Context

This file contains project-specific guidance for AI agents working on the Panopticon codebase.

## Agent skills

### Issue tracker

GitHub Issues and the linked GitHub Project hold live state. `.scratch/` holds synchronized local mirrors. See `docs/agents/issue-tracker.md`.

### Triage labels

Five canonical triage roles use identical GitHub label names. See `docs/agents/triage-labels.md`.

### Domain docs

Panopticon uses one root `CONTEXT.md` and root `docs/adr/`. See `docs/agents/domain.md`.

## Quick reference

- **Language:** Rust (edition 2021)
- **UI framework:** Slint 1.17.1
- **Platform:** Windows 10/11 only (Win32, DWM, Shell APIs)
- **Toolchain:** Rust 1.96.0 pinned in `rust-toolchain.toml`; dependency MSRV is 1.92
- **Build:** `cargo check --locked`, `cargo test --all-targets --locked`, `cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic`, `cargo fmt -- --check`, `cargo build --release --locked`
- **Architecture docs:** `docs/ARCHITECTURE.md`, `docs/IMPLEMENTATION.md`, `docs/PRD.md`
- **Unsafe policy:** Keep blocks minimal, always add `SAFETY` comments, encapsulate handles in wrappers
