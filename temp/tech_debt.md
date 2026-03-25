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

### P1. Estado Global Con UnsafeCell

El patrón `static UnsafeCell` es correcto para single-thread Win32, pero
idealmente se migraría a `SetWindowLongPtrW` / `GWLP_USERDATA` que es el
patrón canónico en Win32-Rust. Requiere refactor del flujo de `CreateWindowExW`.

### P2. Directorio `src/app/` Vacío

Existe un directorio `src/app/` vacío. Debería eliminarse si no se planea usar.

### P3. Backup `src/main.rs.bak`

Archivo de backup del refactor. Eliminar una vez verificado que todo funciona.

### P4. CI/CD Pipeline

No hay GitHub Actions ni pipeline de CI. Recomendación: crear
`.github/workflows/ci.yml` que ejecute `just ci`.

### P5. Cargo-tarpaulin para Coverage

El Justfile tiene tarea `coverage` pero `cargo-tarpaulin` no está instalado
por defecto. Documentar el setup en CONTRIBUTING.md.

### P6. Fallback para Ventanas Minimizadas

El PRD menciona mostrar el icono de la aplicación para ventanas minimizadas
(vía `GetClassLongPtrW` / `GCLP_HICON`). Actualmente muestra `[minimized]`.
