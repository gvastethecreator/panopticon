# Implementation

This document describes how Panopticon is implemented today: what each module does, how state moves through the runtime, and what practical decisions sustain the visible functionality.

## General overview

The main binary is concentrated in `src/main.rs`, which orchestrates:

- main window creation and lifecycle;
- periodic timers;
- synchronisation between state, Slint UI, and DWM thumbnails;
- integration with tray, dock/appbar, native menus, and secondary dialogs;
- settings and workspace persistence.

The implementation relies on several specialised modules to keep all the logic from being tied directly to Win32.

## Binary bootstrap

The `main()` startup follows this conceptual order:

1. initialise logging;
2. read `--workspace` if present;
3. activate DPI awareness and register `TaskbarCreated`;
4. create application icons;
5. load settings from TOML;
6. resolve the active theme;
7. create the Slint `MainWindow`;
8. synchronise initial properties from `AppSettings`;
9. create the shared `AppState` (`Rc<RefCell<_>>`);
10. show the window and register callbacks;
11. defer initialisation that depends on the real `HWND`;
12. start the recurring timers.

This deferred-initialisation pattern is important: Slint needs to create the window first so that Rust can then extract its `HWND` and attach the Win32 pieces.

## Main state

### `ManagedWindow`

Each window visible in the dashboard is modelled as an enriched structure combining:

- persistable metadata (`WindowInfo`);
- optional DWM thumbnail;
- target/display rectangles for animation;
- thumbnail source size;
- timestamps and caches for refresh;
- rasterised icon for fallback and headers.

### `AppState`

`AppState` is the central runtime aggregate. It contains:

- main `hwnd`;
- `windows: Vec<ManagedWindow>`;
- current layout;
- hover and active window;
- tray icon;
- loaded settings;
- animation state;
- scroll and content state;
- separators and drag state;
- loaded background image;
- current theme and theme transition.

In practical terms, `AppState` is the operational source of truth for the program.

## Window enumeration and reconciliation

`refresh_windows()` is one of the implementation centres. Its job is to:

1. call `enumerate_windows()`;
2. discard Panopticon's own window;
3. refresh known friendly app names in `settings`;
4. apply persistent filters by monitor, tag, app, and `hidden`;
5. sort the result if `group_windows_by` is active;
6. reconcile the current `ManagedWindow` vector with the new discovery;
7. create or retain thumbnails as appropriate.

### `window_enum.rs`

This module implements the `EnumWindows` callback and obtains:

- title;
- class;
- process/executable path;
- stable `app_id`;
- monitor.

The `app_id` is constructed with a specific priority:

1. `exe:<lowercase executable path>`
2. `class:<class>`
3. `title:<title>`

This makes per-application rules relatively stable across sessions.

## DWM thumbnails

### RAII wrapper

`src/thumbnail.rs` encapsulates the `HTHUMBNAIL` handle and guarantees `DwmUnregisterThumbnail` in `Drop`.

### Registration and update

`main.rs` decides when to create or release a `Thumbnail`:

- created when a managed window needs to be displayed and does not yet have a thumbnail;
- released if the app is hidden, if the source window is minimised, or if the update fails.

### Per-app refresh modes

The implementation supports three modes in settings:

- `Realtime`
- `Frozen`
- `Interval`

Although the current UI does not expose a direct visual editor for these modes, the runtime respects them if present in the TOML.

### DWM rectangle computation

`compute_dwm_rect()` translates the logical Slint card to the physical DWM rectangle taking into account:

- internal padding;
- upper accent stripe;
- informational band if title/icon is shown;
- viewport for scroll;
- toolbar height;
- DPI scale factor;
- preserve-aspect ratio.

## Layout engine

`src/layout.rs` is the project's geometry engine.

### What it returns

`compute_layout_custom()` produces:

- `rects`: destination rectangles per window;
- `separators`: logical handles to allow persistent resize.

### Available layouts

- `Grid`
- `Mosaic`
- `Bento`
- `Fibonacci`
- `Columns`
- `Row`
- `Column`

### Ratio persistence

When the user drags a separator:

1. `main.rs` ensures base ratios exist for that layout;
2. updates `layout_customizations`;
3. normalises settings;
4. persists the result to disk;
5. recomposes the UI immediately.

For strip layouts (`Row`, `Column`) and columns, the project uses a grouped adjustment (`apply_separator_drag_grouped`) so that the change redistributes more naturally.

## Slint UI

`ui/main.slint` defines three main windows:

- `MainWindow`
- `SettingsWindow`
- `TagDialogWindow`

