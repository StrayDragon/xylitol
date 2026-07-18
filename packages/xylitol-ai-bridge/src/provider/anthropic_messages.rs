use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::dto::AiBridgeStream;
use crate::dto::{AiBridgeChunk, AiBridgeToolSchema};
use crate::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason};
use crate::error::AiBridgeError;
use crate::hooks::{
    HeaderBag, HttpHooks, run_after_response, run_before_headers, run_before_request,
};
use crate::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};

use super::AiBridgeLlmAdapter;

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Messages API adapter.
pub struct AnthropicMessagesAdapter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    hooks: Option<Arc<dyn HttpHooks>>,
}

impl AnthropicMessagesAdapter {
    /// Create a new Anthropic Messages adapter.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".into()),
            max_tokens: 8192,
            hooks,
        }
    }

    fn headers_bag(&self) -> HeaderBag {
        let mut headers = HeaderBag::new();
        headers.insert(
            "content-type".into(),
            Value::String("application/json".into()),
        );
        headers.insert("x-api-key".into(), Value::String(self.api_key.clone()));
        headers.insert(
            "anthropic-version".into(),
            Value::String(ANTHROPIC_VERSION.into()),
        );
        headers
    }
}

#[async_trait]
impl AiBridgeLlmAdapter for AnthropicMessagesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.execute(messages, tools, true, options).await
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        self.execute(messages, tools, false, options).await
    }
}

impl AnthropicMessagesAdapter {
    async fn execute(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let (system_prompt, anthropic_msgs) = convert_agent_messages_for_anthropic(&messages);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": anthropic_msgs,
            "max_tokens": self.max_tokens,
            "stream": stream,
        });

        if let Some(system) = system_prompt {
            body["system"] = Value::String(system);
        }

        if !tools.is_empty() {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        let resolved = crate::thinking::resolve_from_options(
            &options,
            crate::thinking::AiBridgeThinkingAdapterKind::Anthropic,
        );
        crate::thinking::apply_thinking_anthropic(&mut body, &resolved);

        let url = format!("{}/v1/messages", self.base_url);

        let mut headers = self.headers_bag();
        run_before_headers(&self.hooks, &mut headers).await?;
        run_before_request(&self.hooks, &self.model, &mut body).await?;

        let response = self
            .client
            .post(&url)
            .headers(to_reqwest_headers(&headers))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("Anthropic request error: {e}"))
            })?;

        let status = response.status().as_u16();
        run_after_response(
            &self.hooks,
            status,
            &from_reqwest_headers(response.headers()),
        )
        .await;

        if !response.status().is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let msg = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {status}: {body_text}"));
            return Err(AiBridgeError::Provider(anyhow::anyhow!(msg)));
        }

        if stream {
            let trace = crate::provider::trace::ProviderRequestTrace::start(
                "anthropic-messages",
                &self.model,
            );
            Ok(Box::pin(anthropic_stream(response, trace)))
        } else {
            let trace = crate::provider::trace::ProviderRequestTrace::start(
                "anthropic-messages",
                &self.model,
            );
            let json: Value = response
                .json()
                .await
                .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("parse response: {e}")))?;
            if let Some(t) = &trace {
                t.emit_raw("message.json", &json.to_string());
            }
            let chunks = parse_anthropic_response(&json);
            if let Some(t) = &trace {
                for c in &chunks {
                    t.emit_mapped_chunk(c);
                }
            }
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }
}

