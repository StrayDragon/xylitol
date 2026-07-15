//! OpenAI Responses API adapter.
//!
//! Supports both streaming (`stream: true`) and non-streaming calls to
//! `/v1/responses`, emitting reasoning items as [`AiBridgeChunk::ThinkingDelta`] and
//! final message text as [`AiBridgeChunk::TextDelta`].

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

/// Adapter for the OpenAI Responses API (`/v1/responses`).
pub struct OpenAiResponsesAdapter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    hooks: Option<Arc<dyn HttpHooks>>,
}

impl OpenAiResponsesAdapter {
    /// Create a new Responses API adapter.
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
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
            hooks,
        }
    }

    fn headers_bag(&self) -> HeaderBag {
        let mut headers = HeaderBag::new();
        headers.insert(
            "content-type".into(),
            Value::String("application/json".into()),
        );
        headers.insert(
            "authorization".into(),
            Value::String(format!("Bearer {}", self.api_key)),
        );
        headers
    }

    fn build_body(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
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

        body
    }

    async fn send_request(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
    ) -> Result<reqwest::Response, AiBridgeError> {
        let mut body = self.build_body(messages, tools, stream);
        let url = format!("{}/responses", self.base_url);

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
                AiBridgeError::Provider(anyhow::anyhow!("OpenAI Responses request: {e}"))
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

        Ok(response)
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
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace =
            crate::provider::trace::ProviderRequestTrace::start("openai-responses", &self.model);
        let response = self.send_request(messages, tools, true).await?;
        Ok(Box::pin(responses_stream(response, trace)))
    }

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace =
            crate::provider::trace::ProviderRequestTrace::start("openai-responses", &self.model);
        let response = self.send_request(messages, tools, false).await?;
        let json: Value = response
            .json()
            .await
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("parse response: {e}")))?;
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

fn responses_stream(
    response: reqwest::Response,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;
        use eventsource_stream::Eventsource;

        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();

        let mut reasoning_items: HashMap<String, bool> = HashMap::new();
        let mut function_call_args: HashMap<String, String> = HashMap::new();
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
                    .get("delta")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                t.emit_raw(event_type, snippet);
            }

            match event_type {
                "response.output_item.added" => {
                    if let Some(item) = data.get("item") {
                        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let item_id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if item_type == "reasoning" {
                            reasoning_items.insert(item_id, true);
                        }
                    }
                }

                "response.reasoning_text.delta" => {
                    if let Some(delta) = data.get("delta").and_then(|v| v.as_str()) {
                        let chunk = AiBridgeChunk::ThinkingDelta(delta.to_string());
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                    }
                }

                "response.output_text.delta" => {
                    if let Some(delta) = data.get("delta").and_then(|v| v.as_str()) {
                        let chunk = AiBridgeChunk::TextDelta(delta.to_string());
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                    }
                }

                "response.function_call_arguments.delta" => {
                    if let (Some(item_id), Some(delta)) = (
                        data.get("item_id").and_then(|v| v.as_str()),
                        data.get("delta").and_then(|v| v.as_str()),
                    ) {
                        function_call_args
                            .entry(item_id.to_string())
                            .or_default()
                            .push_str(delta);
                    }
                }

                "response.output_item.done" => {
                    if let Some(item) = data.get("item") {
                        let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        if item_type == "function_call" {
                            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let args_str = function_call_args.remove(&id).unwrap_or_default();
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or(serde_json::json!({}));
                            let chunk = AiBridgeChunk::FunctionCall { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }
                    }
                }

                "response.completed" => {
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
                        finish_reason: AiBridgeStopReason::Stop,
                        usage,
                    };
                    if let Some(t) = &trace {
                        t.emit_mapped_chunk(&chunk);
                    }
                    yield chunk;
                }

                "response.usage" => {
                    if let Some(usage) = data.get("usage") {
                        usage_input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(usage_input);
                        usage_output = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(usage_output);
                    }
                }

                _ => {}
            }
        }
    })
}

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
                    let args = item
                        .get("arguments")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));
                    chunks.push(AiBridgeChunk::FunctionCall { name, args, id });
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

fn extract_error_message(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?
        .get("message")?
        .as_str()
        .map(String::from)
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
            AiBridgeMessage::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                ..
            } => {
                if *exclude_from_context {
                    continue;
                }
                let text = format!("$ {command}\n{output}");
                items.push(serde_json::json!({
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": text}],
                }));
            }
            AiBridgeMessage::CompactionSummaryMessage { summary, .. }
            | AiBridgeMessage::BranchSummaryMessage { summary, .. } => {
                items.push(serde_json::json!({
                    "type": "message",
                    "role": "user",
                    "content": [{
                        "type": "input_text",
                        "text": format!("[Context summary: {summary}]"),
                    }],
                }));
            }
            AiBridgeMessage::CustomMessage {
                custom_type: _,
                content,
                ..
            } => {
                let text = content.as_str().unwrap_or("").to_string();
                if text.is_empty() {
                    continue;
                }
                items.push(serde_json::json!({
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": text}],
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
}
