//! OpenAI Responses API adapter via `async-openai` [`Client`] (c1070).
//!
//! Streaming and non-streaming `/v1/responses` calls; reasoning text →
//! [`AiBridgeChunk::ThinkingDelta`], output text → [`AiBridgeChunk::TextDelta`].

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::dto::AiBridgeStream;
use crate::dto::{AiBridgeChunk, AiBridgeToolSchema};
use crate::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason};
use crate::error::AiBridgeError;
use crate::hooks::HttpHooks;
use crate::provider::openai_client::{build_openai_client, normalize_openai_v1_base};

use super::AiBridgeLlmAdapter;

/// Adapter for the OpenAI Responses API (`/v1/responses`).
pub struct OpenAiResponsesAdapter {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAiResponsesAdapter {
    /// Create a new Responses API adapter.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        let base = base_url.map(|b| normalize_openai_v1_base(&b));
        Self {
            client: build_openai_client(api_key, base, hooks),
            model,
        }
    }

    fn build_body(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: &crate::thinking::AiBridgeGenerateOptions,
    ) -> Value {
        let input_items = convert_messages_to_input_items(&messages);
        let mut body = serde_json::json!({
            "model": self.model,
            "input": input_items,
            "stream": stream,
        });

        if !tools.is_empty() {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        let resolved = crate::thinking::resolve_from_options(
            options,
            crate::thinking::AiBridgeThinkingAdapterKind::OpenAi,
        );
        crate::thinking::apply_thinking_openai_responses(&mut body, &resolved);

        body
    }

    fn map_err(err: async_openai::error::OpenAIError) -> AiBridgeError {
        AiBridgeError::Provider(anyhow::anyhow!("OpenAI Responses: {err}"))
    }
}

#[async_trait]
impl AiBridgeLlmAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace =
            crate::provider::trace::ProviderRequestTrace::start("openai-responses", &self.model);
        let body = self.build_body(messages, tools, true, &options);
        // BYOT + `Value`: compatible servers (e.g. llama.cpp) may omit fields that
        // typed `ResponseStreamEvent` requires (`created_at` on `response.created`).
        let sdk_stream = self
            .client
            .responses()
            .create_stream_byot::<_, Value>(body)
            .await
            .map_err(Self::map_err)?;
        Ok(Box::pin(responses_sdk_stream(sdk_stream, trace)))
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace =
            crate::provider::trace::ProviderRequestTrace::start("openai-responses", &self.model);
        let body = self.build_body(messages, tools, false, &options);
        let json: Value = self
            .client
            .responses()
            .create_byot(body)
            .await
            .map_err(Self::map_err)?;
        if let Some(t) = &trace {
            t.emit_raw("response.json", &json.to_string());
        }
        let chunks = parse_responses_output(&json);
        if let Some(t) = &trace {
            for c in &chunks {
                t.emit_mapped_chunk(c);
            }
        }
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

fn responses_sdk_stream(
    mut sdk_stream: impl Stream<Item = Result<Value, async_openai::error::OpenAIError>>
    + Send
    + Unpin
    + 'static,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        let mut state = ResponsesStreamState::default();

        while let Some(item) = sdk_stream.next().await {
            let data = item.map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("OpenAI Responses stream: {e}"))
            })?;

            let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(t) = &trace {
                let snippet = data.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                t.emit_raw(event_type, snippet);
            }

            for chunk in map_responses_sse_event(&data, &mut state) {
                if let Some(t) = &trace {
                    t.emit_mapped_chunk(&chunk);
                }
                yield chunk;
            }
        }
    })
}

/// Mutable state for Responses SSE → chunk mapping (streaming tool calls).
#[derive(Default)]
pub struct ResponsesStreamState {
    /// item_id → (name, partial args json, started)
    function_calls: HashMap<String, (String, String, bool)>,
    usage_input: u64,
    usage_output: u64,
}

