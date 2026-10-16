//! Gateway attribution headers (align with pi `provider-attribution`).
//!
//! OpenCode Zen (`opencode.ai`) expects:
//! - `x-opencode-session` — current xylitol session id
//! - `x-opencode-client` — product tag (`xylitol`)
//!
//! Existing header keys win (caller / hooks may override).

use serde_json::Value;

use crate::hooks::HeaderBag;
use crate::provider::obs_session::{ObsSessionContext, obs_session_context};

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
///
/// Idle path: reads the process / TLS slot. Generate MUST call
/// [`merge_opencode_attribution_from`] with the options snapshot.
pub fn merge_opencode_attribution(headers: &mut HeaderBag, request_or_base_url: &str) {
    merge_opencode_attribution_from(headers, request_or_base_url, &obs_session_context());
}

/// Like [`merge_opencode_attribution`] but uses an explicit snapshot (c2590).
pub fn merge_opencode_attribution_from(
    headers: &mut HeaderBag,
    request_or_base_url: &str,
    ctx: &ObsSessionContext,
) {
    if !is_opencode_host(request_or_base_url) {
        return;
    }
    let Some(session_id) = ctx.session_id.clone().filter(|s| !s.is_empty()) else {
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

/// Session identity this request will present on the LLM channel, if any.
///
/// Today: OpenCode `x-opencode-session` = xylitol session id. Other hosts: none.
/// Must not invent a value when nothing is sent.
pub fn presented_llm_gateway_session_id(
    request_or_base_url: &str,
    ctx: &ObsSessionContext,
) -> Option<String> {
    if !is_opencode_host(request_or_base_url) {
        return None;
    }
    ctx.session_id.clone().filter(|s| !s.is_empty())
}

/// Copy of `ctx` with [`ObsSessionContext::llm_gateway_session_id`] set from the wire fact.
pub fn obs_for_llm_request(
    ctx: &ObsSessionContext,
    request_or_base_url: &str,
) -> ObsSessionContext {
    let mut out = ctx.clone();
    out.llm_gateway_session_id = presented_llm_gateway_session_id(request_or_base_url, ctx);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::obs_session::{ObsSessionContext, ObsSessionScope, set_obs_session};

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
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
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
    }

    #[test]
    fn snapshot_wins_over_process_slot() {
        let _g = ObsSessionScope::enter(ObsSessionContext {
            session_id: Some("process-wrong".into()),
            session_name: None,
            ..Default::default()
        });
        let mut headers = HeaderBag::new();
        let snap = ObsSessionContext {
            session_id: Some("bookmark-a".into()),
            session_name: None,
            ..Default::default()
        };
        merge_opencode_attribution_from(&mut headers, "https://opencode.ai/zen/v1", &snap);
        assert_eq!(
            headers.get("x-opencode-session").and_then(|v| v.as_str()),
            Some("bookmark-a")
        );
        set_obs_session("hijacked", None);
        let mut headers2 = HeaderBag::new();
        merge_opencode_attribution_from(&mut headers2, "https://opencode.ai/zen/v1", &snap);
        assert_eq!(
            headers2.get("x-opencode-session").and_then(|v| v.as_str()),
            Some("bookmark-a")
        );
    }

    #[test]
    fn skips_non_opencode() {
        let _g = ObsSessionScope::enter(ObsSessionContext::default());
        set_obs_session("sid-2", None);
        let mut headers = HeaderBag::new();
        merge_opencode_attribution(&mut headers, "https://api.deepseek.com/v1");
        assert!(headers.is_empty());
    }

    #[test]
    fn presented_gateway_id_is_wire_fact_not_placeholder() {
        let ctx = ObsSessionContext {
            session_id: Some("sid-1".into()),
            ..Default::default()
        };
        assert_eq!(
            presented_llm_gateway_session_id("https://opencode.ai/zen/v1", &ctx).as_deref(),
            Some("sid-1")
        );
        assert_eq!(
            presented_llm_gateway_session_id("https://api.deepseek.com/v1", &ctx),
            None
        );
        let obs = obs_for_llm_request(&ctx, "https://api.openai.com/v1");
        assert!(obs.llm_gateway_session_id.is_none());
        let p = crate::provider::obs_session::langfuse_session_properties_from(&obs);
        assert!(
            !p.iter()
                .any(|(k, _)| k == "xylitol.session.llm_gateway_session_id")
        );
        let presented = obs_for_llm_request(&ctx, "https://opencode.ai/zen/v1");
        let p = crate::provider::obs_session::langfuse_session_properties_from(&presented);
        assert!(p.contains(&(
            "xylitol.session.llm_gateway_session_id".into(),
            "sid-1".into()
        )));
    }
}
