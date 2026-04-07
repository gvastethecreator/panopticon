# Product Requirements Document (PRD)

## Project: Panopticon

**Documented product version:** 0.1.0  
**Document status:** updated to reflect the current implementation  
**Target platform:** Windows 10 / Windows 11 (64-bit)  
**Actual stack:** Rust + Slint + Win32/DWM  
**Product nature:** local desktop utility; no backend or remote services

---

## 1. Executive summary

Panopticon is a visual Windows utility that offers a consolidated view of open desktop windows. The application enumerates top-level windows, creates live thumbnails through the Desktop Window Manager (DWM) API, and presents them in an interactive dashboard with multiple layout algorithms.

The product goal is to let a user switch context, organise windows, and regain visual focus on their workspace faster than with `Alt+Tab` or the traditional taskbar.

---

## 2. Problem it solves

Power desktop users often have many windows open simultaneously. Standard OS mechanisms do not always address these scenarios well:

- the taskbar provides limited context;
- `Alt+Tab` is sequential and not always scalable;
- grouping windows by monitor, application, or task requires multiple manual steps;
- quickly distinguishing minimised windows, hidden processes, or manual groupings is costly.

Panopticon solves this with a persistent, configurable view of the desktop state.

---

## 3. Target users

### 3.1. Primary user

- people who work with many simultaneous windows;
- multi-monitor setup users;
- technical, productivity, or creative profiles that need to supervise multiple apps at once.

### 3.2. Secondary user

- developers who want to group workflows by project;
- operators or analysts who need to keep a visual desktop dashboard;
- power users who prefer tray utilities, profiles, and lightweight automation.

---

## 4. Product goals

### 4.1. Functional goals

1. Display live thumbnails of open windows without manual bitmap captures.
2. Allow visually reordering windows through adaptive layouts.
3. Facilitate activation, hiding, filtering, and grouping from a single surface.
4. Persist preferences and per-application rules across sessions.
5. Maintain a UX consistent with Windows through tray icon, native menus, appbar, and DPI support.

### 4.2. Technical goals

1. Delegate the bulk of the graphics pipeline to DWM.
2. Concentrate layout computation in pure, testable logic.
3. Isolate Win32/FFI interop in scoped, commented `unsafe` blocks.
4. Allow customisation without depending on external infrastructure.

### 4.3. Current non-goals

- real cross-platform support outside Windows;
- cloud synchronisation or remote configuration;
- automation based on advanced rules or scripting;
- analytics, telemetry, or SaaS backend;
- persistent captures, video recording, or temporal window history.

---

## 5. Current functional scope

### 5.1. Window discovery

The product must:

- enumerate top-level windows with `EnumWindows`;
- skip invisible windows, tool windows, non-activatable windows without `WS_EX_APPWINDOW`, windows with an irrelevant owner, and known system surfaces;
- capture per window: `HWND`, title, class, executable path, process name, persistent identifier (`app_id`), and monitor.

### 5.2. Thumbnail visualisation

The product must:

- register a DWM thumbnail per visible managed window;
- update destination rectangles during resize, animation, scroll, and layout changes;
- release the thumbnail when the window no longer applies or when the source is minimised;
- use a visual placeholder and application icon when the source window is minimised.

### 5.3. Layouts

The product must support these layouts:

1. `Grid`
2. `Mosaic`
3. `Bento`
4. `Fibonacci`
5. `Columns`
6. `Row`
7. `Column`

Additionally, it must:

- allow switching layouts by keyboard, toolbar, or tray;
- save the preferred initial layout;
- persist custom ratios when the user drags separators;
- support overflow with horizontal or vertical scroll in `Row` and `Column`.

### 5.4. Window interaction

The product must:

- activate a window when clicking its thumbnail;
- restore a minimised window before focusing it;
- allow closing the target window or terminating its process from the context menu;
- allow hiding an application from the layout without closing the actual process.

### 5.5. Filters, tags, and grouping

The product must:

- filter by monitor;
- filter by manual tag;
- filter by application (`app_id`);
- visually group window order by application, monitor, title, or class;
- allow creating and assigning manual tags from the UI;
- associate colours with tags and specific applications.

### 5.6. Persistence and profiles

