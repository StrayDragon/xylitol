//! Provider HTTP hook helpers — headers/body modify and after-response observe.

use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

use super::{DispatchResult, HookDispatcher, HookEvent, HookPhase};
use crate::domain::error::XyError;

/// Serialize request/response headers to a JSON object (lowercase keys).
pub fn headers_to_json(headers: &HeaderMap) -> Value {
    let mut map = serde_json::Map::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        let val = value.to_str().unwrap_or("").to_string();
        map.insert(key, Value::String(val));
    }
    Value::Object(map)
}

/// Merge header fields from a JSON object into a [`HeaderMap`].
///
/// `Modify.args.headers` from `before_provider_headers` uses this helper.
pub fn merge_headers_from_json(headers: &mut HeaderMap, value: &Value) {
    let Some(obj) = value.as_object() else {
        return;
    };
    for (key, val) in obj {
        let Some(text) = val.as_str() else {
            continue;
        };
        if let (Ok(name), Ok(header_val)) = (
            HeaderName::from_bytes(key.as_bytes()),
            HeaderValue::from_str(text),
        ) {
            headers.insert(name, header_val);
        }
    }
}

/// Run `before_provider_headers` hooks; merge modified headers when returned.
pub async fn run_before_headers(
    hooks: &Option<Arc<HookDispatcher>>,
    headers: &mut HeaderMap,
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
    headers: &HeaderMap,
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

    #[test]
    fn headers_to_json_lowercases_keys() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let json = headers_to_json(&headers);
        assert_eq!(json["content-type"], "application/json");
    }

    #[test]
    fn merge_headers_from_json_adds_header() {
        let mut headers = HeaderMap::new();
        merge_headers_from_json(
            &mut headers,
            &serde_json::json!({"x-test": "1", "content-type": "text/plain"}),
        );
        assert_eq!(headers.get("x-test").unwrap(), "1");
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "text/plain");
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
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        run_after_response(&hooks, 200, &headers).await;
    }
}
