# Technical Debt

Last updated: 2026-08-14

## Active Items

| Priority | Area | Debt | Impact | Next step |
| --- | --- | --- | --- | --- |
| High | Native integration coverage | Tray, DWM thumbnail registration, appbar/dock, Win32 menus, icon extraction, and Explorer restart behavior depend on manual Windows runtime validation. | CI can prove pure logic and compilation, but not the most OS-specific behavior. | Add a small host-gated smoke harness or documented manual checklist that can be run on a real desktop session. |
| Medium | Runtime orchestration | Startup y eventos Win32 siguen coordinados desde la capa app aunque el timer adaptativo, la cadencia y el plan DWM ya son módulos estrechos. | El lifecycle nativo aún requiere razonamiento distribuido. | Continuar extrayendo seams solo al tocar el flujo correspondiente. |
| Medium | Settings/UI exposure gap | Runtime supports some settings such as per-app refresh modes more deeply than the UI exposes them. | Advanced behavior may require editing TOML directly. | Decide whether to expose the remaining settings or document them as advanced TOML-only options. |
| Low | Slint transitive audit warnings | `cargo audit` reports four allowed `unmaintained` warnings: `bincode 2.0.1`, `paste 1.0.15`, `rustybuzz 0.20.1`, and `ttf-parser 0.25.1`, all transitively pulled by Slint/Skia text tooling. | No vulnerability or direct dependency warning was reported; these are maintenance signals, not release blockers. | Recheck after each Slint update and remove each entry when upstream replaces the crate. |

## Recently Closed

| Area | Resolution |
| --- | --- |
| Dependency drift | `Cargo.lock` y el manifest reflejan Slint 1.17.1. Sobre el lock previamente mantenido, este lote añadió cuatro updates transitivos compatibles; el historial queda en `docs/DEPENDENCIES.md`. |
| CI command drift | CI and `Justfile` now use locked commands and cover build/docs/audit. |
| Documentation drift | Docs now reference Slint 1.17.1 and the tracked VS Code task set. |
| Discovery duplicado | `WindowCatalog` comparte una generación canónica entre dashboard, Settings, tray y paleta. |
| Loop idle y DWM | Timer adaptativo y cadencia por tiempo reducen aproximadamente 76% la tasa DWM en reposo observada. |
| Accesibilidad e idioma | Controles custom, thumbnails, navegación y paleta exponen roles/acciones; copy crítico EN/ES tiene test de completitud. |
| Feedback destructivo/persistencia | Reset y kill requieren confirmación; fallos de persistencia tienen banner, reintento y acceso a logs. |
