# Screenshot provenance

The four public captures were recorded on 2026-08-15 from the local Windows release build.

- Runtime data came from four disposable Windows Terminal windows with controlled English titles and text.
- Panopticon ran with an isolated `%APPDATA%` workspace and an exact application filter for Windows Terminal.
- UI Automation verified eight expected thumbnail actions, zero unexpected thumbnails, and zero visible Spanish strings before capture.
- A legacy `language = "spanish"` setting and `PANOPTICON_LANG=es-ES` were deliberately supplied; the product normalized both to English.
- The capture processes, Panopticon process, and isolated fixture windows were stopped after recording.
- Images were resized, padded, and encoded as WebP. The interface and thumbnails were not reconstructed or retouched.
