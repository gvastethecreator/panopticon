# Panopticon

<p align="center">
  <img src="docs/assets/panopticon-icon.svg" alt="Panopticon icon" width="144" height="144">
</p>

<p align="center">
  <strong>Visor nativo para Windows con miniaturas DWM en tiempo real, filtros por monitor y grupos por tags o aplicación.</strong>
</p>

<p align="center">
  <a href="https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml/badge.svg"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-MIT-2ea043"></a>
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-2021-%23CE422B?logo=rust"></a>
  <a href="https://learn.microsoft.com/windows/"><img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4?logo=windows"></a>
</p>

Panopticon es una aplicación de escritorio escrita en Rust que enumera ventanas de Windows y renderiza miniaturas vivas mediante la API de Desktop Window Manager (DWM). Está pensada para ofrecer una vista global del escritorio con layouts matemáticos, reglas persistentes por aplicación y comportamiento estilo utility/tray app.

## Lo más destacado

- **Miniaturas en vivo por GPU** usando `DwmRegisterThumbnail`, sin capturas bitmap manuales.
- **5 layouts**: Grid, Mosaic, Bento, Fibonacci y Columns.
- **Filtros por monitor** desde el tray menu.
- **Grupos manuales por tags**: crea un tag desde una app y asígnalo a otras apps.
- **Agrupación automática por aplicación** mediante filtro por app desde el tray.
- **Memoria por app** para ocultar, preservar aspect ratio y ocultar Panopticon tras activar una ventana.
- **Tray menu potente** con restore de apps ocultas, toggles de comportamiento y filtros activos.
- **Animaciones suaves** al recomputar el layout.
- **Logging estructurado** en `%TEMP%/panopticon/logs/`.
- **Refresco adaptativo** cuando la ventana principal está escondida en el tray.

## Requisitos

| Requisito | Versión |
| --- | --- |
| Sistema operativo | Windows 10 / 11 (64-bit) |
| Toolchain Rust | 1.82+ |
| DWM | Habilitado |

## Instalación rápida

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo build --release
```

El ejecutable queda en `target/release/panopticon.exe`.

## Uso

```bash
cargo run --release
```

### Controles

| Entrada | Acción |
| --- | --- |
| `Tab` | Cambiar al siguiente layout |
| `R` | Refrescar la lista de ventanas |
| Click izquierdo sobre miniatura | Activar la ventana |
| Click derecho sobre miniatura | Abrir opciones por aplicación |
| Click izquierdo en icono del tray | Mostrar / ocultar Panopticon |
| Click derecho en icono del tray | Acciones rápidas, filtros y preferencias |
| `Esc` | Cerrar la aplicación |

### Filtros y grupos

Panopticon ahora puede reducir el tablero según el contexto que necesites:

- **Filter by monitor** — muestra sólo ventanas del monitor seleccionado.
- **Filter by tag** — muestra sólo apps asociadas a un tag manual.
- **Filter by application** — agrupa/filtra automáticamente por aplicación (`app_id`).

### Cómo crear tags

1. Haz click derecho sobre una miniatura.
2. Selecciona **Create tag from this app** para sembrar un tag a partir del nombre de la aplicación.
3. En otras apps, usa **Assign existing tags** para añadir o quitar tags existentes.

> Los tags se persisten en `settings.toml`. Si quieres nombres completamente personalizados, también puedes editarlos manualmente en el archivo de configuración.

## Configuración

Las preferencias se guardan en:

```text
%APPDATA%\Panopticon\settings.toml
```

Ejemplo mínimo:

```toml
initial_layout = "grid"
refresh_interval_ms = 2000
minimize_to_tray = true
close_to_tray = true
preserve_aspect_ratio = false
hide_on_select = true
animate_transitions = true
always_on_top = false
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"

[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
tags = ["work", "browser"]
```

Consulta más ejemplos en [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md).

## Desarrollo

### Tareas principales

```bash
cargo check
cargo test
cargo clippy --all-targets -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

También puedes usar las tareas del workspace o el `Justfile`.

## Documentación

- [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) — instalación y primer arranque.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — diseño interno del proyecto.
- [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) — settings, filtros y tags.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — guía de contribución.
- [`SECURITY.md`](SECURITY.md) — reporte responsable de vulnerabilidades.
- [`SUPPORT.md`](SUPPORT.md) — soporte, preguntas y canales sugeridos.

## Estado del proyecto

El proyecto ya está preparado para trabajo open source básico:

- CI por GitHub Actions en Windows;
- plantillas de issues y PR;
- documentación pública y guía de contribución;
- changelog inicial y políticas básicas de colaboración.

## Contribuir

Las contribuciones son bienvenidas. Antes de abrir un PR:

1. crea una rama propia,
2. ejecuta `cargo fmt`, `cargo clippy` y `cargo test`,
3. documenta cualquier cambio visible o de configuración.

Más detalles en [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Licencia

MIT. Mira [`LICENSE`](LICENSE).
