# Deuda Técnica Resuelta — Panopticon v1.1.0

## Deuda Resuelta

### 1. Estado Global Sin Protección (Severidad: Alta)

**Antes:** `UnsafeCell<Option<AppState>>` sin guards. Si `WM_SIZE` o `WM_PAINT`
llegaban durante `CreateWindowExW`, `app()` hacía panic porque `AppState` era `None`.
**Después:** `is_state_ready()` guard en todos los handlers del `wnd_proc`.
Los mensajes tempranos se manejan gracefully.

### 2. Ausencia de Logging (Severidad: Alta)

**Antes:** `#![windows_subsystem = "windows"]` suprime stdout — errores silenciosos.
**Después:** `tracing` con file appender en `%TEMP%/panopticon/logs/`.
Errores, cambios de layout y activaciones de ventana quedan registrados.

### 3. Sin Manejo de Errores Tipado (Severidad: Media)

**Antes:** Errores de Win32 ignorados silenciosamente, sin tipos de error propios.
**Después:** `PanopticonError` con `thiserror`; `anyhow` disponible para el binario.
DWM failures se loguean y degradan gracefully.

### 4. Sin Tests (Severidad: Media)

**Antes:** Cero tests. El motor de layouts (lógica pura) no tenía ninguna cobertura.
**Después:** 10 tests de integración cubriendo edge cases, bounds, cycling, mixed aspects.

### 5. Unsafe Sin Documentación (Severidad: Media)

**Antes:** 24+ bloques `unsafe` sin comentarios `SAFETY`.
**Después:** Cada bloque `unsafe` tiene un comentario `// SAFETY:` explicando
por qué la operación es válida.

### 6. Clippy Warnings Extensivos (Severidad: Baja)

**Antes:** `cargo clippy -- -D warnings -W clippy::pedantic` fallaba con 72+ errores.
**Después:** Pasa limpio. Fixes idiomáticos (`.div_ceil()`, `.clamp()`, `.total_cmp()`,
`map_or()`, `contains()`, `f64::from()`).

### 7. Sin Automatización (Severidad: Baja)

**Antes:** Sin Justfile, .gitignore minimal, sin rustfmt.toml.
**Después:** Justfile con 12 tareas, .gitignore robusto, rustfmt.toml configurado.

### 8. Código Monolítico (Severidad: Baja)

**Antes:** Todo en un crate binario. No separación lib/bin. Sin modularización de constantes.
**Después:** Arquitectura lib + bin. Módulos separados para constants, error, logging.
Tests de integración posibles gracias al lib crate.

---

## Deuda Técnica Pendiente (Futura)

### P1. Re-registro del tray icon tras reinicio de Explorer

Si `explorer.exe` se reinicia, el área de notificación pierde los iconos. La
mejora pendiente es escuchar el mensaje registrado `TaskbarCreated` y volver a
registrar el tray icon automáticamente.

### P2. Menú contextual avanzado / Preferencias

El tray ya soporta show/hide, refresh, next layout, hide-on-minimize,
hide-on-close, cycle refresh interval y exit. A futuro conviene añadir más
preferencias persistentes (always-on-top, layout favorites, filtros por app,
autostart, etc.).

### P3. Cobertura en CI

Ya existe tarea `coverage` y documentación para `cargo-tarpaulin`, pero no se
incluye aún en GitHub Actions por coste/tiempo de ejecución y compatibilidad en
Windows.
