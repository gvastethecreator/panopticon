# Panopticon

![Panopticon icon](docs/assets/panopticon-icon.svg)

**Visor nativo para Windows que muestra miniaturas DWM en tiempo real, organiza ventanas con layouts matemáticos y permite filtrarlas, agruparlas y gestionarlas desde tray.**

[![CI](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml/badge.svg)](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-2ea043)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-%23CE422B?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4?logo=windows)](https://learn.microsoft.com/windows/)

Panopticon es una aplicación de escritorio escrita en Rust para Windows 10/11 que enumera ventanas de nivel superior, registra miniaturas vivas con la API de **Desktop Window Manager (DWM)** y las presenta dentro de una interfaz construida con **Slint**. El proyecto está pensado como una utilidad visual de productividad: una “cabina de control” para ver, filtrar, reordenar y activar ventanas abiertas sin depender de capturas de pantalla manuales ni de un backend externo.

## Qué hace

- **Descubre ventanas reales del sistema** usando `EnumWindows` y filtra tool windows, superficies del sistema y ventanas no relevantes.
- **Renderiza miniaturas vivas aceleradas por GPU** con `DwmRegisterThumbnail` y `DwmUpdateThumbnailProperties`.
- **Ofrece 7 layouts**: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row` y `Column`.
- **Permite personalización persistente por aplicación**: ocultar, preservar aspecto, ocultar Panopticon al activar una app, color propio, tags y estrategia de refresco del thumbnail.
- **Incluye filtros y agrupación** por monitor, tag, aplicación y criterio de agrupado (`Application`, `Monitor`, `WindowTitle`, `ClassName`).
- **Funciona como utilidad de tray** con icono persistente, menú contextual y restauración de apps ocultas.
- **Incluye una ventana de configuración dedicada** y soporte de perfiles múltiples mediante `--profile`.
- **Soporta dock/appbar** en los bordes de la pantalla usando `SHAppBarMessage`.
- **Aplica theming dinámico** a partir de `assets/themes.json`, con animación de transición entre temas.
- **Registra logs estructurados** en `%TEMP%\panopticon\logs\`.

## Arquitectura de un vistazo

Panopticon no utiliza servicios remotos ni backend: todo ocurre en la máquina local.

1. `main.rs` crea la ventana principal Slint, configura DPI awareness y adquiere el `HWND` nativo.
2. `window_enum.rs` enumera ventanas y genera `WindowInfo` con metadatos persistibles (`app_id`, monitor, proceso, clase, título).
3. `layout.rs` calcula rectángulos puros y separadores de resize en memoria.
4. `thumbnail.rs` administra miniaturas DWM con RAII.
5. `settings.rs` persiste preferencias y reglas por app en TOML.
6. `app/tray.rs`, `app/window_menu.rs` y `app/settings_ui.rs` conectan Win32/Slint con la experiencia de usuario.

Más detalle en [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) y [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md).

## Requisitos

| Requisito | Valor |
| --- | --- |
| Sistema operativo | Windows 10 / 11 (64-bit) |
| Toolchain Rust | estable reciente |
| DWM | habilitado |
| Plataforma | escritorio local, sin soporte Linux/macOS |

## Compilar y ejecutar

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release
```

El ejecutable generado queda en `target/release/panopticon.exe`.

Para abrir una instancia asociada a un perfil concreto:

```bash
cargo run --release -- --profile trabajo
```

## Uso rápido

### Atajos principales

| Entrada | Acción |
| --- | --- |
| `Tab` | Cambiar al siguiente layout |
| `1` … `7` | Seleccionar layout directo |
| `0` | Resetear ratios personalizados del layout actual |
| `R` | Refrescar ventanas |
| `A` | Alternar animaciones |
| `H` | Mostrar/ocultar toolbar |
| `I` | Mostrar/ocultar metadatos de ventanas |
| `P` | Alternar always-on-top |
| `T` | Cambiar tema |
| `O` | Abrir settings |
| `M` | Abrir menú de aplicación |
| `Alt` | Alternar toolbar |
| `Esc` | Salir |

### Interacciones con mouse

| Acción | Resultado |
| --- | --- |
| Click izquierdo en miniatura | Activa la ventana destino |
| Click derecho en miniatura | Abre menú contextual por ventana |
| Drag sobre separadores | Ajusta ratios persistentes del layout |
| Rueda / botón central | Navega layouts con overflow (`Row` / `Column`) |
| Click izquierdo en tray | Mostrar/ocultar Panopticon |
| Click derecho en tray | Abrir acciones rápidas, filtros y opciones |

## Configuración y persistencia

Panopticon guarda configuración en:

```text
%APPDATA%\Panopticon\settings.toml
```

Los perfiles nombrados se guardan en:

```text
%APPDATA%\Panopticon\profiles\<perfil>.toml
```

Si `%APPDATA%` no está disponible, el proyecto hace fallback a `%TEMP%\Panopticon\...`.

Los logs se escriben en:

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

Consulta [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) para el detalle completo del esquema TOML y sus efectos reales en runtime.

## Documentación del proyecto

La documentación está separada por enfoque para que cada archivo sea fácil de mantener:

- [`PRD.md`](PRD.md) — documento de producto actualizado y alineado con la implementación.
- [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) — instalación, primer arranque y flujo de uso inicial.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — arquitectura técnica, capas, runtime y diagramas.
- [`docs/PROJECT_STRUCTURE.md`](docs/PROJECT_STRUCTURE.md) — estructura del repositorio y responsabilidades por archivo/carpeta.
- [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) — detalles de implementación por módulo y flujo interno.
- [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) — configuración persistente, perfiles y ejemplos TOML.
- [`docs/SYSTEM_INTEGRATIONS.md`](docs/SYSTEM_INTEGRATIONS.md) — APIs, librerías y servicios del sistema utilizados.
- [`docs/UX_DESIGN.md`](docs/UX_DESIGN.md) — diseño de UX/UI, layouts, interacciones y decisiones visuales.

Además del paquete documental principal, el repositorio mantiene:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`SUPPORT.md`](SUPPORT.md)
- [`CHANGELOG.md`](CHANGELOG.md)

## Desarrollo

### Comandos útiles

```bash
cargo check
cargo test
cargo clippy -- -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

También puedes usar las tareas del workspace de VS Code (`🔍 check`, `🧪 test`, `🧹 lint`, `🎨 fmt-check`, etc.).

### Qué está cubierto por tests

- tests de integración del motor de layouts en `tests/layout_tests.rs`;
- tests unitarios de configuración en `src/settings.rs`;
- tests unitarios de theming en `src/theme.rs`.

Hoy no hay una suite automatizada equivalente para tray, DWM, menús nativos o enumeración Win32, lo cual es una limitación importante y documentada.

## Estado del proyecto

Panopticon ya tiene una base funcional completa como utilidad local de Windows:

- UI declarativa con Slint;
- integración nativa con Win32, DWM, Shell, GDI y HiDPI;
- persistencia por perfil;
- theming, tray, dock, filtros y tags;
- documentación técnica ampliada.

## Licencia

MIT. Consulta [`LICENSE`](LICENSE).