fn anthropic_stream(
    response: reqwest::Response,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;
        use eventsource_stream::Eventsource;

        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();

        let mut tool_accumulators: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut current_block_index: Option<usize> = None;
        let mut usage_input: u64 = 0;
        let mut usage_output: u64 = 0;

        while let Some(event_result) = event_stream.next().await {
            let event = match event_result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let event_type = event.event.as_str();
            let data: Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(t) = &trace {
                let snippet = data
                    .pointer("/delta/text")
                    .or_else(|| data.pointer("/delta/thinking"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                t.emit_raw(event_type, snippet);
            }

            match event_type {
                "content_block_start" => {
                    let index = data.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                    current_block_index = Some(index);

                    if let Some(content_block) = data.get("content_block") {
                        let block_type = content_block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if block_type == "tool_use" {
                            let id = content_block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = content_block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            tool_accumulators.insert(index, (id.clone(), name.clone(), String::new()));
                            let chunk = AiBridgeChunk::ToolCallStart { id, name };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }
                    }
                }

                "content_block_delta" => {
                    let index = data.get("index").and_then(|v| v.as_u64()).unwrap_or(
                        current_block_index.unwrap_or(0) as u64
                    ) as usize;

                    if let Some(delta) = data.get("delta") {
                        let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");

                        match delta_type {
                            "text_delta" => {
                                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                    let chunk = AiBridgeChunk::TextDelta(text.to_string());
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            "thinking" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    let chunk = AiBridgeChunk::ThinkingDelta(thinking.to_string());
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            "input_json_delta" => {
                                if let (Some(partial_json), Some(acc)) = (
                                    delta.get("partial_json").and_then(|v| v.as_str()),
                                    tool_accumulators.get_mut(&index),
                                ) {
                                    acc.2.push_str(partial_json);
                                    let chunk = AiBridgeChunk::ToolCallDelta {
                                        id: acc.0.clone(),
                                        name: acc.1.clone(),
                                        args_delta: partial_json.to_string(),
                                        args: crate::dto::parse_streaming_json(&acc.2),
                                    };
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                "content_block_stop" => {
                    if let Some(index) = current_block_index
                        && let Some((id, name, args_str)) = tool_accumulators.remove(&index)
                    {
                        let args: Value = serde_json::from_str(&args_str)
                            .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
                        let chunk = AiBridgeChunk::ToolCallEnd { name, args, id };
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                    }
                    current_block_index = None;
                }

                "message_delta" => {
                    let stop_reason = data
                        .get("delta")
                        .and_then(|d| d.get("stop_reason"))
                        .and_then(|v| v.as_str());

                    // Capture output tokens from message_delta
                    if let Some(u) = data.get("usage").and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64()) {
                        usage_output = u;
                    }

                    if stop_reason.is_some() {
                        // Finalize any tool blocks that missed content_block_stop.
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
                            let chunk = AiBridgeChunk::ToolCallEnd { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }

                        let finish = match stop_reason {
                            Some("max_tokens") => AiBridgeStopReason::MaxTokens,
                            _ => AiBridgeStopReason::Stop,
                        };

                        let usage_total = usage_input + usage_output;
                        let usage = if usage_total > 0 {
                            Some(crate::dto::AiBridgeUsage {
                                input: usage_input,
                                output: usage_output,
                                cache_read: 0,
                                cache_write: 0,
                                total_tokens: usage_total,
                                cache_write_1h: 0,
                                cost: None,
                            })
                        } else {
                            None
                        };
                        let chunk = AiBridgeChunk::Done {
                            finish_reason: finish,
                            usage,
                        };
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                    }
                }

                "message_start" => {
                    // Capture input tokens from message_start
                    if let Some(msg) = data.get("message")
                        && let Some(u) = msg.get("usage").and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64()) {
                            usage_input = u;
                        }
                }
                "message_stop" | "ping" => {}
                _ => {}
            }
        }
    })
}

fn parse_anthropic_response(json: &Value) -> Vec<AiBridgeChunk> {
    let mut chunks = Vec::new();

    if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
        for block in content {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        chunks.push(AiBridgeChunk::TextDelta(text.to_string()));
                    }
                }
                "thinking" => {
                    if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                        chunks.push(AiBridgeChunk::ThinkingDelta(thinking.to_string()));
                    }
                }
                "tool_use" => {
                    let id = block
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = block
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args = block.get("input").cloned().unwrap_or(serde_json::json!({}));
                    chunks.push(AiBridgeChunk::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    chunks.push(AiBridgeChunk::ToolCallEnd { name, args, id });
                }
                _ => {}
            }
        }
    }

    let finish = match json.get("stop_reason").and_then(|v| v.as_str()) {
        Some("max_tokens") => AiBridgeStopReason::MaxTokens,
        _ => AiBridgeStopReason::Stop,
    };

    // Parse usage from non-streaming response
    let usage = json.get("usage").and_then(|u| {
        let input = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let total = input + output;
        if total > 0 {
            Some(crate::dto::AiBridgeUsage {
                input,
                output,
                cache_read: 0,
                cache_write: 0,
                total_tokens: total,
                cache_write_1h: 0,
                cost: None,
            })
        } else {
            None
        }
    });

    chunks.push(AiBridgeChunk::Done {
        finish_reason: finish,
        usage,
    });

    chunks
}

