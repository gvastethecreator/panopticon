# Panopticon Documentation

![Panopticon icon](assets/icon.webp)

This folder contains the **full documentation hub** for Panopticon.

The root [`README.md`](../README.md) is now the short, public-friendly landing page. This document is the more complete guide for users and contributors who want context, structure, and pointers to the right files without guessing where to start.

## What Panopticon is

Panopticon is a native Windows application that shows your open windows as **live DWM thumbnails** inside a single dashboard. It is designed as a local productivity utility: a control room where you can scan, group, filter, activate, and customise open applications without relying on screenshots, cloud services, or a background server.

At a glance, Panopticon provides:

- live previews powered by the Windows compositor;
- multiple mathematical layouts for arranging windows;
- per-application rules for visibility, color, tags, and refresh strategy;
- tray-first behaviour and optional appbar/dock mode;
- persistent settings and named profiles;
- a Slint UI with English and Spanish support.

## Choose your path

### If you want to use the app

Start here:

1. [`GETTING_STARTED.md`](GETTING_STARTED.md)
2. [`CONFIGURATION.md`](CONFIGURATION.md)
3. [`UX_DESIGN.md`](UX_DESIGN.md)

### If you want to understand or modify the codebase

Start here:

1. [`ARCHITECTURE.md`](ARCHITECTURE.md)
2. [`PROJECT_STRUCTURE.md`](PROJECT_STRUCTURE.md)
3. [`IMPLEMENTATION.md`](IMPLEMENTATION.md)
4. [`SYSTEM_INTEGRATIONS.md`](SYSTEM_INTEGRATIONS.md)

### If you want the product intent and scope

Read [`PRD.md`](PRD.md).

## Quick overview

### What happens in a normal session

When Panopticon starts, it typically:

1. initialises logging;
2. loads the active configuration/profile;
3. creates the main Slint window;
4. acquires the native `HWND` and applies window appearance;
5. registers the tray icon;
6. enumerates visible top-level windows;
7. registers DWM thumbnails;
8. computes the chosen layout and fills the UI model.

### Core capabilities

| Area | Summary |
| --- | --- |
| Window discovery | Enumerates real top-level windows and filters out non-user surfaces |
| Live previews | Uses `DwmRegisterThumbnail` / `DwmUpdateThumbnailProperties` |
| Layouts | Supports `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, `Column` |
| Interaction | Activate windows, open native menus, drag separators, use tray actions |
| Persistence | Saves global settings, per-app rules, tag styles, and layout customisations |
| Theming | Loads themes from `assets/themes.json` and supports animated transitions |
| Docking | Can operate as an appbar on a screen edge |

## Running Panopticon

### Requirements

- Windows 10 or Windows 11 (64-bit)
- DWM enabled
- Stable Rust toolchain
- A normal interactive desktop session with windows open

### Build and run

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release
```

To run a named profile:

```bash
cargo run --release -- --profile work
```

The profile file is read from `%APPDATA%\Panopticon\profiles\work.toml`.

## First-run checklist

If you are opening the app for the first time, this is the fastest useful walkthrough:

1. open Panopticon while a few normal applications are already running;
2. cycle layouts with `Tab` or jump directly with `1` to `7`;
3. right-click a thumbnail to inspect app/window actions;
4. hide an app and restore it from the tray;
5. open settings with `O` to review filters, theme, and profile options;
6. try `Row` or `Column` and use the wheel or middle-button drag to navigate overflow.

## Important paths

### Configuration

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\profiles\<profile>.toml
```

If `%APPDATA%` is unavailable, Panopticon falls back to `%TEMP%\Panopticon\...`.

### Logs

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

### UI and assets

```text
ui/main.slint
assets/themes.json
```

## Architecture snapshot

Panopticon is intentionally local and relatively direct. The runtime is built around four pillars:

1. **Win32 enumeration** to discover candidate windows.
2. **DWM thumbnails** to render live previews efficiently.
3. **A pure layout engine** to compute geometry and resizable separators.
4. **A Slint UI layer** to expose the dashboard, settings, and dialogs.

There is no backend, web API, remote persistence, or external service dependency.

## Documentation map

Use this table when you know the question you want answered.

| Document | Read it when you need... |
| --- | --- |
| [`GETTING_STARTED.md`](GETTING_STARTED.md) | installation steps, first-run behaviour, shortcuts, and common issues |
| [`CONFIGURATION.md`](CONFIGURATION.md) | all config keys, profile behaviour, tags, and layout persistence |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | a system view of runtime layers, startup flow, and key design decisions |
| [`IMPLEMENTATION.md`](IMPLEMENTATION.md) | code-level behaviour of modules, state, timers, DWM syncing, tray, and dock |
| [`PROJECT_STRUCTURE.md`](PROJECT_STRUCTURE.md) | where files live, what each folder owns, and which paths are editable |
| [`SYSTEM_INTEGRATIONS.md`](SYSTEM_INTEGRATIONS.md) | Win32/DWM/Shell/GDI usage, dependencies, and operational constraints |
| [`UX_DESIGN.md`](UX_DESIGN.md) | user-facing surfaces, interactions, layout mental model, and visual language |
| [`PRD.md`](PRD.md) | product goals, scope, users, constraints, acceptance criteria, future opportunities |

## Development essentials

The most relevant local checks are:

```bash
cargo check
cargo test
cargo clippy -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

The workspace also includes VS Code tasks for the same commands.

Current automated coverage is strongest around:

- layout behaviour;
- settings normalisation;
- theme logic;
- i18n helpers.

Native integrations such as tray behaviour, DWM registration, Win32 menus, and window enumeration are more dependent on manual/runtime validation.

## Known boundaries

Panopticon currently assumes:

- **Windows only**;
- **DWM availability**;
- **local desktop usage**;
- **no Linux/macOS support**;
- **no remote backend or multi-user collaboration model**.

Dock/appbar mode is also a special runtime mode, so some behaviours intentionally differ from floating-window mode.

## Related project files

- [`../CONTRIBUTING.md`](../CONTRIBUTING.md)
- [`../SECURITY.md`](../SECURITY.md)
- [`../SUPPORT.md`](../SUPPORT.md)
- [`../CHANGELOG.md`](../CHANGELOG.md)
- [`../LICENSE`](../LICENSE)

If you only wanted the short overview, head back to [`../README.md`](../README.md). If you want to keep digging, the documents above are your map instead of a maze.
