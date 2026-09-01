<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/header/document.svg?title=Panopticon&subtitle=Every+open+window.+One+live+control+room.&logo=windows&theme=red&align=center&mode=dark" />
    <img alt="Panopticon — every open window in one live control room" src="https://shieldcn.dev/header/document.svg?title=Panopticon&subtitle=Every+open+window.+One+live+control+room.&logo=windows&theme=red&align=center&mode=light" />
  </picture>
</p>

<p align="center">
  <a href="https://github.com/gvastethecreator/panopticon/actions/workflows/ci.yml"><img alt="CI status" src="https://shieldcn.dev/github/ci/gvastethecreator/panopticon.svg?workflow=ci&branch=master&variant=secondary&size=xs" /></a>
  <a href="https://gvastethecreator.github.io/panopticon/"><img alt="Project site" src="https://shieldcn.dev/badge/site-pages-bd321f.svg?logo=githubpages&variant=branded&size=xs" /></a>
  <a href="https://rustup.rs/"><img alt="Rust 1.96" src="https://shieldcn.dev/badge/Rust-1.96-000000.svg?logo=rust&variant=branded&size=xs" /></a>
  <a href="https://github.com/gvastethecreator/panopticon/stargazers"><img alt="GitHub stars" src="https://shieldcn.dev/github/stars/gvastethecreator/panopticon.svg?variant=secondary&size=xs" /></a>
  <a href="LICENSE"><img alt="MIT license" src="https://shieldcn.dev/github/license/gvastethecreator/panopticon.svg?variant=secondary&size=xs" /></a>
</p>

*Desktop utility for viewing, organizing, and activating your open windows through live thumbnails on Windows 10/11.*

Panopticon discovers real top-level windows, renders their live previews, and lets you manage them in a single control room.

[Project site](https://gvastethecreator.github.io/panopticon/) · [Latest release](https://github.com/gvastethecreator/panopticon/releases/latest) · [Source and issues](https://github.com/gvastethecreator/panopticon)

- 👁️ See many windows at once without constantly Alt+Tabbing.
- 👁️ Switch among several layout strategies depending on the task.
- 👁️ Keep a persistent visual workspace with filters, grouping, and tags.
- 👁️ Hide the app in the tray and bring it back instantly when needed.
- 👁️ Local-first, with no accounts or telemetry. Direct builds use a bounded GitHub release update check; Store builds rely on Microsoft Store updates.
- 👁️ **7 layout modes**: `Grid`, `Mosaic`, `Bento`, `Fibonacci`, `Columns`, `Row`, and `Column`.
- 👁️ **Per-app rules** for hiding, aspect ratio, color, tags, and thumbnail refresh mode.
- 👁️ **Grouping and filters** by app, monitor, title, class, and tag.
- 👁️ **Tray utility + appbar/dock mode** for always-available workflows.
- 👁️ **Campbell-first themes, core color overrides, backdrop opacity, background images with fit modes + opacity, animations, customizable shortcuts, workspaces, and persistence** through local TOML files.
- 👁️ **English-only UI** across the dashboard, settings, tray, dialogs, and command palette.

---
> For the full guide, open **[`docs/README.md`](docs/README.md)**.

## Product tour

The dashboard captures use four controlled Windows Terminal fixtures in an isolated workspace. No personal window title or content is included.

| Grid control room | Bento layout |
| --- | --- |
| <img src="docs/assets/screenshots/grid-dashboard.webp" alt="Panopticon grid showing four controlled live Windows Terminal thumbnails" /> | <img src="docs/assets/screenshots/bento-dashboard.webp" alt="Panopticon Bento layout with one large and three supporting live thumbnails" /> |
| **English Settings** | **Command Palette** |
| <img src="docs/assets/screenshots/settings-english.webp" alt="Panopticon Behavior and Display settings in English" /> | <img src="docs/assets/screenshots/command-palette.webp" alt="Panopticon command palette listing layout commands in English" /> |

See [capture provenance](docs/assets/screenshots/README.md) for the isolation and privacy boundary.

## Download

- **Latest release:** [github.com/gvastethecreator/panopticon/releases/latest](https://github.com/gvastethecreator/panopticon/releases/latest)
- **Build from source:** see [Quick start](#quick-start)

## Quick start

```bash
cargo run --locked
```

For an optimized build, use `cargo run --release --locked`. VS Code users can run `♻️ CI` from [`.vscode/tasks.json`](.vscode/tasks.json) for the full check sequence.

## Microsoft Store preparation

The Store channel has a separate MSIX identity, package builder, update policy, listing copy, privacy policy, and qualification workflow. See [`docs/store/README.md`](docs/store/README.md).

The Store package is blocked until Panopticon is reserved in Partner Center and the exact identity is applied. The direct ZIP/Inno release remains a separate channel and requires publisher-owned Authenticode signing for trusted public distribution.

See [`PRIVACY.md`](PRIVACY.md) for the bilingual policy covering window metadata, DWM previews, local storage, logs, and channel-specific network behavior.

## First minute with Panopticon

1. Launch the app with a few normal desktop windows already open.
2. Press `Tab` or `1` to `7` to explore the available layouts.
3. Left-click a thumbnail to activate that window.
4. Right-click a thumbnail to open per-window actions.
5. Press `O` to open settings and review theme, filters, and workspaces.
6. Use the tray icon to hide or show the dashboard without closing it.

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
- For usage and configuration, see the [docs](docs/README.md) folder.
- For Microsoft Store packaging, use the [Store runbook](docs/store/README.md).
- For feature requests and suggestions, create an issue or submit a PR.
- If you like this project, consider giving it a star or becoming a sponsor.
---

<h4 align="right">Support the further development of this tool 🤍</h4>
<p align="right">
  <a href="https://github.com/sponsors/gvastethecreator/"><img src="https://shieldcn.dev/badge/%E2%9D%A4-sponsor%20this%20project-red.svg?animate=pulse" alt="Sponsor this project" /></a>
  <a href="https://x.com/gvastebb"><img src="https://shieldcn.dev/x/mention/gvastebb.svg?variant=branded" alt="Follow me on X" /></a>
</p>
