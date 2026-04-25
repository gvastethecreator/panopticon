# Getting Started

This guide covers how to compile, run, and understand the initial flow of Panopticon without having to read through the entire source code first.

## Requirements

- Windows 10 or Windows 11 (64-bit)
- DWM enabled
- Stable Rust toolchain
- A real desktop with user windows open so Panopticon has something to display

## Cloning and running

### Development mode

```bash
git clone https://github.com/gvastethecreator/panopticon.git
cd panopticon
cargo run
```

### Release mode

```bash
cargo run --release
```

### Running with a specific workspace

```bash
cargo run --release -- --workspace work
```

This loads the configuration from `%APPDATA%\Panopticon\workspaces\work.toml`.

Workspace names become Windows filenames, so keep them free of reserved characters such as `<>:"/\\|?*` and control characters.

You can also use `--workspace=work`, inspect the CLI with `--help`, or print the current app version with `--version`.

## What happens at startup

In a normal session, Panopticon does the following:

1. initialises logging in `%TEMP%\panopticon\logs\`;
2. activates per-monitor DPI awareness;
3. loads settings and the active workspace from TOML;
4. creates the main Slint window;
5. acquires the native `HWND` and applies DWM appearance;
6. registers the tray icon;
7. enumerates visible system windows;
8. registers DWM thumbnails for the visible windows;
9. computes the initial layout and fills the thumbnail model.

If `start_in_tray = true`, the application finishes startup hidden in the tray.

## What you should see

On a correct first start:

- a main window with a bottom status bar;
- dark cards with a colour stripe at the top;
- live thumbnails inside each card;
- an icon in the system tray;
- visible and hidden window counts in the status bar.

If there are no candidate windows, the UI shows an empty state.

## Recommended first walkthrough

1. Press `Tab` to cycle through layouts.
2. Press `1` to `7` to jump to each layout directly.
3. Right-click a thumbnail to open the per-window menu.
4. Hide an application from the layout, then restore it from the tray.
5. Create a tag from an application.
6. Open `Settings` with `O` and review language, filters, theme, background options, shortcuts, and workspaces; click the sidebar mascot to open About and check update status.
7. Try `T` to toggle between themes.
8. Press `F1` while the dashboard is focused to open the About window.
9. If using `Row` or `Column`, scroll with the wheel or middle-button drag.

## Useful shortcuts

These are the default bindings shipped by Panopticon today. The dashboard/global shortcuts can be edited from the settings window; `F1` remains a built-in shortcut for the About window.

| Key / gesture | Result |
| --- | --- |
| `Tab` | next layout |
| `1`...`7` | specific layout |
| `0` | reset custom ratios for the current layout |
| `R` | refresh windows |
| `A` | toggle animations |
| `H` | toggle status bar |
| `I` | toggle window info |
| `P` | toggle always-on-top |
| `T` | change theme |
| `O` | open settings |
| `F1` | open About window |
| `M` | open application menu |
| `Ctrl` + `Alt` + `P` | activate and focus Panopticon globally |
| `Alt` | toggle status bar |
| left-click thumbnail | activate window |
| right-click thumbnail | per-app/window context menu |
| left-click tray | show/hide main window |
| right-click tray | open quick menu |
| `Esc` | exit |

## Files worth looking at early

- [`../README.md`](../README.md) -- short public-facing overview.
- [`README.md`](README.md) -- documentation hub and reading guide.
- [`PRD.md`](PRD.md) -- updated product objectives.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) -- architecture and diagrams.
- [`CONFIGURATION.md`](CONFIGURATION.md) -- all settings keys.
- [`PROJECT_STRUCTURE.md`](PROJECT_STRUCTURE.md) -- repository map.
- [`IMPLEMENTATION.md`](IMPLEMENTATION.md) -- technical details per module.

## Important paths

### Configuration

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\workspaces\<workspace>.toml
```

### Logs

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

### UI and assets

```text
ui/main.slint
assets/themes.json
```

## Local development

The most useful checks during day-to-day work are:

```bash
cargo check
cargo test
cargo clippy -- -D warnings -W clippy::pedantic
cargo fmt -- --check
```

VS Code tasks already exist for these commands in the workspace.

## Common issues

### The app opens but I see no thumbnails

Check:

- that visible user windows exist;
- that you have not left an active filter by monitor, tag, or app;
- that DWM is available;
- that Panopticon is not starting hidden in the tray.

### The tray disappeared after restarting Explorer

The runtime attempts to re-register it automatically when it receives `TaskbarCreated`.

### A workspace does not seem to load

Launch the app with `--workspace <name>` (or `--workspace=<name>`) and verify that `%APPDATA%\Panopticon\workspaces\<name>.toml` exists.

### In dock mode some actions behave differently

This is expected: appbar mode modifies window style, topmost, and effectively forces `hide_on_select` to `false`.
