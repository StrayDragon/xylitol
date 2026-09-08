//! OpenAI Chat Completions adapter.
//!
//! HTTP via current transport (reqwest) + portable hook three-seam (c998).
//! Hooks use [`crate::hooks::HeaderBag`]; reqwest conversion stays
//! in private `reqwest_bridge`. Public `XyModel` surface is only
//! [`super::AdapterXyModel`] wrapping this [`crate::infra::provider::adapter::AiBridgeLlmAdapter`] (c505).

use std::sync::Arc;

use async_trait::async_trait;

use crate::dto::AiBridgeMessage;
use crate::dto::AiBridgeStream;
use crate::dto::AiBridgeToolSchema;
use crate::error::AiBridgeError;
use crate::hooks::HttpHooks;
use crate::provider::native::openai::OpenAIProvider;
use crate::wire_policy::WirePolicy;

use crate::provider::AiBridgeLlmAdapter;

/// Adapter for the OpenAI Chat Completions API.
pub struct OpenAiCompletionsAdapter {
    inner: OpenAIProvider,
}

impl OpenAiCompletionsAdapter {
    /// Create a new Chat Completions adapter with [`WirePolicy::default`].
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self::with_wire_policy(api_key, model, base_url, hooks, WirePolicy::default())
    }

    /// Create with an explicit wire / thinking compat profile.
    pub fn with_wire_policy(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
        wire_policy: WirePolicy,
    ) -> Self {
        Self {
            inner: OpenAIProvider::with_wire_policy(api_key, model, base_url, hooks, wire_policy),
        }
    }
}

#[async_trait]
impl AiBridgeLlmAdapter for OpenAiCompletionsAdapter {
    fn name(&self) -> &str {
        "openai-completions"
    }

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner
            .generate_stream(messages, tools, true, &options)
            .await
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.inner
            .generate_stream(messages, tools, false, &options)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::StreamExt;
    use serde_json::Value;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::hooks::{HeaderBag, HttpHooks};

    struct NoopHooks;

    #[async_trait]
    impl HttpHooks for NoopHooks {
        async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn before_request(
            &self,
            _model: &str,
            _body: &mut Value,
        ) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
    }

    struct ModifyBodyHooks;

    #[async_trait]
    impl HttpHooks for ModifyBodyHooks {
        async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
            Ok(())
        }

        async fn before_request(
            &self,
            _model: &str,
            body: &mut Value,
        ) -> Result<(), AiBridgeError> {
            body["cache_control"] = serde_json::json!({"type": "ephemeral"});
            Ok(())
        }

