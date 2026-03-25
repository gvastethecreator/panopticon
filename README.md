# Panopticon

![Panopticon icon](docs/assets/panopticon-icon.svg)

**Visor nativo para Windows con miniaturas DWM en tiempo real, filtros por monitor y grupos por tags o aplicación.**

[![CI](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml/badge.svg)](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-2ea043)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-%23CE422B?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4?logo=windows)](https://learn.microsoft.com/windows/)

Panopticon es una aplicación de escritorio escrita en Rust que enumera ventanas de Windows y renderiza miniaturas vivas mediante la API de Desktop Window Manager (DWM). Está pensada para ofrecer una vista global del escritorio con layouts matemáticos, reglas persistentes por aplicación y comportamiento estilo utility/tray app.

## Lo más destacado

- **Miniaturas en vivo por GPU** usando `DwmRegisterThumbnail`, sin capturas bitmap manuales.
- **7 layouts**: Grid, Mosaic, Bento, Fibonacci, Columns, Row (horizontal) y Column (vertical).
- **Filtros por monitor** desde el tray menu.
- **Grupos manuales por tags**: crea un tag con nombre + color desde una app y asígnalo a otras apps.
- **Agrupación automática por aplicación** mediante filtro por app desde el tray.
- **Memoria por app** para ocultar, preservar aspect ratio y ocultar Panopticon tras activar una ventana.
- **Tray menu potente** con restore de apps ocultas, toggles de comportamiento y filtros activos.
- **Ventana de configuración dedicada** con apariencia, layout inicial, header, info y dock.
- **Scrollbar overlay** visible sólo al pasar el mouse cuando realmente hay overflow.
- **Fondo configurable + integración visual con Windows 11** mediante backdrop/rounded corners cuando está disponible.
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
| `1` … `7` | Seleccionar layout directamente |
| `R` | Refrescar la lista de ventanas |
| `A` | Alternar animaciones |
| `H` | Mostrar / ocultar header |
| `I` | Mostrar / ocultar información bajo thumbnails |
| `P` | Alternar siempre visible |
| `O` | Abrir la ventana de configuración |
| `0` | Resetear el ajuste manual del layout actual |
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
2. Selecciona **Create custom tag…**.
3. Escribe el nombre del tag y elige un color.
4. Confirma para asignarlo a esa app y reutilizarlo luego en otras apps.
5. En otras apps, usa **Assign existing tags** para añadir o quitar tags existentes.

> Cuando el filtro activo es un tag, el tablero tiñe el fondo del área de contenido con el color de ese grupo.

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
background_color_hex = "181513"
use_system_backdrop = true
show_toolbar = true
show_window_info = true
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"

[tag_styles.work]
color_hex = "D29A5C"

[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
tags = ["work", "browser"]
```

La ventana de configuración permite editar estas preferencias sin tocar el TOML a mano.

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
