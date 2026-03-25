//! Structured logging configuration for Panopticon.
//!
//! Logs are written to rolling daily files under the system temporary
//! directory (`%TEMP%/panopticon/logs/`).  The [`tracing`] facade is used
//! so that log macros (`info!`, `warn!`, `error!`, etc.) work throughout
//! the application without passing a logger instance.

use std::path::PathBuf;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Returns the directory where Panopticon stores its log files.
#[must_use]
pub fn log_directory() -> PathBuf {
    std::env::temp_dir().join("panopticon").join("logs")
}

/// Initialise the global [`tracing`] subscriber with a daily-rolling file
/// appender.
///
/// Returns a [`WorkerGuard`] that **must** be kept alive for as long as the
/// application runs; dropping it flushes and shuts down the logging
/// background thread.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the
/// subscriber cannot be installed.
pub fn init() -> anyhow::Result<WorkerGuard> {
    let log_dir = log_directory();
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = rolling::daily(&log_dir, "panopticon.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true),
        )
        .init();

    tracing::info!(
        log_dir = %log_dir.display(),
        "Panopticon logging initialised",
    );

    Ok(guard)
}