/// Map one Responses SSE JSON payload (lenient `Value`) into zero or more chunks.
///
/// Compatible servers may emit partial objects (e.g. `response.created` without
/// `created_at`); those events are ignored rather than failing deserialization.
///
/// Public for BDD / harness (c1250 pab13).
pub fn map_responses_sse_event(
    data: &Value,
    state: &mut ResponsesStreamState,
) -> Vec<AiBridgeChunk> {
    let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match event_type {
        "response.reasoning_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![AiBridgeChunk::ThinkingDelta(d.to_string())])
            .unwrap_or_default(),
        "response.output_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![AiBridgeChunk::TextDelta(d.to_string())])
            .unwrap_or_default(),
        "response.output_item.added" => {
            let Some(item) = data.get("item") else {
                return Vec::new();
            };
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if item_type != "function_call" {
                return Vec::new();
            }
            let id = item
                .get("id")
                .or_else(|| item.get("call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                return Vec::new();
            }
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            state
                .function_calls
                .insert(id.clone(), (name.clone(), String::new(), true));
            vec![AiBridgeChunk::ToolCallStart { id, name }]
        }
        "response.function_call_arguments.delta" => {
            let (Some(item_id), Some(delta)) = (
                data.get("item_id").and_then(|v| v.as_str()),
                data.get("delta").and_then(|v| v.as_str()),
            ) else {
                return Vec::new();
            };
            let entry = state
                .function_calls
                .entry(item_id.to_string())
                .or_insert_with(|| (String::new(), String::new(), false));
            let mut out = Vec::new();
            if !entry.2 {
                entry.2 = true;
                out.push(AiBridgeChunk::ToolCallStart {
                    id: item_id.to_string(),
                    name: entry.0.clone(),
                });
            }
            entry.1.push_str(delta);
            let args = crate::dto::parse_streaming_json(&entry.1);
            out.push(AiBridgeChunk::ToolCallDelta {
                id: item_id.to_string(),
                name: entry.0.clone(),
                args_delta: delta.to_string(),
                args,
            });
            out
        }
        "response.output_item.done" => {
            let Some(item) = data.get("item") else {
                return Vec::new();
            };
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if item_type != "function_call" {
                return Vec::new();
            }
            let id = item
                .get("id")
                .or_else(|| item.get("call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name_from_item = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (name, args_str, started) = state
                .function_calls
                .remove(&id)
                .unwrap_or_else(|| (name_from_item.clone(), String::new(), false));
            let name = if name.is_empty() {
                name_from_item
            } else {
                name
            };
            let args_str = if args_str.is_empty() {
                item.get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string()
            } else {
                args_str
            };
            let args: Value = serde_json::from_str(&args_str)
                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
            let mut out = Vec::new();
            if !started {
                out.push(AiBridgeChunk::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                });
            }
            out.push(AiBridgeChunk::ToolCallEnd { id, name, args });
            out
        }
        "response.usage" => {
            if let Some(usage) = data.get("usage") {
                state.usage_input = usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_input);
                state.usage_output = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_output);
            }
            Vec::new()
        }
        "response.completed" => {
            if let Some(usage) = data
                .get("response")
                .and_then(|r| r.get("usage"))
                .or_else(|| data.get("usage"))
            {
                state.usage_input = usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_input);
                state.usage_output = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_output);
            }
            let usage_total = state.usage_input + state.usage_output;
            let usage = if usage_total > 0 {
                Some(crate::dto::AiBridgeUsage {
                    input: state.usage_input,
                    output: state.usage_output,
                    cache_read: 0,
                    cache_write: 0,
                    total_tokens: usage_total,
                    cache_write_1h: 0,
                    cost: None,
                })
            } else {
                None
            };
            vec![AiBridgeChunk::Done {
                finish_reason: AiBridgeStopReason::Stop,
                usage,
            }]
        }
        // Partial lifecycle events (`response.created`, `response.in_progress`, …)
        // must not fail the stream on compatible APIs.
        _ => Vec::new(),
    }
}

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI Responses `input` items.
fn parse_responses_output(json: &Value) -> Vec<AiBridgeChunk> {
    let mut chunks = Vec::new();

    if let Some(output) = json.get("output").and_then(|v| v.as_array()) {
        for item in output {
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match item_type {
                "reasoning" => {
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                chunks.push(AiBridgeChunk::ThinkingDelta(text.to_string()));
                            }
                        }
                    }
                }
                "message" => {
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                chunks.push(AiBridgeChunk::TextDelta(text.to_string()));
                            }
                        }
                    }
                }
                "function_call" => {
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args = match item.get("arguments") {
                        Some(Value::String(s)) => crate::dto::parse_streaming_json(s),
                        Some(v) => v.clone(),
                        None => serde_json::json!({}),
                    };
                    chunks.push(AiBridgeChunk::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    chunks.push(AiBridgeChunk::ToolCallEnd { id, name, args });
                }
                _ => {}
            }
        }
    }

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
        finish_reason: AiBridgeStopReason::Stop,
        usage,
    });

    chunks
}

