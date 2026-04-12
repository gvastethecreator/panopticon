//! # Panopticon
//!
//! A real-time window thumbnail viewer for Windows, powered by the
//! Desktop Window Manager (DWM) API.

// Win32 / geometry code requires pervasive integer ↔ float casting.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`layout`] | Mathematical layout algorithms (Grid, Mosaic, Bento, Fibonacci, Columns, Row, Column) |
//! | [`thumbnail`] | RAII wrapper for DWM thumbnail handles |
//! | [`window_enum`] | Discovery and filtering of top-level application windows |
//! | [`error`] | Typed error definitions |
//! | [`constants`] | Application-wide constants |
//! | [`logging`] | Structured logging configuration |
//! | [`settings`] | Persistent user preferences saved as TOML |

pub mod constants;
pub mod error;
pub mod i18n;
pub mod input_ops;
pub mod layout;
pub mod logging;
pub mod settings;
pub mod theme;
pub mod thumbnail;
pub mod ui_option_ops;
pub mod window_enum;
pub mod window_ops;
