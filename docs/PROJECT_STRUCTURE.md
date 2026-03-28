# Estructura del proyecto

Este documento describe cómo está organizado el repositorio, qué responsabilidad tiene cada carpeta y cuáles son las piezas que conviene tratar como código fuente, assets, documentación o artefactos generados.

## Vista general

```text
panopticon/
├── .agents/
├── .github/
├── assets/
├── docs/
├── src/
├── tests/
├── ui/
├── build.rs
├── Cargo.toml
├── README.md
├── PRD.md
├── Justfile
└── ...otros archivos de soporte
```

## Archivos raíz importantes

| Archivo | Propósito |
| --- | --- |
| `Cargo.toml` | manifiesto del crate, dependencias, perfiles y lints |
| `build.rs` | compila `ui/main.slint` durante el build |
| `README.md` | entrada principal del proyecto |
| `PRD.md` | definición de producto actualizada |
| `Justfile` | comandos auxiliares del proyecto |
| `CHANGELOG.md` | historial de cambios visible |
| `CONTRIBUTING.md` | guía de contribución |
| `SECURITY.md` | política de seguridad |
| `SUPPORT.md` | canales y alcance del soporte |
| `rustfmt.toml` | estilo de formato Rust |
| `LICENSE` | licencia del proyecto |

## Carpetas del repositorio

### `src/`

Es la fuente principal del crate y del binario.

#### Archivos en `src/`

| Ruta | Rol |
| --- | --- |
| `src/lib.rs` | índice del crate biblioteca; reexporta módulos base |
| `src/main.rs` | binario principal; event loop, Win32 interop, tray, settings window, dock y sincronización total |
| `src/constants.rs` | constantes de UI, animación y truncado |
| `src/error.rs` | errores tipados del crate |
| `src/layout.rs` | motor de layouts puro y testeable |
| `src/logging.rs` | inicialización de `tracing` con archivos rolling |
| `src/settings.rs` | persistencia TOML y normalización de configuración |
| `src/theme.rs` | catálogo de temas e interpolación visual |
| `src/thumbnail.rs` | wrapper RAII para thumbnails DWM |
| `src/window_enum.rs` | enumeración de ventanas Win32 y extracción de metadatos |

#### Subcarpeta `src/app/`

Agrupa helpers orientados a la UX del binario.

| Ruta | Rol |
| --- | --- |
| `src/app/mod.rs` | índice de helpers del binario |
| `src/app/settings_ui.rs` | puente entre `AppSettings` y `SettingsWindow` |
| `src/app/tray.rs` | iconos, tray icon, menús nativos y acciones del tray |
| `src/app/window_menu.rs` | menú contextual por ventana |

### `ui/`

Contiene la UI declarativa de Slint.

| Ruta | Rol |
| --- | --- |
| `ui/main.slint` | definición de `MainWindow`, `SettingsWindow`, `TagDialogWindow`, tarjetas, toolbar, scrollbars y componentes visuales |

Es una carpeta clave: cualquier cambio de estructura visual o bindings declarativos pasa por aquí.

### `tests/`

Pruebas de integración del proyecto.

| Ruta | Rol |
| --- | --- |
| `tests/layout_tests.rs` | valida el motor de layouts, overflow, counts, ratios y separadores |

Además, hay tests unitarios embebidos en `src/settings.rs` y `src/theme.rs`.

### `assets/`

Assets consumidos en runtime o durante documentación visual.

| Ruta | Rol |
| --- | --- |
| `assets/themes.json` | catálogo de temas base usado por `src/theme.rs` |

### `docs/`

Documentación técnica y de producto del proyecto.

| Ruta | Enfoque |
| --- | --- |
| `docs/GETTING_STARTED.md` | instalación y flujo inicial |
| `docs/ARCHITECTURE.md` | arquitectura, capas y diagramas |
| `docs/CONFIGURATION.md` | configuración persistente y perfiles |
| `docs/PROJECT_STRUCTURE.md` | mapa del repositorio |
| `docs/IMPLEMENTATION.md` | detalles internos por módulo y runtime |
| `docs/SYSTEM_INTEGRATIONS.md` | APIs, dependencias y servicios del sistema |
| `docs/UX_DESIGN.md` | diseño de experiencia y componentes visuales |
| `docs/assets/` | recursos gráficos usados por la documentación |

### `.github/`

Convenciones del proyecto para colaboración, CI y automatización.

- workflows de CI;
- plantillas de issues/PR si existen;
- instrucciones específicas para agentes.

### `.agents/`

Instrucciones y skills específicas del repositorio para asistentes/automatización. No forman parte del producto final, pero sí de su capa de mantenimiento asistido.

### `target/`

Artefactos generados por Cargo.

Incluye:

- binarios de debug/release;
- dependencias compiladas;
- incremental builds;
- cobertura si se ejecuta `tarpaulin`;
- artefactos de tests y doc.

**No debe editarse manualmente.**

### `doc/`

Documentación HTML generada por `cargo doc`. Es un artefacto generado, no una fuente primaria de mantenimiento.

### `temp/`

Carpeta de trabajo auxiliar del repositorio. Por el estado actual contiene archivos temporales y notas operativas, por lo que conviene tratarla como material de soporte y no como código fuente central.

## Organización conceptual del código

El proyecto puede entenderse en cinco grupos:

1. **Núcleo de dominio técnico**  
   `layout.rs`, `settings.rs`, `theme.rs`
2. **Interoperabilidad Win32/DWM**  
   `window_enum.rs`, `thumbnail.rs`, partes grandes de `main.rs`, `tray.rs`, `window_menu.rs`
3. **Capa de UI**  
   `ui/main.slint`, `settings_ui.rs`
4. **Orquestación runtime**  
   `main.rs`
5. **Calidad y soporte**  
   `tests/`, `docs/`, `logging.rs`, `error.rs`

## Qué carpetas son fuente de verdad

### Sí: editar normalmente

- `src/`
- `ui/`
- `tests/`
- `docs/`
- `assets/`
- archivos raíz como `Cargo.toml`, `README.md`, `PRD.md`

### No: generadas o transitorias

- `target/`
- `doc/`
- parte del contenido de `temp/` si solo son salidas o notas de trabajo

## Rutas clave para distintos tipos de cambio

| Si quieres cambiar... | Empieza por... |
| --- | --- |
| comportamiento de layouts | `src/layout.rs` + `tests/layout_tests.rs` |
| persistencia o settings | `src/settings.rs` + `src/app/settings_ui.rs` |
| UX visual principal | `ui/main.slint` |
| enumeración de ventanas | `src/window_enum.rs` |
| miniaturas DWM | `src/thumbnail.rs` + `src/main.rs` |
| tray y menús rápidos | `src/app/tray.rs` |
| menú por ventana | `src/app/window_menu.rs` |
| theming | `src/theme.rs` + `assets/themes.json` |
| documentación | `README.md`, `PRD.md`, `docs/*.md` |

## Observaciones estructurales importantes

- `main.rs` es el archivo más cargado de responsabilidad del proyecto.
- `layout.rs` es la pieza más limpia y desacoplada.
- la UI declarativa está centralizada en un único archivo Slint, lo cual simplifica la búsqueda, pero puede crecer bastante con el tiempo.
- `target/` y `doc/` pueden ser voluminosos; no deben confundirse con código mantenible del producto.
