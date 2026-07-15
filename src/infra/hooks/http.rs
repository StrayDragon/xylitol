//! Provider HTTP hook helpers — headers/body modify and after-response observe.
//!
//! **Client-agnostic**: header bags are JSON objects (`serde_json::Map`), matching
//! script-hook payloads. Adapters convert to/from their HTTP stack (reqwest today,
//! vendor SDK / `http` types later) at the provider edge — do not import HTTP
//! clients here.

use std::sync::Arc;

use serde_json::{Map, Value};

use super::{DispatchResult, HookDispatcher, HookEvent, HookPhase};
use crate::domain::error::XyError;

/// Portable request/response header bag (lowercase keys preferred).
pub type HeaderBag = Map<String, Value>;

/// Serialize a header bag to a JSON object (identity for [`HeaderBag`]).
pub fn headers_to_json(headers: &HeaderBag) -> Value {
    Value::Object(headers.clone())
}

/// Merge header fields from a JSON object into a [`HeaderBag`].
///
/// `Modify.args.headers` from `before_provider_headers` uses this helper.
pub fn merge_headers_from_json(headers: &mut HeaderBag, value: &Value) {
    let Some(obj) = value.as_object() else {
        return;
    };
    for (key, val) in obj {
        let Some(text) = val.as_str() else {
            continue;
        };
        headers.insert(key.to_ascii_lowercase(), Value::String(text.to_string()));
    }
}

/// Run `before_provider_headers` hooks; merge modified headers when returned.
pub async fn run_before_headers(
    hooks: &Option<Arc<HookDispatcher>>,
    headers: &mut HeaderBag,
) -> Result<(), XyError> {
    let Some(dispatcher) = hooks else {
        return Ok(());
    };
    if dispatcher.is_empty() {
        return Ok(());
    }

    let event = HookEvent::BeforeProviderHeaders {
        headers: headers_to_json(headers),
    };
    match dispatcher.dispatch(&event, HookPhase::Pre).await {
        DispatchResult::Allowed => Ok(()),
        DispatchResult::Blocked { reason } => Err(XyError::Provider(anyhow::anyhow!(
            "hook blocked headers: {reason}"
        ))),
        DispatchResult::Modified { args } => {
            if let Some(hdrs) = args.get("headers") {
                merge_headers_from_json(headers, hdrs);
            } else {
                merge_headers_from_json(headers, &args);
            }
            Ok(())
        }
    }
}

/// Run `before_provider_request` hooks; replace body when modified.
pub async fn run_before_request(
    hooks: &Option<Arc<HookDispatcher>>,
    model: &str,
    body: &mut Value,
) -> Result<(), XyError> {
    let Some(dispatcher) = hooks else {
        return Ok(());
    };
    if dispatcher.is_empty() {
        return Ok(());
    }

    let event = HookEvent::BeforeProviderRequest {
        model: model.to_string(),
        body: body.clone(),
    };
    match dispatcher.dispatch(&event, HookPhase::Pre).await {
        DispatchResult::Allowed => Ok(()),
        DispatchResult::Blocked { reason } => Err(XyError::Provider(anyhow::anyhow!(
            "hook blocked request: {reason}"
        ))),
        DispatchResult::Modified { args } => {
            *body = args;
            Ok(())
        }
    }
}

/// Run `after_provider_response` hooks (observe-only; errors are ignored).
pub async fn run_after_response(
    hooks: &Option<Arc<HookDispatcher>>,
    status: u16,
    headers: &HeaderBag,
) {
    let Some(dispatcher) = hooks else {
        return;
    };
    if dispatcher.is_empty() {
        return;
    }

    let event = HookEvent::AfterProviderResponse {
        status,
        headers: headers_to_json(headers),
    };
    let _ = dispatcher.dispatch(&event, HookPhase::Post).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::config::types::{HookEntry, HooksConfig};

    fn bag(pairs: &[(&str, &str)]) -> HeaderBag {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::String((*v).to_string())))
            .collect()
    }

    #[test]
    fn headers_to_json_preserves_keys() {
        let headers = bag(&[("content-type", "application/json")]);
        let json = headers_to_json(&headers);
        assert_eq!(json["content-type"], "application/json");
    }

    #[test]
    fn merge_headers_from_json_adds_header() {
        let mut headers = HeaderBag::new();
        merge_headers_from_json(
            &mut headers,
            &serde_json::json!({"X-Test": "1", "content-type": "text/plain"}),
        );
        assert_eq!(headers.get("x-test").unwrap(), "1");
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");
    }

    #[tokio::test]
    async fn run_before_request_modify_replaces_body() {
        let config = HooksConfig {
            global: vec![HookEntry {
                events: vec!["before_provider_request".into()],
                command: "echo '{\"action\":\"modify\",\"args\":{\"cache_control\":{\"type\":\"ephemeral\"}}}'".into(),
                ..Default::default()
            }],
            project: vec![],
            user: vec![],
        };
        let dispatcher = Arc::new(HookDispatcher::new(&config));
        let hooks = Some(dispatcher);
        let mut body = serde_json::json!({"model": "deepseek", "input": []});
        run_before_request(&hooks, "deepseek", &mut body)
            .await
            .expect("modify should succeed");
        assert_eq!(body["cache_control"]["type"], "ephemeral");
    }

    #[tokio::test]
    async fn empty_hooks_before_request_is_noop() {
        let dispatcher = Arc::new(HookDispatcher::new(&HooksConfig::default()));
        let hooks = Some(dispatcher);
        let mut body = serde_json::json!({"model": "x"});
        run_before_request(&hooks, "x", &mut body).await.unwrap();
        assert_eq!(body["model"], "x");
    }

    #[tokio::test]
    async fn run_after_response_dispatches() {
        let config = HooksConfig {
            global: vec![HookEntry {
                events: vec!["after_provider_response".into()],
                command: "echo '{\"action\":\"allow\"}'".into(),
                ..Default::default()
            }],
            project: vec![],
            user: vec![],
        };
        let dispatcher = Arc::new(HookDispatcher::new(&config));
        let hooks = Some(dispatcher);
        let headers = bag(&[("content-type", "application/json")]);
        run_after_response(&hooks, 200, &headers).await;
    }
}
