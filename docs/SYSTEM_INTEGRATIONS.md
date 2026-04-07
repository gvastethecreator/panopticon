# System Integrations and Dependencies

Panopticon does not depend on external services. Its primary integration is with the Windows operating system itself and a small set of crates from the Rust ecosystem.

## Executive summary

- **Remote backend:** none
- **Database:** none
- **HTTP API:** none
- **System services:** yes, several Windows subsystems
- **Local persistence:** TOML files and disk logs

## Windows APIs and subsystems used

### Desktop Window Manager (DWM)

| API | Usage in Panopticon |
| --- | --- |
| `DwmRegisterThumbnail` | register live thumbnails between windows |
| `DwmUpdateThumbnailProperties` | move, show/hide, and adapt each thumbnail |
| `DwmUnregisterThumbnail` | release resources when destroying or dropping thumbnails |
| `DwmQueryThumbnailSourceSize` | query the source window size |
| `DwmSetWindowAttribute` | apply dark mode, backdrop, and rounded corners |

**Functional role:** product core. Without DWM, Panopticon loses its main live-thumbnail proposition.

### User32 / Windows and Messaging

| API | Usage |
| --- | --- |
| `EnumWindows` | discover top-level windows |
| `GetWindowTextW` / `GetClassNameW` | obtain window metadata |
| `GetWindowLongW` / `GetWindowLongPtrW` / `SetWindowLongPtrW` | read styles and subclass the main window |
| `SetForegroundWindow` / `ShowWindow` | activate and restore windows |
| `PostMessageW(WM_CLOSE)` | request graceful window close |
| `SetWindowPos` | topmost, repositioning, docking, and centering |
| `TrackPopupMenu` / `AppendMenuW` | build native menus |
| `RegisterWindowMessageW("TaskbarCreated")` | detect Explorer restart |

**Functional role:** window control, events, and native behaviour.

### Shell / AppBar / System Tray

| API | Usage |
| --- | --- |
| `Shell_NotifyIconW` | register/remove tray icon |
| `SHAppBarMessage` | dock/appbar mode |
| `ExtractIconExW` | extract icons from executables |

**Functional role:** turns Panopticon into a persistent desktop utility, not just another window.

### GDI

| API | Usage |
| --- | --- |
| `GetDC`, `CreateCompatibleDC`, `CreateDIBSection` | create a temporary surface to rasterise icons |
| `DrawIconEx` | paint icons to an intermediate buffer |
| `SelectObject`, `DeleteDC` | GDI resource management |
| `GetMonitorInfoW`, `MonitorFromWindow` | monitor geometry and dock/DPI support |

**Functional role:** complementary visual support, primarily for icons and screen geometry.

### HiDPI

| API | Usage |
| --- | --- |
| `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` | correct calculation on monitors with different scales |

### Threading / Process introspection

| API | Usage |
| --- | --- |
| `OpenProcess` | query executable path or terminate process |
| `QueryFullProcessImageNameW` | obtain the source binary path |
| `TerminateProcess` | "Kill process" menu action |

## Relevant Rust dependencies

| Crate | Role |
| --- | --- |
| `slint` | main declarative UI framework |
| `slint-build` | compile `ui/main.slint` at build time |
| `windows` | official Win32/DWM/Shell/GDI bindings |
| `raw-window-handle` | access the Slint-generated `HWND` |
| `serde` | settings serialisation/deserialisation |
| `serde_json` | theme catalogue loading |
| `toml` | configuration file persistence |
| `tracing` | structured logging |
| `tracing-subscriber` | subscriber and logging filters |
| `tracing-appender` | rolling file writer |
| `thiserror` | typed crate errors |
| `anyhow` | convenient application-level errors |

## File system integration

### Settings

Panopticon stores settings in:

```text
%APPDATA%\Panopticon\settings.toml
%APPDATA%\Panopticon\profiles\<profile>.toml
```

### Logs

```text
%TEMP%\panopticon\logs\panopticon.log.YYYY-MM-DD
```

### Embedded or locally read assets

| Path | Usage |
| --- | --- |
| `assets/themes.json` | base theme catalogue |
| `ui/main.slint` | UI compiled at build time |
| `background_image_path` | optional image loaded from disk by the user |

## Build integration

`build.rs` runs:

```rust
slint_build::compile("ui/main.slint")
```

This means that any syntax or binding error in Slint becomes part of the normal Cargo compilation pipeline.

## Security and `unsafe`

Panopticon uses `unsafe` out of necessity, not as a general style. The integrations that require it are:

- FFI callbacks (`EnumWindows`, Win32 subclass);
- DWM thumbnail management;
- handle, icon, and native menu manipulation;
- GDI operations;
- DWM window attributes.

### General criteria observable in the code

- relatively small `unsafe` blocks;
- `SAFETY` comments in critical areas;
- safe wrappers where it makes sense, such as `Thumbnail`.

## Operational implications

### What the system must allow

- DWM active;
- real Windows graphical environment;
- normal access to user windows.

### System limitations

- UIPI may limit interactions with elevated windows;
- certain behaviours depend on the Windows compositor and the source window state;
- the tray depends on Explorer; hence the re-registration logic.

## What the project does not use

It is worth stating explicitly because it helps understand the project's nature:

- no HTTP client;
- no cloud services;
- no network IPC;
- no SQL/NoSQL database;
- no remote telemetry;
- no authentication or user accounts.

Panopticon is, in essence, a local utility rich in Windows integration and fairly austere in dependencies outside that world.