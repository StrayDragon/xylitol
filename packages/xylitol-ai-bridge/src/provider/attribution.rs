//! Gateway attribution headers (align with pi `provider-attribution`).
//!
//! OpenCode Zen (`opencode.ai`) expects:
//! - `x-opencode-session` — current xylitol session id
//! - `x-opencode-client` — product tag (`xylitol`)
//!
//! Existing header keys win (caller / hooks may override).

use serde_json::Value;

use crate::hooks::HeaderBag;
use crate::provider::obs_session::obs_session_context;

const OPENCODE_HOST: &str = "opencode.ai";
const OPENCODE_CLIENT: &str = "xylitol";

/// True when `url` (absolute or base) targets OpenCode (`opencode.ai`).
pub fn is_opencode_host(url: &str) -> bool {
    let candidate = url.trim();
    if candidate.is_empty() {
        return false;
    }
    let parsed = match reqwest::Url::parse(candidate) {
        Ok(u) => u,
        Err(_) => match reqwest::Url::parse(&format!("https://{candidate}")) {
            Ok(u) => u,
            Err(_) => return false,
        },
    };
    parsed
        .host_str()
        .is_some_and(|h| h.eq_ignore_ascii_case(OPENCODE_HOST))
}

/// Insert OpenCode session attribution when the request targets `opencode.ai`
/// and a session id is bound. Does not overwrite keys already present.
pub fn merge_opencode_attribution(headers: &mut HeaderBag, request_or_base_url: &str) {
    if !is_opencode_host(request_or_base_url) {
        return;
    }
    let Some(session_id) = obs_session_context().session_id else {
        return;
    };
    if !headers.contains_key("x-opencode-session") {
        headers.insert("x-opencode-session".into(), Value::String(session_id));
    }
    if !headers.contains_key("x-opencode-client") {
        headers.insert(
            "x-opencode-client".into(),
            Value::String(OPENCODE_CLIENT.into()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::obs_session::{clear_obs_session, set_obs_session};

    #[test]
    fn detects_opencode_host() {
        assert!(is_opencode_host("https://opencode.ai/zen/v1"));
        assert!(is_opencode_host(
            "https://opencode.ai/zen/v1/chat/completions"
        ));
        assert!(!is_opencode_host("https://api.deepseek.com"));
        assert!(!is_opencode_host(""));
    }

    #[test]
    fn merges_only_for_opencode_with_session() {
        clear_obs_session();
        let mut headers = HeaderBag::new();
        merge_opencode_attribution(&mut headers, "https://opencode.ai/zen/v1");
        assert!(headers.is_empty());

        set_obs_session("sid-1", None);
        merge_opencode_attribution(&mut headers, "https://opencode.ai/zen/v1");
        assert_eq!(
            headers.get("x-opencode-session").and_then(|v| v.as_str()),
            Some("sid-1")
        );
        assert_eq!(
            headers.get("x-opencode-client").and_then(|v| v.as_str()),
            Some("xylitol")
        );

        headers.insert("x-opencode-client".into(), Value::String("custom".into()));
        merge_opencode_attribution(&mut headers, "https://opencode.ai/zen/v1");
        assert_eq!(
            headers.get("x-opencode-client").and_then(|v| v.as_str()),
            Some("custom"),
            "existing keys must win"
        );
        clear_obs_session();
    }

    #[test]
    fn skips_non_opencode() {
        set_obs_session("sid-2", None);
        let mut headers = HeaderBag::new();
        merge_opencode_attribution(&mut headers, "https://api.deepseek.com/v1");
        assert!(headers.is_empty());
        clear_obs_session();
    }
}
