# Technical Debt

Last updated: 2026-07-30

## Active Items

| Priority | Area | Debt | Impact | Next step |
| --- | --- | --- | --- | --- |
| High | Native integration coverage | Tray, DWM thumbnail registration, appbar/dock, Win32 menus, icon extraction, and Explorer restart behavior depend on manual Windows runtime validation. | CI can prove pure logic and compilation, but not the most OS-specific behavior. | Add a small host-gated smoke harness or documented manual checklist that can be run on a real desktop session. |
| Medium | Runtime orchestration | `src/main.rs` still concentrates startup, timers, refresh, and several Win32 coordination paths. | Changes around lifecycle and native events remain harder to reason about than pure modules. | Continue extracting narrow seams only when touching related behavior. |
| Medium | Settings/UI exposure gap | Runtime supports some settings such as per-app refresh modes more deeply than the UI exposes them. | Advanced behavior may require editing TOML directly. | Decide whether to expose the remaining settings or document them as advanced TOML-only options. |
| Low | Slint transitive audit warnings | Earlier audits reported allowed unmaintained warnings for `bincode 2.0.1` and `paste 1.0.15` through Slint compiler dependencies. This host does not have `cargo-audit` installed for a fresh local result. | No direct vulnerable dependency was previously reported, but audit output needs periodic confirmation. | Let CI run `cargo audit` after each Slint update; remove this item once upstream replaces those crates or audit is clean. |

## Recently Closed

| Area | Resolution |
| --- | --- |
| Dependency drift | `Cargo.lock` and direct manifest versions now reflect the current Slint 1.17.1 baseline plus latest compatible crates. |
| CI command drift | CI and `Justfile` now use locked commands and cover build/docs/audit. |
| Documentation drift | Docs now reference Slint 1.17.1 and the tracked VS Code task set. |
