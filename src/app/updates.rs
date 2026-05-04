//! Background update checker for GitHub releases.

use std::sync::{
    mpsc::{self, TryRecvError},
    Mutex, OnceLock,
};

use serde::Deserialize;
use windows::core::{w, PCWSTR};
use windows::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
    WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
};

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
    let body = http_get(LATEST_RELEASE_ENDPOINT, user_agent)?;
    let payload: GitHubReleasePayload =
        serde_json::from_str(&body).map_err(|error| error.to_string())?;
    validate_published_release(payload)
        .map_err(|error| format!("{error}; fallback to release list"))
        .or_else(|latest_error| {
            let list_body = http_get(RELEASES_ENDPOINT, user_agent)?;
            let releases: Vec<GitHubReleasePayload> =
                serde_json::from_str(&list_body).map_err(|error| error.to_string())?;
            select_latest_published_release(releases).ok_or_else(|| {
                format!("{latest_error}; fallback list returned no published releases")
            })
        })
}

/// Convert a Rust string to a null-terminated wide-string buffer.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// RAII guard for `WinHTTP` HINTERNET handles (raw `*mut std::ffi::c_void`).
struct HttpHandle(*mut std::ffi::c_void);

impl Drop for HttpHandle {
    fn drop(&mut self) {
        // SAFETY: handle is always valid until we close it.
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}

/// Perform a synchronous HTTP GET via `WinHTTP`.
///
/// # Errors
///
/// Returns an error string if any `WinHTTP` step fails.
fn http_get(url: &str, user_agent: &str) -> Result<String, String> {
    let (host, path, is_https) = parse_url(url)?;

    let user_agent_wide = to_wide(user_agent);
    let host_wide = to_wide(&host);
    let path_wide = to_wide(&path);

    // SAFETY: WinHTTP APIs are thread-safe; we synchronously open, request, read, and close.
    unsafe {
        // 1. Open a WinHTTP session.
        let session = WinHttpOpen(
            PCWSTR(user_agent_wide.as_ptr()),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        );
        if session.is_null() {
            return Err("WinHttpOpen failed".to_owned());
        }
        let _session_guard = HttpHandle(session);

        // 2. Connect to the server.
        let port = if is_https { 443 } else { 80 };
        let connect = WinHttpConnect(session, PCWSTR(host_wide.as_ptr()), port, 0);
        if connect.is_null() {
            return Err("WinHttpConnect failed".to_owned());
        }
        let _connect_guard = HttpHandle(connect);

        // 3. Open the request.
        let flags = if is_https {
            WINHTTP_FLAG_SECURE
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };
        let request = WinHttpOpenRequest(
            connect,
            w!("GET"),
            PCWSTR(path_wide.as_ptr()),
            None,
            PCWSTR::null(),
            std::ptr::null(),
            flags,
        );
        if request.is_null() {
            return Err("WinHttpOpenRequest failed".to_owned());
        }
        let _request_guard = HttpHandle(request);

        // 4. Build and send headers.
        let headers = "Accept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n";
        let headers_wide: Vec<u16> = headers.encode_utf16().collect();
        WinHttpSendRequest(
            request,
            Some(&headers_wide),
            None,
            headers_wide.len() as u32,
            0,
            0,
        )
        .map_err(|error| format!("WinHttpSendRequest failed: {error}"))?;

        // 5. Receive the response.
        WinHttpReceiveResponse(request, std::ptr::null_mut())
            .map_err(|error| format!("WinHttpReceiveResponse failed: {error}"))?;

        // 6. Read the response body.
        let mut body = Vec::new();
        loop {
            let mut available: u32 = 0;
            WinHttpQueryDataAvailable(request, &raw mut available)
                .map_err(|error| format!("WinHttpQueryDataAvailable failed: {error}"))?;
            if available == 0 {
                break;
            }
            let mut chunk = vec![0u8; available as usize];
            let mut read: u32 = 0;
            WinHttpReadData(
                request,
                chunk.as_mut_ptr().cast::<std::ffi::c_void>(),
                available,
                &raw mut read,
            )
            .map_err(|error| format!("WinHttpReadData failed: {error}"))?;
            chunk.truncate(read as usize);
            body.extend_from_slice(&chunk);
        }

        String::from_utf8(body).map_err(|error| format!("invalid UTF-8 in response: {error}"))
    }
}

/// Extract host, path, and HTTPS flag from a URL.
/// Supports `http://host/path` and `https://host/path`.
fn parse_url(url: &str) -> Result<(String, String, bool), String> {
    let rest = url
        .strip_prefix("https://")
        .map(|r| (r, true))
        .or_else(|| url.strip_prefix("http://").map(|r| (r, false)))
        .ok_or_else(|| "URL must start with http:// or https://".to_owned())?;

    let (host_and_port, path) = rest.0.split_once('/').unwrap_or((rest.0, "/"));
    let host = host_and_port
        .split_once(':')
        .map_or(host_and_port, |(h, _)| h);

    Ok((host.to_owned(), format!("/{path}"), rest.1))
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
