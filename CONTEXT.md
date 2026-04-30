# Context: Panopticon

## Purpose

This document defines the canonical domain vocabulary for Panopticon. Agents should use these exact terms rather than inventing synonyms.

## Glossary

| Term | Definition |
|------|-----------|
| **ManagedWindow** | A window discovered by Win32 that Panopticon actively tracks, including its DWM thumbnail, target/display rectangles, and metadata. |
| **WindowInfo** | Raw data about a top-level window (hwnd, title, app_id, process name/path, class name, monitor) before it is materialised into a `ManagedWindow`. |
| **DWM Thumbnail** | A live preview of a window rendered by the Windows Desktop Window Manager. Not a manually captured screenshot. |
| **Layout** | A geometry algorithm that distributes window rectangles across the available dashboard area. The seven layouts are: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, `Column`. |
| **LayoutResult** | The pure output of the layout engine: a set of computed rectangles plus draggable separator positions. |
| **AppState** | The centralised runtime state containing the collection of `ManagedWindow`s, current settings, theme, tray icon, dock status, and scroll/selection state. |
| **Workspace** | A persisted profile of settings and per-app rules stored as a TOML file under `%APPDATA%\Panopticon\workspaces\`, loaded via the `--workspace <name>` CLI flag. |
| **Per-app Rule** | A behaviour rule applied to all windows sharing the same `app_id` (hide, aspect ratio, colour, tags, thumbnail refresh mode). |
| **Tag** | A manual label assigned to windows for visual grouping. Tags have an associated colour. |
| **Filter** | An active criterion that reduces the set of visible windows (by monitor, app, title, class, or tag). |
| **Grouping** | The visual ordering of windows by application, monitor, title, or class. |
| **Tray** | The system tray icon mode of operation: persistent background utility with context menu and auto-restore on Explorer restart. |
| **Dock / AppBar** | A mode where the Panopticon window anchors to a screen edge and reserves desktop space via `SHAppBarMessage`. |
| **Theme** | A UI colour scheme, either a preset from `assets/themes.json` or a solid colour, with animated transitions. |
| **Backdrop** | The visual layer behind the dashboard, supporting configurable opacity and background images with fit modes (`cover`, `contain`, `fill`, `preserve`). |
| **Refresh** | The periodic re-enumeration of top-level Win32 windows to reconcile state. Frequency is configurable: `1s`, `2s`, `5s`, `10s`. |
| **Separator** | A draggable resize handle between layout regions that lets the user adjust proportions. Custom ratios are persisted per layout. |
| **Thumbnail Refresh Mode** | How a DWM thumbnail updates: `Realtime` (live), `Frozen` (static), or `Interval` (periodic). |
| **Campbell-first** | The default theme family inspired by Campbell terminal colours. |
| **Enumeration** | The Win32 `EnumWindows` pass that discovers top-level windows and builds `WindowInfo` structs. |
| **Window Subclassing** | The Win32 technique used to intercept native window messages (tray, dock, scroll, hotkeys) that Slint cannot handle declaratively. |

## Terms to avoid

| Don't use | Use instead | Why |
|-----------|------------|-----|
| Screenshot / capture | DWM thumbnail | Panopticon does not capture bitmaps manually |
| Profile | Workspace | "Workspace" is the persisted TOML unit |
| Dashboard view | Layout | "Layout" is the specific algorithm mode |
| Tab | Window / ManagedWindow | "Tab" implies browser tabs, not OS windows |

## Related documents

- `docs/ARCHITECTURE.md` — runtime layers and module map
- `docs/PRD.md` — product goals, scope, and constraints
- `docs/IMPLEMENTATION.md` — module-level implementation details
- `docs/adr/` — architectural decision records
