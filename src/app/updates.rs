//! Background update checker for GitHub releases.

use std::sync::{
    mpsc::{self, TryRecvError},
    Mutex, OnceLock,
};

use serde::Deserialize;

const LATEST_RELEASE_ENDPOINT: &str =
    "https://api.github.com/repos/gvastethecreator/panopticon/releases/latest";
const RELEASES_ENDPOINT: &str = "https://api.github.com/repos/gvastethecreator/panopticon/releases";

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
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
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
    let payload = match fetch_latest_release_payload(&user_agent) {
        Ok(payload) => payload,
        Err(error) => {
            return UpdateCheckOutcome::Failed { reason: error };
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

fn fetch_latest_release_payload(user_agent: &str) -> Result<GitHubReleasePayload, String> {
    fetch_release_payload(LATEST_RELEASE_ENDPOINT, user_agent)
        .and_then(validate_published_release)
        .or_else(|latest_error| {
            let releases = fetch_release_list(user_agent).map_err(|fallback_error| {
                format!("{latest_error}; fallback list failed: {fallback_error}")
            })?;
            select_latest_published_release(releases).ok_or_else(|| {
                format!("{latest_error}; fallback list returned no published releases")
            })
        })
}

fn fetch_release_payload(url: &str, user_agent: &str) -> Result<GitHubReleasePayload, String> {
    github_request(url, user_agent)?
        .into_json()
        .map_err(|error| error.to_string())
}

fn fetch_release_list(user_agent: &str) -> Result<Vec<GitHubReleasePayload>, String> {
    github_request(RELEASES_ENDPOINT, user_agent)?
        .into_json()
        .map_err(|error| error.to_string())
}

fn github_request(url: &str, user_agent: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", user_agent)
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| error.to_string())
}

fn validate_published_release(
    payload: GitHubReleasePayload,
) -> Result<GitHubReleasePayload, String> {
    if payload.draft || payload.prerelease {
        Err("latest release endpoint returned an unpublished release".to_owned())
    } else {
        Ok(payload)
    }
}

fn select_latest_published_release(
    releases: Vec<GitHubReleasePayload>,
) -> Option<GitHubReleasePayload> {
    releases
        .into_iter()
        .find(|release| !release.draft && !release.prerelease)
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
    use super::{
        is_newer_version, parse_semver_triplet, select_latest_published_release,
        GitHubReleasePayload,
    };

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

    #[test]
    fn fallback_release_selection_skips_drafts_and_prereleases() {
        let selected = select_latest_published_release(vec![
            GitHubReleasePayload {
                tag_name: "v0.2.0-rc.1".to_owned(),
                html_url: "https://example.test/rc".to_owned(),
                draft: false,
                prerelease: true,
            },
            GitHubReleasePayload {
                tag_name: "v0.1.22".to_owned(),
                html_url: "https://example.test/stable".to_owned(),
                draft: false,
                prerelease: false,
            },
        ])
        .expect("published release");

        assert_eq!(selected.tag_name, "v0.1.22");
    }
}