// ── AiBridgeMessage conversion ────────────────────────────────────

/// Convert a slice of [`AiBridgeMessage`] values to Anthropic request body
/// (returns `(system_prompt, messages)` tuple).
pub fn convert_agent_messages_for_anthropic(
    messages: &[AiBridgeMessage],
) -> (Option<String>, Vec<Value>) {
    let system_parts: Vec<String> = Vec::new();
    let mut msgs: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            AiBridgeMessage::UserMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": blocks,
                    }));
                }
            }
            AiBridgeMessage::AssistantMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "assistant",
                        "content": blocks,
                    }));
                }
            }
            AiBridgeMessage::ToolResultMessage {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                let inner: Vec<Value> = content
                    .iter()
                    .map(|p| match p {
                        AiBridgePart::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        AiBridgePart::Image(img) => serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": img.media_type,
                                "data": img.data.as_deref().unwrap_or(""),
                            },
                        }),
                        _ => serde_json::json!({"type": "text", "text": ""}),
                    })
                    .collect();

                msgs.push(serde_json::json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": inner,
                        "is_error": *is_error,
                    }],
                }));
            }
        }
    }

    merge_consecutive_messages(&mut msgs);

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n"))
    };

    (system, msgs)
}

fn agent_parts_to_anthropic_blocks(parts: &[AiBridgePart]) -> Vec<Value> {
    parts
        .iter()
        .map(|part| match part {
            AiBridgePart::Text { text } => serde_json::json!({
                "type": "text",
                "text": text,
            }),
            AiBridgePart::Thinking { thinking, .. } => serde_json::json!({
                "type": "text",
                "text": thinking,
            }),
            AiBridgePart::Image(img) => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": img.media_type,
                    "data": img.data.as_deref().unwrap_or(""),
                },
            }),
            AiBridgePart::ToolCall {
                id,
                name,
                arguments,
            } => serde_json::json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": arguments,
            }),
        })
        .collect()
}

fn merge_consecutive_messages(messages: &mut Vec<Value>) {
    let mut i = 0;
    while i + 1 < messages.len() {
        let current_role = messages[i]
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let next_role = messages[i + 1]
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if current_role == next_role {
            let next = messages.remove(i + 1);
            if let (Some(current_content), Some(next_content)) = (
                messages[i]
                    .get_mut("content")
                    .and_then(|v| v.as_array_mut()),
                next.get("content").and_then(|v| v.as_array()),
            ) {
                current_content.extend(next_content.iter().cloned());
            }
        } else {
            i += 1;
        }
    }
}

fn extract_error_message(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?
        .get("message")?
        .as_str()
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AiBridgeMessage, AiBridgePart};

    #[test]
    fn convert_user_message() {
        let msgs = vec![AiBridgeMessage::user("hello")];
        let (system, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert!(system.is_none());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn convert_assistant_with_tool_call() {
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::text("Let me check"),
                AiBridgePart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "/tmp"}),
                },
            ],
            stop_reason: Some(crate::dto::AiBridgeStopReason::ToolUse),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["id"], "call-1");
    }

    #[test]
    fn convert_tool_result() {
        let msgs = vec![AiBridgeMessage::tool_result(
            "call-1",
            "",
            vec![AiBridgePart::text("result here")],
            false,
        )];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "call-1");
    }

    #[test]
    fn convert_projected_env_as_user() {
        let msgs = vec![AiBridgeMessage::user("$ ls\noutput")];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[tokio::test]
    async fn anthropic_body_omits_thinking_when_off() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
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

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
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

        let body = captured.lock().unwrap().clone().expect("body");
        assert!(
            body.get("thinking").is_none(),
            "off must omit thinking block, got {body}"
        );
    }

    #[tokio::test]
    async fn anthropic_body_includes_budget_for_medium() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
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

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let _ = adapter
            .generate(vec![AiBridgeMessage::user("hi")], &[], opts)
            .await
            .expect("generate medium");

        let body = captured.lock().unwrap().clone().expect("body");
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], 8192);
    }
}