        async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
    }

    #[tokio::test]
    async fn empty_hooks_completions_noop() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "hi"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(NoopHooks)),
        );
        let mut stream = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], Default::default())
            .await
            .expect("generate");
        let mut saw_text = false;
        while let Some(item) = stream.next().await {
            let chunk = item.expect("chunk");
            if matches!(chunk, crate::dto::AiBridgeChunk::TextDelta(_)) {
                saw_text = true;
            }
        }
        assert!(saw_text);
    }

    #[tokio::test]
    async fn before_provider_request_modify_reaches_wiremock() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "cache_control": {"type": "ephemeral"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(ModifyBodyHooks)),
        );
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], Default::default())
            .await
            .expect("generate with modify");
    }

    #[tokio::test]
    async fn completions_body_includes_reasoning_effort_medium() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "reasoning_effort": "medium"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(NoopHooks)),
        );
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], opts)
            .await
            .expect("generate medium");
    }

    #[tokio::test]
    async fn completions_body_omits_effort_when_off() {
        use std::sync::{Arc as StdArc, Mutex};

        let captured: StdArc<Mutex<Option<Value>>> = StdArc::new(Mutex::new(None));
        let captured_hook = captured.clone();

        struct CaptureHooks {
            body: StdArc<Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
                Ok(())
            }

            async fn before_request(
                &self,
                _model: &str,
                body: &mut Value,
            ) -> Result<(), AiBridgeError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured_hook,
            })),
        );
        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions::default(),
            )
            .await
            .expect("generate off");

        let body = captured.lock().unwrap().clone().expect("captured body");
        assert!(
            body.get("reasoning_effort").is_none(),
            "off must omit reasoning_effort, got {body}"
        );
    }

    #[tokio::test]
    async fn completions_body_map_override_high_to_max() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(wiremock::matchers::body_partial_json(serde_json::json!({
                "reasoning_effort": "max"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(NoopHooks)),
        );
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "high".into(),
            level_map: map,
            thinking_budgets: None,
            system_prompt: None,
            obs_parent: None,
            obs_session: Default::default(),
        };
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], opts)
            .await
            .expect("generate map override");
    }

    /// After switching level Off → high, the next Completions body MUST carry
    /// `reasoning_effort: "high"` (c1165 regression lock for UI cycle → request).
    #[tokio::test]
    async fn completions_body_effort_follows_level_switch() {
        use std::sync::{Arc as StdArc, Mutex};

        let captured: StdArc<Mutex<Vec<Value>>> = StdArc::new(Mutex::new(Vec::new()));
        let captured_hook = captured.clone();

        struct CaptureHooks {
            bodies: StdArc<Mutex<Vec<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
                Ok(())
            }

            async fn before_request(
                &self,
                _model: &str,
                body: &mut Value,
            ) -> Result<(), AiBridgeError> {
                self.bodies.lock().unwrap().push(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                bodies: captured_hook,
            })),
        );

        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions {
                    thinking_level: "off".into(),
                    ..Default::default()
                },
            )
            .await
            .expect("off");
        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &[],
                crate::thinking::AiBridgeGenerateOptions {
                    thinking_level: "high".into(),
                    ..Default::default()
                },
            )
            .await
            .expect("high after switch");

        let bodies = captured.lock().unwrap().clone();
        assert_eq!(bodies.len(), 2, "expected two requests");
        assert!(
            bodies[0].get("reasoning_effort").is_none(),
            "off: {bodies:?}"
        );
        assert_eq!(
            bodies[1].get("reasoning_effort"),
            Some(&serde_json::json!("high")),
            "after switch to high: {bodies:?}"
        );
    }

    #[tokio::test]
    async fn completions_mcp_tool_names_are_provider_safe() {
        use std::sync::{Arc as StdArc, Mutex};

        let captured: StdArc<Mutex<Option<Value>>> = StdArc::new(Mutex::new(None));

        struct CaptureHooks {
            body: StdArc<Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
                Ok(())
            }

            async fn before_request(
                &self,
                _model: &str,
                body: &mut Value,
            ) -> Result<(), AiBridgeError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "x",
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": "ok"},
                    "finish_reason": "stop"
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&server)
            .await;

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let tools = [
            AiBridgeToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            AiBridgeToolSchema {
                name: "mcp__context7__resolve-library-id".into(),
                description: "docs".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            AiBridgeToolSchema {
                name: "mcp:legacy:colon".into(),
                description: "legacy".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let _ = adapter
            .generate(
                vec![AiBridgeMessage::user("hi")],
                &tools,
                Default::default(),
            )
            .await
            .expect("generate with tools");

        let body = captured.lock().unwrap().clone().expect("body");
        let arr = body["tools"].as_array().expect("tools");
        assert_eq!(arr.len(), 3);
        for (i, t) in arr.iter().enumerate() {
            let name = t
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(
                crate::provider::tool_wire::is_provider_safe_tool_name(name),
                "tools[{i}].function.name={name:?}"
            );
            assert!(!name.contains(':'), "colon on wire: {name}");
            assert!(!name.contains('.'), "dot on wire: {name}");
        }
        assert_eq!(
            arr[2].pointer("/function/name").and_then(|v| v.as_str()),
            Some("mcp__legacy__colon")
        );
    }

    fn completions_ok_json() -> serde_json::Value {
        serde_json::json!({
            "id": "x",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "ok"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })
    }

    #[tokio::test]
    async fn overlapping_generate_uses_options_snapshot_not_process_slot() {
        use std::time::Duration;

        use crate::provider::obs_session::{ObsSessionContext, ObsSessionScope, set_obs_session};
        use crate::provider::trace::{ObsGateScope, ObsGateState, SpanCollectScope};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(200))
                    .set_body_json(completions_ok_json()),
            )
            .expect(2)
            .mount(&server)
            .await;

        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let _sess = ObsSessionScope::enter(ObsSessionContext {
            session_id: Some("process-wrong".into()),
            session_name: None,
            ..Default::default()
        });
        let collect = SpanCollectScope::enter();
        set_obs_session("process-wrong", None);

        let adapter = OpenAiCompletionsAdapter::new(
            "sk-test".into(),
            "gpt-test".into(),
            Some(server.uri()),
            None,
        );

        let opts_a = crate::thinking::AiBridgeGenerateOptions {
            obs_session: ObsSessionContext {
                session_id: Some("bookmark-a".into()),
                session_name: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let opts_b = crate::thinking::AiBridgeGenerateOptions {
            obs_session: ObsSessionContext {
                session_id: Some("bookmark-b".into()),
                session_name: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let (ra, rb, _) = tokio::join!(
            adapter.generate(vec![AiBridgeMessage::user("a")], &[], opts_a),
            adapter.generate(vec![AiBridgeMessage::user("b")], &[], opts_b),
            async {
                tokio::time::sleep(Duration::from_millis(40)).await;
                set_obs_session("hijacked", None);
            }
        );
        drop(ra.expect("generate a"));
        drop(rb.expect("generate b"));

        fastrace::flush();
        let spans = collect.records();
        let llm: Vec<_> = spans.iter().filter(|s| s.name == "llm.request").collect();
        let ids: Vec<&str> = llm
            .iter()
            .filter_map(|s| {
                s.properties
                    .iter()
                    .find(|(k, _)| k.as_ref() == "langfuse.session.id")
                    .map(|(_, v)| v.as_ref())
            })
            .collect();
        assert!(
            ids.contains(&"bookmark-a"),
            "missing bookmark-a on llm.request: {ids:?} spans={spans:?}"
        );
        assert!(
            ids.contains(&"bookmark-b"),
            "missing bookmark-b on llm.request: {ids:?}"
        );
        assert!(
            !ids.iter()
                .any(|id| *id == "process-wrong" || *id == "hijacked"),
            "process slot leaked into llm.request: {ids:?}"
        );
        for s in &llm {
            let get = |k: &str| {
                s.properties
                    .iter()
                    .find(|(kk, _)| kk.as_ref() == k)
                    .map(|(_, v)| v.as_ref())
            };
            assert_eq!(
                get("langfuse.session.id"),
                get("xylitol.session.id"),
                "langfuse.session.id must equal xylitol.session.id"
            );
            assert!(
                get("xylitol.session.llm_gateway_session_id").is_none(),
                "mock OpenAI host did not present a channel session id; must not invent one"
            );
        }
    }
}
