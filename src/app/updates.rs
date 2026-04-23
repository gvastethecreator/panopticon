//! Background update checker for GitHub releases.

use std::sync::{
    mpsc::{self, TryRecvError},
    Mutex, OnceLock,
};

use serde::Deserialize;

const LATEST_RELEASE_ENDPOINT: &str =
    "https://api.github.com/repos/gvastethecreator/panopticon/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UpdateCheckOutcome {
    UpToDate {
        latest_version: String,
    },
    Available {
        latest_version: String,
        release_url: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Deserialize)]
struct GitHubReleasePayload {
    tag_name: String,
    html_url: String,
}

fn receiver_slot() -> &'static Mutex<Option<mpsc::Receiver<UpdateCheckOutcome>>> {
    static UPDATE_CHECK_RECEIVER: OnceLock<Mutex<Option<mpsc::Receiver<UpdateCheckOutcome>>>> =
        OnceLock::new();
    UPDATE_CHECK_RECEIVER.get_or_init(|| Mutex::new(None))
}

pub(crate) fn request_latest_release_check(current_version: &str) -> bool {
    let Ok(mut guard) = receiver_slot().lock() else {
        tracing::warn!("unable to lock update-check state");
        return false;
    };

    if guard.is_some() {
        return false;
    }

    let (sender, receiver) = mpsc::channel();
    *guard = Some(receiver);
    drop(guard);

    let current_version = current_version.to_owned();
    std::thread::spawn(move || {
        let _ = sender.send(check_latest_release(&current_version));
    });

    true
}

pub(crate) fn poll_latest_release_check() -> Option<UpdateCheckOutcome> {
    let Ok(mut guard) = receiver_slot().lock() else {
        tracing::warn!("unable to lock update-check state");
        return Some(UpdateCheckOutcome::Failed {
            reason: "internal update-check state unavailable".to_owned(),
        });
    };

    let receiver = guard.as_ref()?;
    match receiver.try_recv() {
        Ok(outcome) => {
            *guard = None;
            Some(outcome)
        }
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            *guard = None;
            Some(UpdateCheckOutcome::Failed {
                reason: "update-check worker disconnected".to_owned(),
            })
        }
    }
}

fn check_latest_release(current_version: &str) -> UpdateCheckOutcome {
    let user_agent = format!("Panopticon/{current_version}");
    let response = match ureq::get(LATEST_RELEASE_ENDPOINT)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", &user_agent)
        .call()
    {
        Ok(response) => response,
        Err(error) => {
            return UpdateCheckOutcome::Failed {
                reason: error.to_string(),
            };
        }
    };

    let payload: GitHubReleasePayload = match response.into_json() {
        Ok(payload) => payload,
        Err(error) => {
            return UpdateCheckOutcome::Failed {
                reason: error.to_string(),
            };
        }
    };

    let latest_version = ensure_version_prefix(&payload.tag_name);
    if is_newer_version(&payload.tag_name, current_version) {
        UpdateCheckOutcome::Available {
            latest_version,
            release_url: payload.html_url,
        }
    } else {
        UpdateCheckOutcome::UpToDate { latest_version }
    }
}

fn ensure_version_prefix(version: &str) -> String {
    let normalized = version.trim();
    if normalized.starts_with('v') || normalized.starts_with('V') {
        normalized.to_owned()
    } else {
        format!("v{normalized}")
    }
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    match (parse_semver_triplet(latest), parse_semver_triplet(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

fn parse_semver_triplet(version: &str) -> Option<(u64, u64, u64)> {
    let trimmed = version.trim();
    let without_prefix = trimmed.trim_start_matches(['v', 'V']);
    let core = without_prefix
        .split_once('-')
        .map_or(without_prefix, |(main, _)| main);

    let mut segments = core.split('.');
    let major = segments.next()?.parse::<u64>().ok()?;
    let minor = segments.next().unwrap_or("0").parse::<u64>().ok()?;
    let patch = segments.next().unwrap_or("0").parse::<u64>().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_semver_triplet};

    #[test]
    fn parse_semver_triplet_accepts_common_git_tags() {
        assert_eq!(parse_semver_triplet("v0.1.21"), Some((0, 1, 21)));
        assert_eq!(parse_semver_triplet("0.2"), Some((0, 2, 0)));
        assert_eq!(parse_semver_triplet("V1.5.7-beta.1"), Some((1, 5, 7)));
    }

    #[test]
    fn parse_semver_triplet_rejects_non_numeric_versions() {
        assert_eq!(parse_semver_triplet("stable"), None);
        assert_eq!(parse_semver_triplet("v1.x.3"), None);
    }

    #[test]
    fn version_comparison_only_flags_strictly_newer_versions() {
        assert!(is_newer_version("v0.1.22", "0.1.21"));
        assert!(!is_newer_version("v0.1.21", "0.1.21"));
        assert!(!is_newer_version("v0.1.20", "0.1.21"));
    }
}
