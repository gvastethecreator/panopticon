//! Structured logging configuration for Panopticon.
//!
//! Logs are written to rolling daily files under the system temporary
//! directory (`%TEMP%/panopticon/logs/`).  The [`tracing`] facade is used
//! so that log macros (`info!`, `warn!`, `error!`, etc.) work throughout
//! the application without passing a logger instance.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const LOG_RETENTION: Duration = Duration::from_hours(720);

#[derive(Debug, Default, PartialEq, Eq)]
struct LogRetentionSummary {
    removed: usize,
    kept: usize,
}

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
    let retention_result = prune_expired_logs(&log_dir, SystemTime::now());

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
    match retention_result {
        Ok(summary) => tracing::info!(
            removed = summary.removed,
            kept = summary.kept,
            retention_days = LOG_RETENTION.as_secs() / 86_400,
            "Panopticon log retention applied"
        ),
        Err(error) => tracing::warn!(%error, "failed to apply Panopticon log retention"),
    }

    Ok(guard)
}

fn prune_expired_logs(log_dir: &Path, now: SystemTime) -> std::io::Result<LogRetentionSummary> {
    let mut summary = LogRetentionSummary::default();
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };

        if !file_type.is_file() || file_type.is_symlink() || !is_owned_daily_log_name(file_name) {
            continue;
        }

        let modified = entry.metadata()?.modified()?;
        let is_expired = now
            .duration_since(modified)
            .is_ok_and(|age| age > LOG_RETENTION);
        if is_expired {
            std::fs::remove_file(entry.path())?;
            summary.removed += 1;
        } else {
            summary.kept += 1;
        }
    }
    Ok(summary)
}

fn is_owned_daily_log_name(file_name: &str) -> bool {
    let Some(date) = file_name.strip_prefix("panopticon.log.") else {
        return false;
    };
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use std::fs::{File, FileTimes};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::{is_owned_daily_log_name, prune_expired_logs};

    #[test]
    fn owned_log_selector_is_exact() {
        assert!(is_owned_daily_log_name("panopticon.log.2026-08-14"));
        assert!(!is_owned_daily_log_name("panopticon.log"));
        assert!(!is_owned_daily_log_name("other.log.2026-08-14"));
        assert!(!is_owned_daily_log_name("panopticon.log.2026-8-14"));
        assert!(!is_owned_daily_log_name("panopticon.log.2026-08-14.bak"));
    }

    #[test]
    fn retention_removes_only_expired_owned_daily_logs() {
        let fixture = std::env::temp_dir().join(format!(
            "panopticon-log-retention-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        std::fs::create_dir(&fixture).expect("create fixture");

        let old_owned = fixture.join("panopticon.log.2020-01-01");
        let old_neighbor = fixture.join("other.log.2020-01-01");
        let current_owned = fixture.join("panopticon.log.2026-08-14");
        for path in [&old_owned, &old_neighbor, &current_owned] {
            File::create(path).expect("create fixture file");
        }

        let old_time = UNIX_EPOCH + Duration::from_hours(24);
        let old_times = FileTimes::new().set_modified(old_time);
        File::options()
            .write(true)
            .open(&old_owned)
            .expect("open owned fixture")
            .set_times(old_times)
            .expect("age owned fixture");
        File::options()
            .write(true)
            .open(&old_neighbor)
            .expect("open neighbor fixture")
            .set_times(old_times)
            .expect("age neighbor fixture");

        let summary = prune_expired_logs(&fixture, SystemTime::now()).expect("prune fixture");
        assert_eq!(summary.removed, 1);
        assert_eq!(summary.kept, 1);
        assert!(!old_owned.exists());
        assert!(old_neighbor.exists());
        assert!(current_owned.exists());

        std::fs::remove_file(old_neighbor).expect("remove neighbor fixture");
        std::fs::remove_file(current_owned).expect("remove current fixture");
        std::fs::remove_dir(fixture).expect("remove fixture directory");
    }
}
