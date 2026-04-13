# UX/UI Design

This document describes the intended user experience of Panopticon, what visual surfaces compose it, and what principles are reflected in the current implementation.

## Design intent

Panopticon does not behave like a document or form application. It is designed as a **visual observation and context-switching tool**. The UI must let the user:

- see many windows at the same time;
- quickly identify which app each card represents;
- activate or dismiss windows with few gestures;
- maintain persistent context across sessions.

## Main surfaces

### 1. Main window (`MainWindow`)

This is the product dashboard. It contains:

- optional upper toolbar;
- main area with thumbnails;
- empty state when there are no windows;
- overlay scrollbar in layouts with overflow;
- resize handles for layouts that support persistent ratios.

### 2. Toolbar

The toolbar summarises the dashboard operational state:

- current layout;
- number of visible windows;
- number of hidden apps;
- refresh interval;
- active filters and grouping;
- topmost and animation state.

It also works as an interactive surface:

- click to cycle layouts;
- right-click to open the main menu.

### 3. Thumbnail cards (`ThumbnailCard`)

Each system window is represented as a card with:

- upper accent stripe;
- dark placeholder where the DWM thumbnail is drawn on top;
- optional title and app information;
- optional icon;
- hover/active visual states;
- alternative state for minimised windows.

#### Visible decisions

- the border changes on hover or when the card is active;
- the card does not render the thumbnail itself: it leaves a prepared area for DWM to overlay;
- the design prioritises dark contrast with warm or theme-derived accent.

### 4. Settings window

This is the declarative configuration panel. It is organised by blocks:

- behaviour;
- display;
- theme;
- layout;
- refresh;
- fixed dimensions;
- dock;
- filters;
- background;
- profiles;
- hidden apps;
- shortcuts.

Its UX purpose is to let the user adjust almost everything without manually editing TOML.

### 5. Tag dialog

A compact dialog for creating a tag and assigning it to a specific app. It asks for:

- tag name;
- colour preset;
- create/assign confirmation.

### 6. Tray icon and native menus

Panopticon adopts a "tray-first" model:

- it can remain alive even when the main window is hidden;
- the tray allows reopening, refreshing, filtering, and restoring hidden apps;
- it is the operational centre when using the app persistently throughout an entire session.

## Visual language

### Palette

The classic theme uses a dark base with amber accent, but the theme system allows deriving palettes from `assets/themes.json`.

Main chromatic elements:

- **bg**: general background;
- **toolbar-bg**: upper bar;
- **card-bg / panel-bg / surface**: layered surfaces;
- **accent**: protagonist colour for the active state or group;
- **muted / label / text**: typographic hierarchy levels.

### Contrast and depth

The UI uses:

- stacked dark surfaces;
- thin, subtle borders;
- upper accent stripe;
- faint shadows on overlays;
- a visual language compatible with Windows 11 when `use_system_backdrop` is active.

### Themes

Themes are not merely cosmetic: they affect almost every surface and are interpolated with animation, so the change is not abrupt.

## Layout modes and mental model

| Layout | Mental model | When it is most useful |
| --- | --- | --- |
| `Grid` | regular grid | general overview, many similar items |
| `Mosaic` | rows with ratio-adapted width | mix of wide and tall windows |
| `Bento` | one protagonist window + secondaries | primary focus with lateral context |
| `Fibonacci` | progressive space division | visual exploration and asymmetric compositions |
| `Columns` | balanced columns | dashboard-style or masonry flows |
| `Row` | scrollable horizontal strip | sequential comparison of many windows |
| `Column` | scrollable vertical strip | panel or tall-dock use |

## Main interactions

### Mouse

- left-click on card: activate window;
- right-click on card: open per-window context menu;
- drag on separators: adjust persistent ratios;
- wheel / middle-button pan: scroll layouts with overflow;
- left-click on tray: toggle visibility;
- right-click on tray: open main menu.

### Keyboard

- `1` to `7`: direct layout change;
- `Tab`: next layout;
- `0`: reset ratios;
- `R`: refresh;
- `A`, `H`, `I`, `P`, `T`: quick toggles;
- `O`: settings;
- `M`: main menu;
- `Alt`: toolbar;
- `Esc`: exit.

## Special UI states

### Minimised window

When the source window is minimised, the card does not try to sustain a useful thumbnail and switches to a presentation centred on the icon and app name.

### Empty state

If there are no eligible windows, the UI presents a central empty panel with explanatory text instead of leaving a dead canvas.

### Overlay scrollbar

Instead of permanent scrollbars, Panopticon uses subtle overlays that appear with activity and fade afterwards. This keeps the dashboard clean when there is no overflow.

## UX principles reflected in the implementation

### 1. Observation priority

The main content is the user's windows, not the app's controls. The interface tries to occupy little "visual noise" and gives prominence to the thumbnails.

### 2. Immediate control

Almost all frequent actions are a single gesture or keypress away:

- activate;
- hide;
- refresh;
- change layout;
- filter;
- open settings.

### 3. Context persistence

Panopticon remembers filters, grouping, colours, tags, layouts, and profiles. The UX does not treat each start as a new session without memory.

### 4. Desktop utility, not a document

The presence of tray, dock/appbar, topmost, and start-in-tray clearly shows that the application is designed to coexist with the desktop, not to be opened and closed on demand.

## Design/implementation observations

- the declarative UI defines menu overlays, but the active main flow uses native Win32 menus;
- the size section in settings now serves both dock/appbar thickness and undocked floating window sizing;
- the tag system is well integrated, but still relies heavily on the per-window context menu rather than bulk editing from settings.

## UX evolution opportunities

1. make the profile concept and its startup impact more visible;
2. expose more batch/global editing for advanced options that already exist in runtime, such as per-thumbnail refresh mode, instead of concentrating them in the per-window context menu;
3. decide whether declarative menu overlays should be activated or removed;
4. improve visual communication of dock mode and complex active filters.
