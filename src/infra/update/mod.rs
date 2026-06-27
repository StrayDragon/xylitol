//! Version checking against a remote API.
//!
//! The update URL can be configured via the `XYLITOL_UPDATE_URL` environment variable.
//! If not set, uses the placeholder constant from `constants.rs`.

use super::constants::{
    ENV_OFFLINE, ENV_SKIP_VERSION_CHECK, ENV_UPDATE_URL, HTTP_USER_AGENT, VERSION_CHECK_URL,
};

/// Information about a new version.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub package_name: Option<String>,
    pub note: Option<String>,
}

/// Get the update URL, checking environment variable first.
fn get_update_url() -> String {
    std::env::var(ENV_UPDATE_URL).unwrap_or_else(|_| VERSION_CHECK_URL.to_string())
}

/// Check for a newer version of the application.
pub async fn check_for_new_version(current: &str) -> Option<VersionInfo> {
    if std::env::var(ENV_SKIP_VERSION_CHECK).is_ok() || std::env::var(ENV_OFFLINE).is_ok() {
        return None;
    }

    let url = get_update_url();
    let client = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .build()
        .ok()?;

    let resp = client.get(&url).send().await.ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;

    let version = data.get("version")?.as_str()?.to_string();
    let package_name = data
        .get("packageName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let note = data
        .get("note")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if is_newer(&version, current) {
        Some(VersionInfo {
            version,
            package_name,
            note,
        })
    } else {
        None
    }
}

fn is_newer(candidate: &str, current: &str) -> bool {
    fn parse(s: &str) -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    }
    let c = parse(candidate);
    let r = parse(current);
    c > r
}