// ── AiBridgeMessage conversion ────────────────────────────────────

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI Responses `input` items.
pub fn messages_to_responses_input(messages: &[AiBridgeMessage]) -> Vec<Value> {
    convert_messages_to_input_items(messages)
}

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI Responses `input` items.
fn convert_messages_to_input_items(messages: &[AiBridgeMessage]) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            AiBridgeMessage::UserMessage { content, .. } => {
                let text = collect_text_parts(content);
                if !text.is_empty() {
                    // Responses API requires each input item to declare its
                    // `type`; a bare {role, content} object yields
                    // "Cannot determine type of 'item'" once non-message items
                    // (function_call / function_call_output) are mixed in.
                    items.push(serde_json::json!({
                        "type": "message",
                        "role": "user",
                        "content": [{"type": "input_text", "text": text}],
                    }));
                }
            }
            AiBridgeMessage::AssistantMessage { content, .. } => {
                let text = collect_text_parts(content);
                let tool_calls: Vec<Value> = content
                    .iter()
                    .filter_map(|p| match p {
                        AiBridgePart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => Some(serde_json::json!({
                            "type": "function_call",
                            "id": id,
                            "call_id": id,
                            "name": name,
                            "arguments": arguments.to_string(),
                        })),
                        _ => None,
                    })
                    .collect();

                if !text.is_empty() || !tool_calls.is_empty() {
                    // Only emit a message item when there is assistant text;
                    // a tool-call round may have no text (pure function_call).
                    if !text.is_empty() {
                        items.push(serde_json::json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": text}],
                        }));
                    }
                    items.extend(tool_calls);
                }
            }
            AiBridgeMessage::ToolResultMessage {
                tool_use_id,
                content,
                ..
            } => {
                let text = collect_text_parts(content);
                items.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": tool_use_id,
                    "output": text,
                }));
            }
        }
    }

    items
}