It also defines the reusable components that structure the UX:

- `ThumbnailCard`
- `ResizeHandle`
- `Toolbar`
- `OverlayScrollbar`
- `ContextMenuOverlay`
- `EmptyState`

### Important note about menus

The main menu interaction is resolved through native Win32 menus implemented in `src/app/tray.rs` and `src/app/window_menu.rs`. The declarative UI maintains the dashboard and dialogs, but the real menu flow no longer depends on Slint overlays.

## Keyboard and mouse interaction

The implementation supports direct shortcuts in `handle_key()`:

- layout selection;
- customisation reset;
- animation, toolbar, info, and topmost toggles;
- settings and main menu opening;
- manual refresh;
- theme change;
- exit.

The key map is no longer hard-coded only in the handler itself: it is driven by `settings.shortcuts`, normalised by `ShortcutBindings::normalized()`, and surfaced in the settings window as editable single-key bindings.

Additionally, the Win32 subclass intercepts the wheel and middle button to enable pan/scroll in layouts with overflow.

## Settings, workspaces, and dialogs

### `settings.rs`

This module defines:

- `AppSettings`
- `AppRule`
- `BackgroundImageFit`
- `ShortcutBindings`
- `TagStyle`
- `ThumbnailRefreshMode`
- `WindowGrouping`
- `DockEdge`

It also handles:

- default and per-workspace paths;
- workspace listing;
- TOML save/load;
- data normalisation;
- business helpers (`toggle_hidden`, `toggle_app_tag`, `set_tag_filter`, etc.).

### `settings_ui.rs`

This is the adapter layer between `AppSettings` and `SettingsWindow`. It translates enums, background-fit modes, theme previews, and editable shortcut strings to the Slint settings UI and back.

### Settings window

The settings window allows editing:

- general behaviour;
- display;
- theme presets and preview grid;
- default layout;
- refresh interval;
- fixed dimensions;
- dock edge;
- filters;
- background image path and fit mode;
- keyboard shortcuts and the optional `Alt` toolbar toggle;
- profiles;
- hidden apps.

`secondary_windows.rs` also wires a native `rfd::FileDialog` picker for browsing and clearing the dashboard background image.

### Multiple workspaces

The app can:

- save the current state to a workspace;
- launch another instance with `--workspace`;
- display a label for the current workspace.

## Tray and native menus

### `src/app/tray.rs`

Manages:

- tray icon creation with `Shell_NotifyIconW`;
- re-registration after Explorer restart;
- tray context menu;
- in-memory icon generation;
- icon extraction from windows or executables.

### `src/app/window_menu.rs`

Defines the per-window menu, with actions such as:

- hide from layout;
- toggle preserve-aspect;
- toggle hide-on-select;
- create custom tag;
- change card colour;
- close window;
- kill process.

## Dock / appbar

Dock mode is implemented inside `main.rs` using `SHAppBarMessage`.

When activated:

- the window changes to `WS_POPUP | WS_VISIBLE` style;
- registers as an appbar;
- reserves desktop space;
- repositions on environment changes;
- certain system menu commands are blocked;
- `hide_on_select` is effectively invalidated.

### Important nuance

`fixed_width` and `fixed_height` act as dock thickness while the appbar is active, and as requested floating window dimensions while the main window is undocked.

## Theming and iconography

### `theme.rs`

Builds a `UiTheme` from:

- a preset embedded in `assets/themes.json`, or
- the classic theme derived from `background_color_hex`.

It also implements interpolation between themes for smooth transitions.

### Icon rendering

The app can:

- request the icon from the window itself;
- request it from the Win32 class;
- extract it from the executable;
- rasterise it to RGBA for Slint using GDI.

This allows showing icons on cards and placeholders for minimised windows.

## Logging

`logging.rs` configures `tracing` with a daily rolling appender. The logger stays alive for the entire process lifetime via `WorkerGuard`.

## Quality and testing

### Existing coverage

- `tests/layout_tests.rs` covers the layout engine;
- `src/settings.rs` tests TOML persistence, normalisation, and rules;
- `src/theme.rs` validates the catalogue and interpolation.

### Current gaps

There is no comparable coverage for:

- tray icon and native menus;
- real Win32 enumeration;
- DWM thumbnails;
- appbar/dock;
- icon extraction and GDI rendering.

## What would benefit from future refactoring

1. split `main.rs` by responsibility;
2. extract a more explicit state/action layer;
3. further decouple menu and dialog logic;
4. clarify or remove disconnected declarative pieces from the Slint UI;
5. close the gap between persisted settings and options actually exposed in the UI.
