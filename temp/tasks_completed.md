# Tareas Completadas — Refactorización Panopticon v1.1.0

## Arquitectura

- [x] Dividir crate en **lib + bin**: `src/lib.rs` exporta módulos públicos; `src/main.rs` es sólo el binario Win32.
- [x] Crear `src/constants.rs` — constantes extraídas de `main.rs` (colores BGR, timers, VK codes, límites de texto).
- [x] Crear `src/error.rs` — errores tipados con `thiserror` (`PanopticonError`).
- [x] Crear `src/logging.rs` — logging estructurado con `tracing` + `tracing-appender` (rolling diario en `%TEMP%/panopticon/logs/`).
- [x] Migrar estado del binario a **`GWLP_USERDATA`** — eliminar el singleton global y adjuntar `AppState` al `HWND`.
- [x] Reutilizar `src/app/` con `mod.rs` + `tray.rs` para helpers del binario.

## Dependencias (Cargo.toml)

- [x] Añadir `thiserror`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`.
- [x] Añadir metadatos del paquete (`description`, `license`, `readme`).
- [x] Configurar `[lints.clippy]` con pedantic + allows razonables.
- [x] Añadir `strip = true` al perfil release.

## Calidad de Código

- [x] **Clippy pedantic limpio:** `cargo clippy -- -D warnings -W clippy::pedantic` pasa sin errores.
- [x] Reemplazar `manual_div_ceil` → `usize::div_ceil()`.
- [x] Reemplazar `manual_is_multiple_of` → `usize::is_multiple_of()`.
- [x] Reemplazar `.max().min()` → `.clamp()`.
- [x] Reemplazar `.map().unwrap_or()` → `.map_or()`.
- [x] Reemplazar `iter().any()` → `contains()` para slices de `&str`.
- [x] Reemplazar loop con indexación → destructuring iterator.
- [x] Usar `f64::from(i32)` para conversiones lossless (en vez de `as f64`).
- [x] Usar `f64::total_cmp()` en vez de `partial_cmp().unwrap()`.
- [x] Usar `std::ptr::null_mut()` en vez de `0 as *mut _`.
- [x] Usar `std::ptr::from_mut()` en vez de `&mut x as *mut T`.
- [x] Usar `&raw const` para pasar structs a FFI.
- [x] Extraer helpers `lparam_x()` / `lparam_y()` para eliminar bit-manipulation duplicada.
- [x] Extraer `handle_keydown()` y `truncate_title()` para reducir complejidad de `wnd_proc` / `paint`.

## Seguridad y Correctitud

- [x] **Bug corregido:** `is_state_ready()` guard previene panic cuando `WM_SIZE` / `WM_PAINT` llegan antes de que `AppState` esté inicializado (durante `CreateWindowExW`).
- [x] **Refactor posterior:** se elimina completamente el patrón global `UnsafeCell` y se sustituye por `GWLP_USERDATA` + cleanup en `WM_NCDESTROY`.
- [x] Documentar `unsafe impl Sync` con justificación de por qué es sound (single-threaded message loop).
- [x] Añadir comentarios `// SAFETY:` en cada bloque `unsafe` (24+ bloques).
- [x] Marcar `app()` como `unsafe fn` con documentación de precondiciones.

## UX / Interfaz

- [x] Añadir **tray icon real** con icono generado en runtime (no stock icon por defecto salvo fallback).
- [x] Añadir menú contextual del tray: show/hide, refresh, next layout, exit.
- [x] Minimizar/cerrar → esconder al tray; `Esc` mantiene salida inmediata.
- [x] Mejorar la toolbar y el empty state con una interfaz más sobria y profesional.
- [x] Mostrar icono real de la ventana cuando el thumbnail no está disponible.

## Documentación

- [x] Añadir `//!` doc-comments a todos los módulos.
- [x] Añadir `///` doc-comments a todas las funciones y tipos públicos.
- [x] Añadir `#[must_use]` a funciones puras públicas.
- [x] Derivar `Hash` en `LayoutType` (necesario para tests + futuro uso en collections).
- [x] README completo: badges, features, requisitos, instalación, uso, controles, layout modes, desarrollo, logging, contributing.
- [x] `docs/ARCHITECTURE.md` con diagramas de flujo, decisiones de diseño, dependencias.

## Tests

- [x] 10 tests de integración para el motor de layouts: zero windows, single window, correct count, bounds checking, positive dimensions, layout cycling, aspect ratio edge case, label coverage, small area stress, mixed aspects.
- [x] `cargo test` — 10/10 ok.

## Automatización y Tooling

- [x] `Justfile` con 12 tareas: build, release, check, lint, fmt, fmt-check, test, coverage, doc, run, run-release, clean, ci.
- [x] `.vscode/tasks.json` con tareas cortas y legibles con emojis.
- [x] `.vscode/launch.json` actualizado para usar las nuevas tareas.
- [x] `rustfmt.toml` (edition 2021, max_width 100).
- [x] `.gitignore` robusto (target, IDE, OS files, env, coverage, logs, backups).
- [x] `.github/workflows/ci.yml` con fmt, clippy, tests y release build en Windows.

## Formato

- [x] `cargo fmt` aplicado a todo el proyecto.
- [x] `cargo fmt -- --check` pasa limpio.