fn collect_text_parts(parts: &[AiBridgePart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let Some(text) = part.as_text() {
            buf.push_str(text);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AiBridgeMessage, AiBridgePart};

    /// Every Responses API input item MUST declare a `type`, or OpenAI rejects
    /// the request with "Cannot determine type of 'item'" once non-message
    /// items (function_call / function_call_output) are mixed in.
    #[test]
    fn all_items_have_type_field() {
        let msgs = vec![
            AiBridgeMessage::user("hello"),
            AiBridgeMessage::assistant("hi there"),
        ];
        let items = convert_messages_to_input_items(&msgs);
        for item in &items {
            assert!(item.get("type").is_some(), "item missing `type`: {item}");
        }
    }

    /// Regression: a tool-calling continuation round (user → assistant with a
    /// tool_call → tool result) must produce well-typed items: message,
    /// function_call, function_call_output. This is exactly the sequence that
    /// failed before c375 (no continuation) and then hit the Responses API
    /// "Cannot determine type of 'item'" error after c375 enabled it.
    #[test]
    fn tool_round_items_are_well_typed() {
        let msgs = vec![
            AiBridgeMessage::user("list files"),
            AiBridgeMessage::AssistantMessage {
                content: vec![
                    AiBridgePart::text("let me check"),
                    AiBridgePart::ToolCall {
                        id: "call-1".into(),
                        name: "ls".into(),
                        arguments: serde_json::json!({"path": "."}),
                    },
                ],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
            AiBridgeMessage::tool_result(
                "call-1",
                "ls",
                vec![AiBridgePart::text("file.txt")],
                false,
            ),
        ];
        let items = convert_messages_to_input_items(&msgs);
        // user message, assistant message, function_call, function_call_output
        let types: Vec<&str> = items
            .iter()
            .map(|i| i.get("type").and_then(|v| v.as_str()).unwrap_or("(none)"))
            .collect();
        assert_eq!(
            types,
            vec![
                "message",
                "message",
                "function_call",
                "function_call_output"
            ],
            "item types in order: {types:?}"
        );
        // function_call_output must carry the call_id matching the call.
        let output = items
            .iter()
            .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("function_call_output"))
            .unwrap();
        assert_eq!(output["call_id"], "call-1");
        assert_eq!(output["output"], "file.txt");
    }

    #[test]
    fn assistant_tool_only_round_emits_function_call_not_empty_message() {
        // Assistant round with a tool_call and NO text must not emit an empty
        // message item (would confuse the API); only the function_call item.
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![AiBridgePart::ToolCall {
                id: "c1".into(),
                name: "ls".into(),
                arguments: serde_json::json!({}),
            }],
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let items = convert_messages_to_input_items(&msgs);
        assert_eq!(
            items.len(),
            1,
            "only the function_call item, no empty message"
        );
        assert_eq!(items[0]["type"], "function_call");
    }

    #[test]
    fn partial_response_created_is_ignored_not_fatal() {
        // llama.cpp (and other compatible servers) may omit `created_at` on
        // `response.created`; typed ResponseStreamEvent would fail here.
        let data: Value = serde_json::from_str(
            r#"{"type":"response.created","response":{"id":"resp_1","object":"response","status":"in_progress"}}"#,
        )
        .unwrap();
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert!(chunks.is_empty());
    }

    #[test]
    fn output_text_delta_maps_to_text_chunk() {
        let data = serde_json::json!({
            "type": "response.output_text.delta",
            "delta": "hello"
        });
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(&chunks[0], AiBridgeChunk::TextDelta(t) if t == "hello"));
    }

    #[test]
    fn completed_with_nested_usage_emits_done() {
        let data = serde_json::json!({
            "type": "response.completed",
            "response": {
                "usage": { "input_tokens": 3, "output_tokens": 5 }
            }
        });
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert_eq!(chunks.len(), 1);
        match &chunks[0] {
            AiBridgeChunk::Done { usage: Some(u), .. } => {
                assert_eq!(u.input, 3);
                assert_eq!(u.output, 5);
                assert_eq!(u.total_tokens, 8);
            }
            other => panic!("expected Done with usage, got {other:?}"),
        }
    }

    #[test]
    fn function_call_args_stream_before_done() {
        // t0718-shaped: item.added → many args deltas → item.done
        let mut state = ResponsesStreamState::default();
        let added = serde_json::json!({
            "type": "response.output_item.added",
            "item": { "type": "function_call", "id": "fc_1", "name": "ls", "arguments": "" }
        });
        let start = map_responses_sse_event(&added, &mut state);
        assert!(matches!(
            &start[..],
            [AiBridgeChunk::ToolCallStart { id, name }] if id == "fc_1" && name == "ls"
        ));

        let d1 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "{\"path\":"
        });
        let mid = map_responses_sse_event(&d1, &mut state);
        assert!(
            matches!(&mid[..], [AiBridgeChunk::ToolCallDelta { id, args_delta, .. }] if id == "fc_1" && args_delta == "{\"path\":"),
            "got {mid:?}"
        );

        let d2 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "\"/tmp\"}"
        });
        let mid2 = map_responses_sse_event(&d2, &mut state);
        assert!(matches!(&mid2[..], [AiBridgeChunk::ToolCallDelta { .. }]));

        let done = serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "function_call",
                "id": "fc_1",
                "name": "ls",
                "arguments": "{\"path\":\"/tmp\"}"
            }
        });
        let end = map_responses_sse_event(&done, &mut state);
        match &end[..] {
            [AiBridgeChunk::ToolCallEnd { id, name, args }] => {
                assert_eq!(id, "fc_1");
                assert_eq!(name, "ls");
                assert_eq!(args["path"], "/tmp");
            }
            other => panic!("expected ToolCallEnd only, got {other:?}"),
        }
    }

    #[test]
    fn build_body_injects_reasoning_effort() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let body = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &opts);
        assert_eq!(body["reasoning"]["effort"], "medium");

        let off = crate::thinking::AiBridgeGenerateOptions::default();
        let body_off = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &off);
        assert!(body_off.get("reasoning").is_none());

        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let mapped = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "high".into(),
            level_map: map,
            thinking_budgets: None,
        };
        let body_map = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &mapped);
        assert_eq!(body_map["reasoning"]["effort"], "max");
    }
}
