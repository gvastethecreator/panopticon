//! Typed error definitions for the Panopticon application.
//!
//! Uses [`thiserror`] for ergonomic error derives. Library code returns
//! [`PanopticonError`]; the binary entry point converts to [`anyhow::Error`]
//! for top-level reporting.

use thiserror::Error;

/// Errors that can occur during Panopticon operations.
#[derive(Debug, Error)]
pub enum PanopticonError {
    /// The Win32 window class could not be registered.
    #[error("failed to register window class")]
    ClassRegistration,

    /// The main application window could not be created.
    #[error("failed to create main window")]
    WindowCreation,

    /// A DWM thumbnail operation failed.
    #[error("DWM thumbnail error: {0}")]
    Thumbnail(#[from] windows::core::Error),

    /// Logging subsystem failed to initialise.
    #[error("logging initialisation failed: {0}")]
    Logging(String),
}

/// Convenience alias for `Result<T, PanopticonError>`.
pub type Result<T> = std::result::Result<T, PanopticonError>;