The product must:

- save global configuration and per-app rules in TOML;
- support multiple profiles via `--profile <name>`;
- allow saving a profile from the settings window;
- allow opening another instance using a different profile;
- seed default profiles if none exist.

### 5.7. Tray utility and lifecycle

The product must:

- register an icon in the system tray;
- restore it if Explorer restarts (`TaskbarCreated`);
- allow minimising or closing to tray according to configuration;
- start hidden in tray if configured by the user;
- exit cleanly, releasing thumbnails and removing the tray icon.

### 5.8. Visual customisation

The product must:

- apply a classic theme or presets derived from `assets/themes.json`;
- animate theme transitions;
- allow a solid background and a background image behind the dashboard;
- use rounded corners and Windows 11 backdrop when available.

### 5.9. Dock / appbar

The product can dock to a screen edge as an appbar. In that mode it must:

- reserve desktop space with `SHAppBarMessage`;
- reposition itself when the shell or display changes;
- effectively disable `hide_on_select`;
- block certain system menu commands related to moving or closing the window.

---

## 6. Non-functional requirements

### 6.1. Platform

- exclusive support for Windows 10/11;
- explicit dependency on DWM and Win32;
- no administrator privileges required for the general case.

### 6.2. Performance

- graphics composition must be delegated to DWM whenever possible;
- enumeration refresh must be configurable;
- animations must be smooth and time-bounded (`180 ms` for layouts, `220 ms` for themes);
- the UI loop refresh must coexist with the main event loop without blocking the application.

### 6.3. Robustness

- if a window disappears, the associated thumbnail must be removed without crashing;
- if a thumbnail fails to update, it must be regenerated or released with controlled degradation;
- if an icon cannot be generated, a fallback to the system or executable icon must exist.

### 6.4. Security and maintainability

- `unsafe` code must be justified with `SAFETY` comments;
- raw pointers must not be exposed in public crate APIs;
- the layout engine and settings normalisation must remain easily testable.

### 6.5. Observability

- the app must emit structured logs to a local file;
- the log path must be deterministic and easy to inspect during support or debugging.

---

## 7. Constraints and dependencies

### 7.1. Runtime dependencies

- `slint` for the declarative UI;
- `windows` crate for DWM, User32, Shell, GDI, HiDPI, and Threading;
- `raw-window-handle` to obtain the Slint window `HWND`;
- `serde`, `serde_json`, and `toml` for persistence and the theme catalogue;
- `tracing` and `tracing-appender` for logging.

### 7.2. Relevant technical constraints

- the project uses a single-threaded event loop shared with Slint/Win32;
- interaction with elevated windows may be limited by UIPI;
- some capabilities depend on the specific DWM behaviour on the user's system.

---

## 8. Important edge cases

1. **Minimised window**  
    The DWM thumbnail may become useless; Panopticon releases the thumbnail and shows an icon-based visual fallback.

2. **Closed window or terminated process**  
    The next enumeration or update removes the entry and reflows the layout.

3. **Explorer restarted**  
    The tray icon must be re-registered upon receiving `TaskbarCreated`.

4. **Monitors with different DPI**  
    The app must run with `PER_MONITOR_AWARE_V2` and recalculate rectangles with the correct scale factor.

5. **Scrollable layouts**  
    `Row` and `Column` can exceed the viewport; the UI must provide scrolling and an overlay scrollbar.

6. **Dock mode**  
    The window changes its visual role and system constraints; some options behave differently than in floating mode.

---

## 9. Acceptance criteria

The product is considered functionally acceptable when:

1. it compiles and starts on Windows with `cargo run` or `cargo run --release`;
2. it enumerates and displays thumbnails of active windows in at least one layout;
3. it allows activating a window by clicking its thumbnail;
4. it allows changing layout and keeps the dashboard consistent upon refresh;
5. it persists settings and per-app rules across sessions;
6. it supports tray icon, filters, and per-window context menu;
7. it maintains sufficient technical documentation to reproduce, maintain, and extend the project.

---

## 10. Future opportunities

- expand test coverage for Win32/DWM/tray;
- clean up or remove disconnected declarative UI components if they are no longer part of the active runtime;
- better document real performance metrics on Windows;