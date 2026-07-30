# Panopticon
> <img src="assets/floppy-small.webp" align="right"/>
![Rust](https://shieldcn.dev/badge/Rust-000000.svg?logo=rust&logoColor=fff&variant=branded&size=xs) ![Runs on Windows](https://shieldcn.dev/badge/Runs%20on-Windows%2011-0078D4.svg?logo=windows&logoColor=fff&size=xs) ![GitHub CI](https://shieldcn.dev/github/ci/gvastethecreator/panopticon.svg?variant=secondary&size=xs) [![License](https://www.shieldcn.dev/github/license/gvastethecreator/panopticon.svg?variant=secondary&size=xs)](LICENSE)

*Desktop utility for viewing, organising, and activating your open windows through live thumbnails on Windows 10/11.*

It discovers real top-level windows, renders their live previews, and lets you manage them in a single control room.

- 👁️ see many windows at once without constantly alt-tabbing.
- 👁️ switch between several layout strategies depending on the task.
- 👁️ keep a persistent visual workspace with filters, grouping, and tags.
- 👁️ hide the app in the tray and bring it back instantly when needed.
- 👁️ fully local, no cloud or external services.
- 👁️ **7 layout modes**: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, and `Column`.
- 👁️ **Per-app rules** for hiding, aspect ratio, color, tags, and thumbnail refresh mode.
- 👁️ **Grouping and filters** by app, monitor, title, class, and tag.
- 👁️ **Tray utility + appbar/dock mode** for always-available workflows.
- 👁️ **Campbell-first themes, core colour overrides, backdrop opacity, background images with fit modes + opacity, animations, customizable shortcuts, workspaces, and persistence** through local TOML files.
- 👁️ **Bilingual UI** with English and Spanish support.

---
> If you want the full guide jump to **[`docs/README.md`](docs/README.md)**.

## 💾 Download

- **Latest release:** [github.com/gvastethecreator/panopticon/releases/latest](https://github.com/gvastethecreator/panopticon/releases/latest)
- **Build from source:** see [Quick start](#quick-start)

## Quick start

```bash
cargo run --locked
```

For an optimised build use `cargo run --release --locked`. VS Code users can run `♻️ ci` from [`.vscode/tasks.json`](.vscode/tasks.json) for the full check sequence.



## First minute with Panopticon

1. Launch the app with a few normal desktop windows already open.
2. Press `Tab` or `1` to `7` to explore the available layouts.
3. Left-click a thumbnail to activate that window.
4. Right-click a thumbnail to open per-window actions.
5. Press `O` to open settings and review language, theme, filters, and workspaces.
6. Use the tray icon to hide/show the dashboard without closing it.

### Handy shortcuts


| Input | Action |
| --- | --- |
| `Tab` | Next layout |
| `1` ... `7` | Select layout directly |
| `0` | Reset custom ratios for the current layout |
| `R` | Refresh windows |
| `A` | Toggle animations |
| `H` | Show/hide status bar |
| `I` | Show/hide window metadata |
| `P` | Toggle always-on-top |
| `T` | Next theme |
| `Shift` + `T` | Previous theme |
| `O` | Open settings |
| `F1` | Open About window |
| `M` | Open application menu |
| `Ctrl` + `Alt` + `P` | Activate and focus Panopticon globally |
| `Alt` | Toggle status bar |
| `Esc` | Exit |

---
- For deep technical details, check the [docs](docs/README.md) folder.
- For feature requests and suggestions, create an issue or submit a PR.
- If you like this project, consider giving it a star or becoming a sponsor.
---

<h4 align="right">Support the further development of this tool 🤍</h4>
<p align="right">
  <a href="https://github.com/sponsors/gvastethecreator/"><img src="https://shieldcn.dev/badge/%E2%9D%A4-sponsor%20this%20project-red.svg?animate=pulse" alt="Sponsor this project" /></a>
  <a href="https://x.com/gvastebb"><img src="https://shieldcn.dev/x/mention/gvastebb.svg?variant=branded" alt="Follow on X" /></a>
</p>
