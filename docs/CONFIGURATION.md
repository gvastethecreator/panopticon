# Configuration

Panopticon stores its preferences in local TOML files. There is no database or remote synchronisation: the persisted configuration belongs to the local project/user on Windows.

## Configuration locations

### Default workspace

```text
%APPDATA%\Panopticon\settings.toml
```

### Named workspaces

```text
%APPDATA%\Panopticon\workspaces\<workspace>.toml
```

### Fallback if `%APPDATA%` does not exist

```text
%TEMP%\Panopticon\settings.toml
%TEMP%\Panopticon\workspaces\<workspace>.toml
```

## General schema

```toml
language = "english"
initial_layout = "grid"
refresh_interval_ms = 2000
refresh_performance_mode = "balanced"
minimize_to_tray = true
close_to_tray = true
preserve_aspect_ratio = false
hide_on_select = true
animate_transitions = true
always_on_top = false
center_secondary_windows = true
active_monitor_filter = "DISPLAY1"
active_tag_filter = "work"
active_app_filter = "exe:c:\\program files\\arc\\arc.exe"
group_windows_by = "application"
fixed_width = 320
fixed_height = 220
dock_edge = "left"
dock_column_thickness = 320
dock_row_thickness = 180
theme_id = "campbell"
background_color_hex = "0C0C0C"
show_toolbar = true
toolbar_position = "bottom"
show_window_info = true
start_in_tray = false
run_at_startup = false
background_image_path = "C:\\wallpapers\\workspace.png"
background_image_fit = "cover"
background_image_opacity_pct = 25
thumbnail_render_scale_pct = 100
locked_layout = false
lock_cell_resize = false
show_app_icons = true
dismissed_empty_state_welcome = false

[workspace]
display_name = "Design Focus"
description = "Rules and filters for UX review sessions"
created_unix_ms = 1714235200123
updated_unix_ms = 1714239000456
schema_version = 1

[theme_color_overrides]
accent_hex = "5CA9FF"
surface_hex = "202020"
card_hex = "181818"
text_hex = "F5F5F5"
muted_hex = "999999"
border_hex = "444444"

[shortcuts]
layout_grid = "1"
layout_mosaic = "2"
layout_bento = "3"
layout_fibonacci = "4"
layout_columns = "5"
layout_row = "6"
layout_column = "7"
reset_layout = "0"
cycle_layout = "Tab"
cycle_theme = "T"
toggle_animations = "A"
toggle_toolbar = "H"
toggle_window_info = "I"
toggle_always_on_top = "P"
open_settings = "O"
open_menu = "M"
open_command_palette = "K"
global_activate = "Ctrl+Alt+P"
refresh_now = "R"
exit_app = "Esc"
alt_toggles_toolbar = true

[tag_styles.work]
color_hex = "D29A5C"

[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
preserve_aspect_ratio_override = true
hide_on_select = false
hide_on_select_override = false
pinned_position = 2
tags = ["work", "browser"]
color_hex = "5CA9FF"
thumbnail_refresh_mode = "realtime"
thumbnail_refresh_interval_ms = 5000

[layout_customizations.Grid]
col_ratios = [0.7, 0.3]
row_ratios = [0.4, 0.6]

[layout_presets."Focus Grid"]
layout = "grid"

[layout_presets."Focus Grid".customization]
col_ratios = [0.7, 0.3]
row_ratios = [0.6, 0.4]
```

## Global keys

