# Panopticon documentation

![Panopticon preview](assets/panopticon.webp)

This folder is the documentation hub. The root [`README.md`](../README.md) is the short public landing page.

## What Panopticon is

Panopticon is a native Windows application that shows your open windows as **live DWM thumbnails** in a single dashboard. It is a local productivity utility: a control room where you can scan, group, filter, activate, and customize open applications without screenshots, cloud services, or a background server.

- Live previews powered by the Windows compositor
- Multiple mathematical layouts for arranging windows
- Per-application rules for visibility, color, tags, and refresh strategy
- Tray-first behavior and optional appbar/dock mode
- Persistent settings and named workspaces
- An English-only Slint UI across the dashboard, settings, tray, dialogs, and command palette

## Choose your path

### If you want to use the app

1. [`GETTING_STARTED.md`](GETTING_STARTED.md)
2. [`CONFIGURATION.md`](CONFIGURATION.md)

### If you want to contribute or package a Store build

- [`../CONTRIBUTING.md`](../CONTRIBUTING.md)
- [`store/README.md`](store/README.md)

## Quick overview

### What happens in a normal session

When Panopticon starts, it typically:

1. initializes logging;
2. loads the active configuration/workspace;
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
| Persistence | Saves global settings, per-app rules, tag styles, and layout customizations |
| Theming | Loads themes from `assets/themes.json` and supports animated transitions |
| Docking | Can operate as an appbar on a screen edge |

## Running Panopticon

### Requirements

- Windows 10 or Windows 11 (64-bit)
- DWM enabled
- Rust toolchain from the pinned `../rust-toolchain.toml` file
- A normal interactive desktop session with windows open

### Build and run

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run --release --locked
```

To run a named workspace:

```bash
cargo run --release --locked -- --workspace work
```

The workspace file is read from `%APPDATA%\Panopticon\workspaces\work.toml`.

## First-run checklist

1. Open Panopticon while a few normal applications are already running.
2. Cycle layouts with `Tab` or jump directly with `1` to `7`.
3. Right-click a thumbnail to inspect app/window actions.
4. Hide an app and restore it from the tray.
5. Open settings with `O` to review filters, theme, and workspace options, then click the sidebar mascot to open About/update details.
6. Try `Row` or `Column` and use the wheel or middle-button drag to navigate overflow.

## Important paths

### Configuration

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\workspaces\<workspace>.toml
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

## Credits

- UI iconography includes assets from [HugeIcons](https://hugeicons.com/).

## How it works

Panopticon is local and relatively direct. The runtime is built around four pillars:

1. **Win32 enumeration** to discover candidate windows.
2. **DWM thumbnails** to render live previews efficiently.
3. **A pure layout engine** to compute geometry and resizable separators.
4. **A Slint UI layer** to expose the dashboard, settings, and dialogs.

There is no backend, web API, remote persistence, or external service dependency.

## Documentation map

| Document | Read it when you need... |
| --- | --- |
| [`GETTING_STARTED.md`](GETTING_STARTED.md) | installation steps, first-run behavior, shortcuts, and common issues |
| [`CONFIGURATION.md`](CONFIGURATION.md) | all config keys, workspace behavior, tags, and layout persistence |
| [`store/README.md`](store/README.md) | Microsoft Store packaging and Partner Center workflow |
| [`assets/screenshots/README.md`](assets/screenshots/README.md) | public screenshot provenance |

## Development essentials

```bash
cargo check --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings -W clippy::pedantic
cargo fmt -- --check
cargo build --release --locked
cargo doc --no-deps --locked
```

The root `Justfile` wraps the same commands, including `just ci` for the complete local gate. VS Code users can run the equivalent sequence from [`../.vscode/tasks.json`](../.vscode/tasks.json) with `♻️ CI`.

Current automated coverage is strongest around layout behavior, settings normalization, theme logic, and i18n helpers.

Native integrations such as tray behavior, DWM registration, Win32 menus, and window enumeration depend more on manual or runtime validation.

## Known boundaries

Panopticon currently assumes:

- **Windows only**
- **DWM availability**
- **local desktop usage**
- **no Linux/macOS support**
- **no remote backend or multi-user collaboration model**

Dock/appbar mode is a special runtime mode, so some behaviors differ from floating-window mode.

## Related project files

- [`../CONTRIBUTING.md`](../CONTRIBUTING.md)
- [`../SECURITY.md`](../SECURITY.md)
- [`../SUPPORT.md`](../SUPPORT.md)
- [`../CHANGELOG.md`](../CHANGELOG.md)
- [`../LICENSE`](../LICENSE)

For the short overview, go back to [`../README.md`](../README.md).
