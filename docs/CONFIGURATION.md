# Configuration

Panopticon stores its preferences in local TOML files. There is no database or remote synchronisation: the persisted configuration belongs to the local project/user on Windows.

## Configuration locations

### Default profile

```text
%APPDATA%\Panopticon\settings.toml
```

### Named profiles

```text
%APPDATA%\Panopticon\profiles\<profile>.toml
```

### Fallback if `%APPDATA%` does not exist

```text
%TEMP%\Panopticon\settings.toml
%TEMP%\Panopticon\profiles\<profile>.toml
```

## General schema

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
active_app_filter = "exe:c:\\program files\\arc\\arc.exe"
group_windows_by = "application"
fixed_width = 320
fixed_height = 220
dock_edge = "left"
theme_id = "nord"
background_color_hex = "181513"
use_system_backdrop = true
show_toolbar = true
show_window_info = true
start_in_tray = false
background_image_path = "C:\\wallpapers\\workspace.png"
background_image_fit = "cover"
locked_layout = false
lock_cell_resize = false
show_app_icons = true

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
refresh_now = "R"
exit_app = "Esc"
alt_toggles_toolbar = true

[tag_styles.work]
color_hex = "D29A5C"

[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
hide_on_select_override = false
tags = ["work", "browser"]
color_hex = "5CA9FF"
thumbnail_refresh_mode = "realtime"
thumbnail_refresh_interval_ms = 5000

[layout_customizations.Grid]
col_ratios = [0.7, 0.3]
row_ratios = [0.4, 0.6]
```

## Global keys

| Key | Type | Default | Runtime effect | Notes |
| --- | --- | --- | --- | --- |
| `initial_layout` | `LayoutType` | `Grid` | active layout at startup | also updated when changing layout at runtime |
| `refresh_interval_ms` | `u32` | `2000` | frequency of `refresh_windows()` | normalised to `1000`, `2000`, `5000`, or `10000` |
| `minimize_to_tray` | `bool` | `true` | minimise hides the app to tray | affects `WM_SIZE` |
| `close_to_tray` | `bool` | `true` | close hides the app to tray | affects `WM_CLOSE` |
| `preserve_aspect_ratio` | `bool` | `false` | global default for thumbnails | can be overridden per app |
| `hide_on_select` | `bool` | `true` | hides Panopticon when activating an app | effectively disabled in dock mode |
| `animate_transitions` | `bool` | `true` | animates layout changes | current duration: `180 ms` |
| `always_on_top` | `bool` | `false` | uses `SetWindowPos(HWND_TOPMOST)` | also affects secondary dialogs |
| `active_monitor_filter` | `Option<String>` | `None` | filters windows by monitor | persistent across sessions |
| `active_tag_filter` | `Option<String>` | `None` | filters windows by tag | mutually exclusive with `active_app_filter` |
| `active_app_filter` | `Option<String>` | `None` | filters windows by app | mutually exclusive with `active_tag_filter` |
| `group_windows_by` | `WindowGrouping` | `None` | reorders visible windows | does not filter; only groups/sorts |
| `fixed_width` | `Option<u32>` | `None` | lateral dock thickness or floating window width | when undocked and set, it resizes the main window width at runtime |
| `fixed_height` | `Option<u32>` | `None` | top/bottom dock thickness or floating window height | when undocked and set, it resizes the main window height at runtime |
| `dock_edge` | `Option<DockEdge>` | `None` | activates appbar mode | values: `left`, `right`, `top`, `bottom` |
| `theme_id` | `Option<String>` | `None` | selects a preset from `assets/themes.json` | `None` = classic theme |
| `background_color_hex` | `String` | `181513` | base client colour | also participates in the classic theme fallback |
| `use_system_backdrop` | `bool` | `true` | backdrop + rounded corners on Windows 11 | via `DwmSetWindowAttribute` |
| `show_toolbar` | `bool` | `true` | show/hide upper header | also changes the usable viewport area |
| `show_window_info` | `bool` | `true` | shows title/app on the thumbnail | affects the usable thumbnail height |
| `start_in_tray` | `bool` | `false` | starts hidden | releases thumbnails before hiding |
| `background_image_path` | `Option<String>` | `None` | draws an image behind the dashboard | silently cleared on load failure |
| `background_image_fit` | `BackgroundImageFit` | `Cover` | scales the dashboard background image | values: `cover`, `contain`, `fill`, `preserve` |
| `locked_layout` | `bool` | `false` | locks layout changes | disables shortcuts and toolbar for layouts |
| `lock_cell_resize` | `bool` | `false` | locks separator dragging | can coexist with `locked_layout` |
| `show_app_icons` | `bool` | `true` | shows icons on cards | uses cache + GDI rasterisation |
| `shortcuts` | `ShortcutBindings` | built-in defaults | defines the dashboard key map | single-key bindings plus `Tab`, `Esc`, `Enter`, `Space`; invalid values fall back |
| `layout_customizations` | `Map<String, LayoutCustomization>` | empty | custom ratios per layout | generated when dragging separators |
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
refresh_now = "R"
exit_app = "Esc"
alt_toggles_toolbar = true
```

Important notes:

- supported bindings are **single keys** or the named special keys `Tab`, `Esc`, `Enter`, and `Space`;
- invalid or multi-key expressions such as `Ctrl+T` are normalised back to the default binding;
- `alt_toggles_toolbar` is a separate compatibility switch for the Win32 `Alt` toolbar toggle and is not part of the general shortcut parser.

## Per-application rules

Each `app_rules` entry is keyed by `app_id`. The most common format is:

```toml
[app_rules."exe:c:\\program files\\arc\\arc.exe"]
display_name = "Arc"
hidden = false
preserve_aspect_ratio = true
hide_on_select = false
hide_on_select_override = false
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
| `preserve_aspect_ratio` | `bool` | overrides the global default |
| `hide_on_select` | `bool` | inherited/legacy value |
| `hide_on_select_override` | `Option<bool>` | explicit modern override |
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

Notes:

- `Grid` uses `col_ratios` and `row_ratios`.
- `Mosaic` primarily persists `row_ratios`.
- `Bento` uses a main column ratio and sidebar ratios.
- `Columns` uses column ratios.
- `Row` and `Column` persist per-item ratios on the scrollable axis.
- `Fibonacci` does not use persistent separator customisation.

## Profiles

Panopticon supports separate per-file profiles.

### How they work

- the app can start with `--profile <name>`;
- the CLI also accepts `--profile=<name>` for the same behaviour;
- `--help` prints the available startup flags and `--version` prints the current app version;
- the settings window allows saving the current state to another profile;
- it also allows opening another instance with a different profile;
- if no extra profiles exist, the runtime seeds `profile-1` and `profile-2`.

## Automatic normalisation

Before entering the runtime, `AppSettings::normalized()` corrects several cases:

- refresh intervals outside the allowed set;
- empty filters;
- empty or duplicate tags;
- invalid hex colours;
- unsupported shortcut bindings (including multi-key expressions);
- profile names not valid for Windows;
- rules with an empty `app_id`;
- orphaned tag styles;
- legacy `hide_on_select` inheritance towards the modern override model.

## Important relationships and exclusions

### Filters

- activating `active_tag_filter` clears `active_app_filter`;
- activating `active_app_filter` clears `active_tag_filter`;
- `active_monitor_filter` can coexist with the above.

### Dock

- if `dock_edge` is active, `hide_on_select_for(app)` returns `false` at runtime even if an override exists;
- `fixed_width` / `fixed_height` act as dock thickness while docked and as requested floating size while undocked.

### Themes

- `theme_id = None` uses the classic theme;
- if `theme_id` does not exist in `assets/themes.json`, the runtime falls back to the classic theme.
