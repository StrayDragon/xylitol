//! Version checking against a remote API.

#![allow(dead_code)]

use super::constants::{ENV_OFFLINE, ENV_SKIP_VERSION_CHECK, HTTP_USER_AGENT, VERSION_CHECK_URL};

/// Information about a new version.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub package_name: Option<String>,
    pub note: Option<String>,
}

/// Check for a newer version of the application.
pub async fn check_for_new_version(current: &str) -> Option<VersionInfo> {
    if std::env::var(ENV_SKIP_VERSION_CHECK).is_ok() || std::env::var(ENV_OFFLINE).is_ok() {
        return None;
    }

    let client = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .build()
        .ok()?;

    let resp = client.get(VERSION_CHECK_URL).send().await.ok()?;
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
