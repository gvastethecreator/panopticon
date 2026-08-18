# Panopticon certification notes

Copy and adapt this document immediately before submission. Replace bracketed values and update the date.

## Notes for certification

**Notes date:** `[YYYY-MM-DD]`  
**Product:** Panopticon  
**Store ID:** `[STORE ID]`  
**Package version:** `[MAJOR.MINOR.BUILD.REVISION]`  
**Submission commit:** `[GIT SHA]`

Panopticon is a local-first Windows desktop window-overview utility. It enumerates normal top-level windows and displays live previews registered through Windows Desktop Window Manager (DWM). No Panopticon account or credentials are required.

### Recommended test setup

Open several harmless Windows applications before launching Panopticon, for example:

- Notepad with synthetic text;
- Calculator;
- Paint with a blank/generated image;
- File Explorer in a non-sensitive test folder;
- another simple test window with a neutral title.

Do not use windows containing private account, customer, email, chat, browser, source-code, or credential content because their visible contents may appear in DWM previews.

### Basic test path

1. Install the package.
2. Launch **Panopticon** from Start.
3. Confirm normal open top-level windows appear as live previews.
4. Select a preview to activate its source window.
5. Press `1` through `7` or use the UI to switch between Grid, Mosaic, Bento, Fibonacci, Columns, Row, and Column.
6. Open Settings and test language, theme, filter, workspace, and shortcut controls.
7. Hide the app to the system tray and restore it.
8. Test the configured global activation shortcut.
9. If testing appbar/dock mode, enable it, verify positioning, then disable it before uninstall.
10. Close one source window and confirm Panopticon removes the stale preview.

### Expected exclusions

Panopticon intentionally excludes windows that are not appropriate as normal user-facing top-level targets, including Panopticon itself and selected shell/system surfaces. The exact visible set depends on the certification environment and Windows version.

### Data behavior

- Window titles, process/application metadata, class names, monitor geometry, icons, and DWM preview handles are used locally.
- Live thumbnails can visually contain sensitive data from source windows. Panopticon does not intentionally upload or archive them.
- Settings, rules, tags, workspaces, themes, shortcuts, and selected background paths are stored locally.
- No Panopticon account is required.
- The Store build does not start the GitHub Releases update request.
- Core window discovery and preview functionality works offline.

### Update behavior

This package was compiled with:

```text
PANOPTICON_DISTRIBUTION_CHANNEL=store
```

Microsoft Store manages package updates. When the user triggers the in-app update action, Panopticon reports the current version without contacting the GitHub Releases API.

The GitHub/direct distribution is a separate build channel and is not the package submitted here.

### Potentially disruptive actions

Panopticon can activate, minimize, restore, close, move, or arrange source windows when the user explicitly invokes those actions. Use disposable test windows for close/terminate scenarios.

Appbar/dock and global-hotkey features register Windows shell resources while enabled. Normal exit and uninstall must release those registrations.

### Support and privacy

- Privacy policy: `[PUBLIC PRIVACY POLICY URL]`
- Product website: `[PUBLIC PRODUCT URL]`
- Support: `[PUBLIC SUPPORT URL OR EMAIL]`

## Restricted capability: `runFullTrust`

Panopticon is a native Rust/Slint desktop application. It requires `runFullTrust` to perform these desktop operations:

1. Enumerate normal top-level Win32 windows and retrieve metadata needed to identify and filter them.
2. Register and manage live DWM thumbnails for those windows.
3. Activate, restore, minimize, close, move, or arrange a selected source window at the user's request.
4. Register and manage a system-tray icon.
5. Register a configurable global hotkey.
6. Optionally register appbar/dock behavior with the Windows shell.
7. Persist local settings, rules, tags, workspaces, logs, and selected local background paths.
8. Interact with Explorer and normal shell APIs needed to recover from shell restart.

Panopticon does not use `runFullTrust` to elevate silently, bypass Windows access controls, capture credentials, inject code into source applications, upload window contents, or monitor user input entered into other applications.

## Reviewer troubleshooting

### No windows appear

Open normal desktop applications such as Notepad, Calculator, Paint, or File Explorer and press `R` to refresh. Some system, cloaked, tool, or non-user-facing windows are intentionally excluded.

### A preview is blank or stale

DWM preview availability depends on the source window and Windows state. Restore/minimize the source window, refresh Panopticon, or use another normal application for validation.

### The global hotkey does not register

Another application may already own the selected key combination. Choose a different shortcut in Settings and retry.

### Tray icon disappears after Explorer restart

Panopticon handles the Windows `TaskbarCreated` message and should recreate shell resources. Allow a short refresh interval and open the tray overflow area if the icon is not immediately visible.

### Check for updates does not open GitHub

That is expected in the Microsoft Store build. Package updates are managed by Microsoft Store.