| Key | Type | Default | Runtime effect | Notes |
| --- | --- | --- | --- | --- |
| `language` | `Locale` | `English` | selects the application UI language | supported values: `english`, `spanish`; `PANOPTICON_LANG` can override it at runtime |
| `initial_layout` | `LayoutType` | `Grid` | active layout at startup | also updated when changing layout at runtime |
| `refresh_interval_ms` | `u32` | `2000` | frequency of `refresh_windows()` | normalised to `1000`, `2000`, `5000`, or `10000` |
| `refresh_performance_mode` | `RefreshPerformanceMode` | `balanced` | high-level refresh preset for global discovery cadence | `realtime`→`1000`, `balanced`→`2000`, `battery-saver`→`5000`, `manual` keeps `refresh_interval_ms` |
| `minimize_to_tray` | `bool` | `true` | minimise hides the app to tray | affects `WM_SIZE` |
| `close_to_tray` | `bool` | `true` | close hides the app to tray | affects `WM_CLOSE` |
| `preserve_aspect_ratio` | `bool` | `false` | global default for thumbnails | can be overridden per app |
| `hide_on_select` | `bool` | `true` | hides Panopticon when activating an app | effectively disabled in dock mode |
| `animate_transitions` | `bool` | `true` | animates layout changes | current duration: `180 ms` |
| `always_on_top` | `bool` | `false` | uses `SetWindowPos(HWND_TOPMOST)` | also affects secondary dialogs |
| `center_secondary_windows` | `bool` | `true` | centers secondary windows on open | affects Settings/About/Command Palette/tag dialogs when opened |
| `active_monitor_filter` | `Option<String>` | `None` | filters windows by monitor | persistent across sessions |
| `active_tag_filter` | `Option<String>` | `None` | filters windows by tag | mutually exclusive with `active_app_filter` |
| `active_app_filter` | `Option<String>` | `None` | filters windows by app | mutually exclusive with `active_tag_filter` |
| `group_windows_by` | `WindowGrouping` | `None` | reorders visible windows | does not filter; only groups/sorts |
| `fixed_width` | `Option<u32>` | `None` | floating window width | when undocked and set, it resizes the main window width at runtime; values are clamped to at least `320` |
| `fixed_height` | `Option<u32>` | `None` | floating window height | when undocked and set, it resizes the main window height at runtime; values are clamped to at least `220` |
| `dock_edge` | `Option<DockEdge>` | `None` | activates appbar mode | values: `left`, `right`, `top`, `bottom`; runtime forces `Column` on left/right and `Row` on top/bottom |
| `dock_column_thickness` | `Option<u32>` | `None` | dock thickness for left/right mode | clamped to at least `180` when set |
| `dock_row_thickness` | `Option<u32>` | `None` | dock thickness for top/bottom mode | clamped to at least `120` when set |
| `theme_id` | `Option<String>` | `Some("campbell")` | selects a preset from `assets/themes.json` | new profiles default to `campbell`; `None` = classic theme |
| `background_color_hex` | `String` | `0C0C0C` | base client colour | defaults to Campbell's background for new profiles; classic still uses this as its fallback background |
| `theme_color_overrides` | `ThemeColorOverrides` | empty | optional manual overrides for core theme slots | supported keys: `accent_hex`, `surface_hex`, `card_hex`, `text_hex`, `muted_hex`, `border_hex`; blank/invalid values are discarded during normalization |
| `show_toolbar` | `bool` | `true` | show/hide the status bar | also changes the usable viewport area |
| `toolbar_position` | `ToolbarPosition` | `bottom` | places the status bar on top or bottom | values: `top`, `bottom`; only matters when `show_toolbar = true` |
| `show_window_info` | `bool` | `true` | shows title/app on the thumbnail | affects the usable thumbnail height |
| `start_in_tray` | `bool` | `false` | starts hidden | releases thumbnails before hiding |
| `run_at_startup` | `bool` | `false` | registers Panopticon in the current user Windows startup sequence | implemented through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` |
| `background_image_path` | `Option<String>` | `None` | draws an image behind the dashboard | silently cleared on load failure |
| `background_image_fit` | `BackgroundImageFit` | `Cover` | scales the dashboard background image | values: `cover`, `contain`, `fill`, `preserve` |
| `background_image_opacity_pct` | `u8` | `25` | controls the dashboard background-image opacity | clamped to `0..=100`; `0` keeps the file configured but makes it visually transparent |
| `thumbnail_render_scale_pct` | `u8` | `100` | reduces DWM thumbnail detail to trade sharpness for performance | normalized to the discrete set `25`, `50`, `75`, `100` |
| `locked_layout` | `bool` | `false` | locks layout changes | disables shortcuts and menu-driven layout changes |
| `lock_cell_resize` | `bool` | `false` | locks separator dragging | can coexist with `locked_layout` |
| `show_app_icons` | `bool` | `true` | shows icons on cards | uses cache + GDI rasterisation |
| `dismissed_empty_state_welcome` | `bool` | `false` | hides the first-run empty-state welcome card once dismissed | persisted immediately when the user closes the welcome hint |
| `shortcuts` | `ShortcutBindings` | built-in defaults | defines the dashboard key map | dashboard bindings stay single-key; `global_activate` accepts `Ctrl` / `Alt` / `Shift` chords and empty disables it |
| `workspace` | `WorkspaceMetadata` | empty | stores friendly metadata for workspace management UI | keys: `display_name`, `description`, `created_unix_ms`, `updated_unix_ms`, `schema_version` |
| `layout_customizations` | `Map<String, LayoutCustomization>` | empty | custom ratios per layout | generated when dragging separators |
| `layout_presets` | `Map<String, LayoutPreset>` | empty | named snapshots of layout mode + custom ratios | saved/applied/deleted from Settings > Advanced |
| `app_rules` | `Map<String, AppRule>` | empty | persistent per-app rules | key = `app_id` |
| `tag_styles` | `Map<String, TagStyle>` | empty | manual tag colours | normalised against actually existing tags |

## Shortcut bindings

Keyboard bindings are persisted under the nested `[shortcuts]` table.

```toml
[shortcuts]
layout_grid = "1"
layout_mosaic = "2"
layout_bento = "3"
layout_fibonacci = "4"
layout_columns = "5"
layout_row = "6"
layout_column = "7"
reset_layout = "0"
cycle_layout = "Tab"
cycle_theme = "T"
toggle_animations = "A"
toggle_toolbar = "H"
toggle_window_info = "I"
toggle_always_on_top = "P"
open_settings = "O"
open_menu = "M"
open_command_palette = "K"
global_activate = "Ctrl+Alt+P"
refresh_now = "R"
exit_app = "Esc"
alt_toggles_toolbar = true
```

Important notes:

- dashboard bindings are **single keys** or the named special keys `Tab`, `Esc`, `Enter`, and `Space`;
- `cycle_theme = "T"` moves forward, while pressing `Shift+T` at runtime cycles back to the previous theme;
- invalid dashboard expressions such as `Ctrl+T` are normalised back to the default binding;
- `global_activate` is optional, accepts `Ctrl` / `Alt` / `Shift` plus a final key such as `P`, `Space`, or `F12`, and clearing it disables the global hotkey;
- `alt_toggles_toolbar` is a separate compatibility switch for the Win32 `Alt` status-bar toggle and is not part of the general shortcut parser.

## Language and locale

- `language = "english"` is the persisted default for every new or migrated workspace;
- `language = "spanish"` switches the full Slint UI, native dialogs, tray tooltip, and runtime labels to Spanish;
- the optional `PANOPTICON_LANG` environment variable (`en`, `es`, `en-US`, `es-MX`, etc.) takes precedence over the saved value for the current process only.

## Theme colour overrides

Core theme slots can be overridden explicitly under the `[theme_color_overrides]` table.

```toml
[theme_color_overrides]
accent_hex = "5CA9FF"
surface_hex = "202020"
card_hex = "181818"
text_hex = "F5F5F5"
muted_hex = "999999"
border_hex = "444444"
```

Notes:

- every value must be a 6-digit RGB hex string;
- empty or invalid values are dropped during normalization;
- unspecified keys continue using the active preset value;
- changing `theme_id` keeps the manual overrides unless you clear them from the settings UI or TOML.

## Per-application rules

Each `app_rules` entry is keyed by `app_id`. The most common format is:

```toml
[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
preserve_aspect_ratio_override = true
hide_on_select = false
hide_on_select_override = false
pinned_position = 2
tags = ["work", "browser"]
color_hex = "5CA9FF"
thumbnail_refresh_mode = "interval"
thumbnail_refresh_interval_ms = 5000
```

### `AppRule` fields

| Field | Type | Usage |
| --- | --- | --- |
| `display_name` | `String` | friendly label in menus and restore |
| `hidden` | `bool` | excludes the app from the visible layout |
| `preserve_aspect_ratio` | `bool` | inherited/legacy stored value |
| `preserve_aspect_ratio_override` | `Option<bool>` | explicit modern override |
| `hide_on_select` | `bool` | inherited/legacy value |
| `hide_on_select_override` | `Option<bool>` | explicit modern override |
| `pinned_position` | `Option<usize>` | preferred slot index (0-based) used by runtime pin ordering |
| `tags` | `Vec<String>` | manual tags for grouping/filtering |
| `color_hex` | `Option<String>` | fixed card colour |
| `thumbnail_refresh_mode` | `ThumbnailRefreshMode` | `realtime`, `frozen`, `interval` |
| `thumbnail_refresh_interval_ms` | `Option<u32>` | used when the mode is `interval` |

## Tags and styles

Manual tags are stored in `tag_styles` and assigned to apps via `app_rules.<app>.tags`.

```toml
[tag_styles.work]
color_hex = "D29A5C"

[tag_styles.browser]
color_hex = "5CA9FF"
```

### Important behaviour

- tags are normalised to lowercase;
- duplicates and extra whitespace are removed;
- if a tag is no longer used in `app_rules`, its style may disappear after normalisation;
- when `active_tag_filter` is active, the tag colour is used as the default dashboard accent.

## Layout customisations

Layouts with persistable separators write their ratios under `layout_customizations`.

```toml
[layout_customizations.Columns]
col_ratios = [0.2, 0.5, 0.3]

[layout_customizations.Row]
col_ratios = [0.15, 0.35, 0.2, 0.3]
```

Named snapshots can also be persisted under `layout_presets`:

```toml
[layout_presets."Focus Grid"]
layout = "grid"

[layout_presets."Focus Grid".customization]
col_ratios = [0.7, 0.3]
row_ratios = [0.6, 0.4]
```

Notes:

- `Grid` uses `col_ratios` and `row_ratios`.
- `Mosaic` primarily persists `row_ratios`.
- `Bento` uses a main column ratio and sidebar ratios.
- `Columns` uses column ratios.
- `Row` and `Column` persist per-item ratios on the scrollable axis.
- `Fibonacci` does not use persistent separator customisation.

## Workspaces

Panopticon supports separate per-file workspaces.

### How they work

- the app can start with `--workspace <name>`;
- the CLI also accepts `--workspace=<name>` for the same behaviour;
- `--help` prints the available startup flags and `--version` prints the current app version;
- interactive workspace names must be valid Windows filename stems, so avoid `<>:"/\\|?*` and control characters;
- the settings window allows saving the current state to another workspace;
- the settings window supports workspace lifecycle operations (save, duplicate, rename, delete);
- workspace metadata is persisted in the `[workspace]` table (`display_name`, `description`, timestamps);
- it also allows opening another instance with a different workspace;
- if no extra workspaces exist, the runtime seeds `workspace-1` and `workspace-2`.

## Workspace metadata

Every workspace TOML file can include a `[workspace]` section used by the Settings > Profiles page.

```toml
[workspace]
display_name = "Design Focus"
description = "Pinned apps and filters for UI review"
created_unix_ms = 1714235200123
updated_unix_ms = 1714239000456
schema_version = 1
```

Notes:

- `display_name` and `description` are optional and trimmed during normalization;
- timestamps are Unix milliseconds (`u64`) updated on save;
- `schema_version` is reserved for metadata migrations.

## Automatic normalisation

Before entering the runtime, `AppSettings::normalized()` corrects several cases:

- refresh intervals outside the allowed set;
- fixed refresh mode mappings (`realtime`, `balanced`, `battery-saver`) override the explicit interval with their canonical cadence;
- empty filters;
- empty or duplicate tags;
- invalid hex colours;
- floating/dock dimensions lower than the configured safe minimums;
- unsupported shortcut bindings (including multi-key expressions);
- workspace names not valid for Windows;
- rules with an empty `app_id`;
- hidden app rules automatically drop `pinned_position`;
- `layout_presets` entries with empty names or empty ratio snapshots are discarded;
- orphaned tag styles;
- legacy inherited per-app copies of `preserve_aspect_ratio` and `hide_on_select`, so old global defaults do not stay pinned unless an explicit `*_override` exists.

## Important relationships and exclusions

### Filters

- activating `active_tag_filter` clears `active_app_filter`;
- activating `active_app_filter` clears `active_tag_filter`;
- `active_monitor_filter` can coexist with the above.

### Dock

- if `dock_edge` is active, `hide_on_select_for(app)` returns `false` at runtime even if an override exists;
- when `dock_edge` is `left` or `right`, the effective runtime layout is forced to `Column`;
- when `dock_edge` is `top` or `bottom`, the effective runtime layout is forced to `Row`;
- `fixed_width` / `fixed_height` are now reserved for floating mode sizing;
- `dock_column_thickness` only applies to left/right dock mode and `dock_row_thickness` only applies to top/bottom dock mode.

### Themes

- `theme_id = None` uses the classic theme;
- new workspaces start with `theme_id = "campbell"` and a matching `background_color_hex = "0C0C0C"`;
- if `theme_id` does not exist in `assets/themes.json`, the runtime falls back to the classic theme.
