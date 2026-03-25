# Panopticon — Window Viewer

[![Build](https://img.shields.io/badge/build-cargo-orange)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)]()
[![License](https://img.shields.io/badge/license-MIT-green)]()

A real-time window thumbnail viewer for Windows, powered by the Desktop Window Manager (DWM) API.
Panopticon renders hardware-accelerated live previews of every open window, organised in five mathematical layout modes.

## Features

- **🖼️ Real-time thumbnails** — GPU-composited via DWM; zero bitmap capture, zero CPU rendering.
- **📐 5 layout modes** — Grid, Mosaic, Bento, Fibonacci, Columns.
- **🖱️ Click-to-activate** — Click any thumbnail to bring that window to the foreground.
- **📏 Per-Monitor DPI Aware** — Correct rendering on mixed-DPI multi-monitor setups.
- **📝 Structured logging** — Logs written to `%TEMP%/panopticon/logs/` via `tracing`.
- **⚡ Low footprint** — < 1 % CPU idle, < 50 MB RAM.

## Requirements

| Requirement | Version |
|---|---|
| OS | Windows 10 / 11 (64-bit) |
| Rust toolchain | 1.82+ (edition 2021) |
| DWM | Enabled (default on Windows 10+) |

## Installation

```bash
# Clone the repository
git clone https://github.com/<user>/panopticon.git
cd panopticon

# Build (release, optimised)
cargo build --release
```

The binary is located at `target/release/panopticon.exe`.

## Usage

```bash
# Run directly
cargo run --release

# Or execute the binary
./target/release/panopticon.exe
```

### Controls

| Input | Action |
|---|---|
| **Tab** | Cycle to the next layout mode |
| **R** | Refresh the window list manually |
| **Click** on thumbnail | Activate (bring to front) the selected window |
| **Click** on toolbar | Switch layout mode |
| **Esc** | Exit Panopticon |

### Layout Modes

| Mode | Description |
|---|---|
| **Grid** | Equal-sized cells in a √n × √n grid |
| **Mosaic** | Rows with aspect-ratio-weighted column widths |
| **Bento** | Primary window (60 %) + sidebar stack |
| **Fibonacci** | Golden-ratio spiral subdivision |
| **Columns** | Masonry-style shortest-column-first |

## Development

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install just (task runner) — optional but recommended
cargo install just
```

### Common Tasks

```bash
just build       # 🔨 Debug build
just release     # 🚀 Release build
just check       # ✅ Type check
just lint        # 🧹 Clippy (pedantic)
just fmt         # 🎨 Format code
just test        # 🧪 Run tests
just coverage    # 📊 Coverage report (requires cargo-tarpaulin)
just doc         # 📖 Open generated docs
just ci          # 🔄 Full CI check (fmt + lint + test)
```

### Manual Commands

```bash
cargo clippy -- -D warnings -W clippy::pedantic
cargo fmt -- --check
cargo test
cargo doc --no-deps --open
```

## Architecture

```
src/
├── lib.rs          — Library root: re-exports all modules
├── main.rs         — Win32 window, message loop, painting, state
├── constants.rs    — Colours, timers, key codes, geometry
├── error.rs        — Typed errors (thiserror)
├── layout.rs       — Layout engine (Grid, Mosaic, Bento, Fibonacci, Columns)
├── logging.rs      — tracing + file appender setup
├── thumbnail.rs    — RAII wrapper for DWM HTHUMBNAIL
└── window_enum.rs  — Window enumeration and filtering
tests/
└── layout_tests.rs — Integration tests for the layout engine
docs/
└── ARCHITECTURE.md — Detailed architecture documentation
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for an in-depth design overview.

## Logging

Panopticon writes structured logs to:

```
%TEMP%\panopticon\logs\panopticon.log
```

Set the `RUST_LOG` environment variable to control verbosity:

```bash
set RUST_LOG=debug
./target/release/panopticon.exe
```

## Contributing

1. Fork the repository.
2. Create a feature branch: `git checkout -b feat/my-feature`.
3. Ensure all checks pass: `just ci`.
4. Submit a pull request.

## License

MIT
