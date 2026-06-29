# Technical Debt

Last updated: 2026-06-29

## Active Items

| Priority | Area | Debt | Impact | Next step |
| --- | --- | --- | --- | --- |
| High | Native integration coverage | Tray, DWM thumbnail registration, appbar/dock, Win32 menus, icon extraction, and Explorer restart behavior depend on manual Windows runtime validation. | CI can prove pure logic and compilation, but not the most OS-specific behavior. | Add a small host-gated smoke harness or documented manual checklist that can be run on a real desktop session. |
| Medium | Runtime orchestration | `src/main.rs` still concentrates startup, timers, refresh, and several Win32 coordination paths. | Changes around lifecycle and native events remain harder to reason about than pure modules. | Continue extracting narrow seams only when touching related behavior. |
| Medium | Settings/UI exposure gap | Runtime supports some settings such as per-app refresh modes more deeply than the UI exposes them. | Advanced behavior may require editing TOML directly. | Decide whether to expose the remaining settings or document them as advanced TOML-only options. |
| Low | Slint transitive audit warnings | `cargo audit` reports allowed unmaintained warnings for `bincode 2.0.1` via `typed-index-collections -> i-slint-compiler` and `paste 1.0.15` via `ravif/image -> i-slint-compiler`. | No direct vulnerable dependency in Panopticon, but audit output is not perfectly clean. | Track Slint/compiler dependency updates and remove this item once upstream replaces those crates. |
| Low | Local Rust stable alias | The machine's `stable` toolchain failed to update because rustup could not remove a missing doc file. | Local validation must use the pinned `1.96.0` toolchain until stable is repaired. | Repair or remove/reinstall the local stable toolchain outside repo changes. |

## Recently Closed

| Area | Resolution |
| --- | --- |
| Dependency drift | `Cargo.lock` and direct manifest versions now reflect the current Slint 1.17.0 baseline. |
| CI command drift | CI and `Justfile` now use locked commands and cover build/docs/audit. |
| Documentation drift | Docs no longer reference Slint 1.15.1, a missing docs image, or untracked VS Code tasks. |
