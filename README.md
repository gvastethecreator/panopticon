# Panopticon

![Panopticon icon](docs/assets/icon.webp)

**A native Windows dashboard for viewing, organising, and activating your open windows through live DWM thumbnails.**

[![CI](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml/badge.svg)](https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-2ea043)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-%23CE422B?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4?logo=windows)](https://learn.microsoft.com/windows/)

Panopticon is a local desktop utility built in Rust for Windows 10/11. It discovers real top-level windows, renders their live previews via **Desktop Window Manager (DWM)**, and lets you manage them in a single Slint-based control room.

If you want the full guide instead of the quick landing page, jump to **[`docs/README.md`](docs/README.md)**.

## Why use it?

Panopticon is useful when you want to:

- see many windows at once without constantly alt-tabbing;
- keep a persistent visual workspace with filters, grouping, and tags;
- switch between several layout strategies depending on the task;
- hide the app in the tray and bring it back instantly when needed;
- stay fully local, with no backend, cloud sync, or external services.

## What you get

- **Live DWM thumbnails** instead of static screenshots.
- **7 layout modes**: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, and `Column`.
- **Per-app rules** for hiding, aspect ratio, color, tags, and thumbnail refresh mode.
- **Grouping and filters** by app, monitor, title, class, and tag.
- **Tray utility + appbar/dock mode** for always-available workflows.
- **Themes, background images with fit modes, animations, customizable shortcuts, profiles, and persistence** through local TOML files.
- **Bilingual UI** with English and Spanish support.

## Quick start

### Requirements

| Requirement | Value |
| --- | --- |
| Operating system | Windows 10 / 11 (64-bit) |
| Rust toolchain | Recent stable |
| DWM | Enabled |
| Platform support | Windows only |

### Run it

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release
```

The executable is generated at `target/release/panopticon.exe`.

To run a named profile:

```bash
cargo run --release -- --profile work
```

Panopticon also understands:

- `cargo run --release -- --profile=work`
- `cargo run --release -- --help`
- `cargo run --release -- --version`

## First minute with Panopticon

1. Launch the app with a few normal desktop windows already open.
2. Press `Tab` or `1` to `7` to explore the available layouts.
3. Left-click a thumbnail to activate that window.
4. Right-click a thumbnail to open per-window actions.
5. Press `O` to open settings and review theme, filters, and profiles.
6. Use the tray icon to hide/show the dashboard without closing it.

### Handy shortcuts

These are the **default** bindings. You can rebind them from `Settings -> Keyboard Shortcuts`, and the standalone `Alt` toolbar toggle can also be disabled there.

| Input | Action |
| --- | --- |
| `Tab` | Next layout |
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

## Where things are stored

Configuration is stored locally in:

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\profiles\<profile>.toml
```

If `%APPDATA%` is unavailable, Panopticon falls back to `%TEMP%\Panopticon\...`.

Logs are written to:

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

## Documentation

The root README is intentionally short. The fuller handbook now lives in **[`docs/README.md`](docs/README.md)**.

Useful entry points:

- [`docs/README.md`](docs/README.md) — complete documentation hub and reading guide.
- [`docs/GETTING_STARTED.md`](docs/GETTING_STARTED.md) — install, launch, first-run flow, common issues.
- [`docs/CONFIGURATION.md`](docs/CONFIGURATION.md) — settings, profiles, and TOML schema.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — architecture, runtime layers, and diagrams.
- [`docs/IMPLEMENTATION.md`](docs/IMPLEMENTATION.md) — deeper implementation details by module.
- [`docs/PROJECT_STRUCTURE.md`](docs/PROJECT_STRUCTURE.md) — repository map and code ownership.
- [`docs/PRD.md`](docs/PRD.md) — product goals, scope, and constraints.

Project/community references:

- [`CONTRIBUTING.md`](CONTRIBUTING.md)
- [`SECURITY.md`](SECURITY.md)
- [`SUPPORT.md`](SUPPORT.md)
- [`CHANGELOG.md`](CHANGELOG.md)

## Development

Most day-to-day checks are:

```bash
cargo check
cargo test
cargo clippy -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

VS Code workspace tasks are also available for these commands.

## Scope and status

Panopticon is currently:

- a **local-first Windows utility**;
- built with **Rust + Slint + Win32/DWM**;
- focused on **desktop window observation and activation**;
- not intended for Linux, macOS, web, or remote multi-user usage.

## License

MIT. See [`LICENSE`](LICENSE).
