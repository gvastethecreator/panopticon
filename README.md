# Panopticon

![Panopticon icon](docs/assets/icon.webp)

**Native Windows viewer that displays real-time DWM thumbnails, organises windows with mathematical layouts, and lets you filter, group, and manage them from the system tray.**

[![CI](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml/badge.svg)](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-2ea043)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-%23CE422B?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4?logo=windows)](https://learn.microsoft.com/windows/)

Panopticon is a desktop application written in Rust for Windows 10/11 that enumerates top-level windows, registers live thumbnails with the **Desktop Window Manager (DWM)** API, and presents them inside a **Slint**-based UI. The project is designed as a visual productivity utility: a "control room" to view, filter, reorder, and activate open windows without manual screenshots or an external backend.

## What it does

- **Discovers real system windows** using `EnumWindows` and filters out tool windows, system surfaces, and irrelevant windows.
- **Renders live GPU-accelerated thumbnails** with `DwmRegisterThumbnail` and `DwmUpdateThumbnailProperties`.
- **Offers 7 layouts**: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, and `Column`.
- **Supports persistent per-app customisation**: hide, preserve aspect ratio, hide Panopticon on activation, custom colour, tags, and thumbnail refresh strategy.
- **Includes filters and grouping** by monitor, tag, application, and grouping criterion (`Application`, `Monitor`, `WindowTitle`, `ClassName`).
- **Works as a tray utility** with a persistent icon, context menu, and hidden-app restoration.
- **Includes a dedicated settings window** and multi-profile support via `--profile`.
- **Supports dock/appbar** on screen edges using `SHAppBarMessage`.
- **Applies dynamic theming** from `assets/themes.json`, with animated theme transitions.
- **Writes structured logs** to `%TEMP%\panopticon\logs\`.
- **Built-in i18n** with English (default) and Spanish.

## Architecture at a glance

Panopticon does not use remote services or a backend: everything happens on the local machine.

1. `main.rs` creates the main Slint window, configures DPI awareness, and acquires the native `HWND`.
2. `window_enum.rs` enumerates windows and builds `WindowInfo` with persistable metadata (`app_id`, monitor, process, class, title).
3. `layout.rs` computes pure rectangles and resize separators in memory.
4. `thumbnail.rs` manages DWM thumbnails with RAII.
5. `settings.rs` persists preferences and per-app rules in TOML.
6. `app/tray.rs`, `app/window_menu.rs`, and `app/settings_ui.rs` connect Win32/Slint to the user experience.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) for more detail.

## Requirements

| Requirement | Value |
| --- | --- |
| Operating system | Windows 10 / 11 (64-bit) |
| Rust toolchain | recent stable |
| DWM | enabled |
| Platform | local desktop, no Linux/macOS support |

## Build and run

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release
```

The resulting executable is at `target/release/panopticon.exe`.

To launch an instance tied to a specific profile:

```bash
cargo run --release -- --profile work
```

## Quick usage

### Main shortcuts

| Input | Action |
| --- | --- |
| `Tab` | Cycle to next layout |
| `1` ... `7` | Select layout directly |
| `0` | Reset custom ratios for the current layout |
| `R` | Refresh windows |
| `A` | Toggle animations |
| `H` | Show/hide toolbar |
| `I` | Show/hide window metadata |
| `P` | Toggle always-on-top |
| `T` | Change theme |
| `O` | Open settings |
| `M` | Open application menu |
| `Alt` | Toggle toolbar |
| `Esc` | Exit |

### Mouse interactions

| Action | Result |
| --- | --- |
| Left-click on thumbnail | Activate target window |
| Right-click on thumbnail | Open per-window context menu |
| Drag on separators | Adjust persistent layout ratios |
| Wheel / middle button | Navigate layouts with overflow (`Row` / `Column`) |
| Left-click on tray | Show/hide Panopticon |
| Right-click on tray | Open quick actions, filters, and options |

## Configuration and persistence

Panopticon saves configuration in:

```text
%APPDATA%\Panopticon\settings.toml
```

Named profiles are saved in:

```text
%APPDATA%\Panopticon\profiles\<profile>.toml
```

If `%APPDATA%` is not available, the project falls back to `%TEMP%\Panopticon\...`.

Logs are written to:

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

See [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) for the full TOML schema and its runtime effects.

## Project documentation

Documentation is split by focus so each file is easy to maintain:

- [`PRD.md`](PRD.md) — product requirements document aligned with the current implementation.
- [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) — installation, first launch, and initial usage flow.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — technical architecture, layers, runtime, and diagrams.
- [`docs/PROJECT_STRUCTURE.md`](docs/PROJECT_STRUCTURE.md) — repository structure and per-file responsibilities.
- [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) — implementation details by module and internal flow.
- [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) — persistent configuration, profiles, and TOML examples.
- [`docs/SYSTEM_INTEGRATIONS.md`](docs/SYSTEM_INTEGRATIONS.md) — APIs, libraries, and system services used.
- [`docs/UX_DESIGN.md`](docs/UX_DESIGN.md) — UX/UI design, layouts, interactions, and visual decisions.

Additional community files:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`SUPPORT.md`](SUPPORT.md)
- [`CHANGELOG.md`](CHANGELOG.md)

## Development

### Useful commands

```bash
cargo check
cargo test
cargo clippy -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

You can also use the VS Code workspace tasks (`check`, `test`, `lint`, `fmt-check`, etc.).

### What is covered by tests

- layout engine integration tests in `tests/layout_tests.rs`;
- settings unit tests in `src/settings.rs`;
- theming unit tests in `src/theme.rs`;
- i18n unit tests in `src/i18n.rs`.

There is no automated test suite for tray, DWM, native menus, or Win32 enumeration, which is a known and documented limitation.

## Project status

Panopticon has a fully functional base as a local Windows utility:

- declarative UI with Slint;
- native integration with Win32, DWM, Shell, GDI, and HiDPI;
- per-profile persistence;
- theming, tray, dock, filters, and tags;
- i18n (English / Spanish);
- expanded technical documentation.

## License

MIT. See [`LICENSE`](LICENSE).
