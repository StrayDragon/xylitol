//! OpenAI Responses API adapter.
//!
//! Supports both streaming (`stream: true`) and non-streaming calls to
//! `/v1/responses`, emitting reasoning items as [`XyChunk::ThinkingDelta`] and
//! final message text as [`XyChunk::TextDelta`].

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;

use crate::domain::error::XyError;
use crate::domain::message::{AgentMessage, AgentPart, XyStopReason};
use crate::domain::types::{XyChunk, XyToolSchema};
use crate::runtime_protocol::XyStream;

use super::LlmAdapter;

/// Adapter for the OpenAI Responses API (`/v1/responses`).
pub struct OpenAiResponsesAdapter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiResponsesAdapter {
    /// Create a new Responses API adapter.
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            headers.insert("authorization", val);
        }
        headers
    }

    fn build_body(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
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
}

#[async_trait]
impl LlmAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        let body = self.build_body(messages, tools, true);
        let url = format!("{}/responses", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| XyError::Provider(anyhow::anyhow!("OpenAI Responses request: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let msg = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status.as_u16(), body_text));
            return Err(XyError::Provider(anyhow::anyhow!(msg)));
        }

        Ok(Box::pin(responses_stream(response)))
    }

    async fn generate(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        let body = self.build_body(messages, tools, false);
        let url = format!("{}/responses", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| XyError::Provider(anyhow::anyhow!("OpenAI Responses request: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let msg = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status.as_u16(), body_text));
            return Err(XyError::Provider(anyhow::anyhow!(msg)));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| XyError::Provider(anyhow::anyhow!("parse response: {e}")))?;
        let chunks = parse_responses_output(&json);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

fn responses_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
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
                        yield XyChunk::ThinkingDelta(delta.to_string());
                    }
                }

                "response.output_text.delta" => {
                    if let Some(delta) = data.get("delta").and_then(|v| v.as_str()) {
                        yield XyChunk::TextDelta(delta.to_string());
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
                            yield XyChunk::FunctionCall { name, args, id };
                        }
                    }
                }

                "response.completed" => {
                    let usage_total = usage_input + usage_output;
                    let usage = if usage_total > 0 {
                        Some(crate::domain::message::XyUsage {
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
                    yield XyChunk::Done {
                        finish_reason: XyStopReason::Stop,
                        usage,
                    };
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

fn parse_responses_output(json: &Value) -> Vec<XyChunk> {
    let mut chunks = Vec::new();

    if let Some(output) = json.get("output").and_then(|v| v.as_array()) {
        for item in output {
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match item_type {
                "reasoning" => {
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                chunks.push(XyChunk::ThinkingDelta(text.to_string()));
                            }
                        }
                    }
                }
                "message" => {
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                chunks.push(XyChunk::TextDelta(text.to_string()));
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
                    chunks.push(XyChunk::FunctionCall { name, args, id });
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
            Some(crate::domain::message::XyUsage {
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

    chunks.push(XyChunk::Done {
        finish_reason: XyStopReason::Stop,
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

// ── AgentMessage conversion ────────────────────────────────────

/// Convert a slice of [`AgentMessage`] values to OpenAI Responses `input` items.
fn convert_messages_to_input_items(messages: &[AgentMessage]) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            AgentMessage::UserMessage { content, .. } => {
                let text = collect_text_parts(content);
                if !text.is_empty() {
                    items.push(serde_json::json!({
                        "role": "user",
                        "content": text,
                    }));
                }
            }
            AgentMessage::AssistantMessage { content, .. } => {
                let text = collect_text_parts(content);
                let tool_calls: Vec<Value> = content
                    .iter()
                    .filter_map(|p| match p {
                        AgentPart::ToolCall {
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
                    items.push(serde_json::json!({
                        "role": "assistant",
                        "content": text,
                    }));
                    items.extend(tool_calls);
                }
            }
            AgentMessage::ToolResultMessage {
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
            AgentMessage::BashExecutionMessage {
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
                    "role": "user",
                    "content": text,
                }));
            }
            AgentMessage::CompactionSummaryMessage { summary, .. }
            | AgentMessage::BranchSummaryMessage { summary, .. } => {
                items.push(serde_json::json!({
                    "role": "user",
                    "content": format!("[Context summary: {summary}]"),
                }));
            }
            AgentMessage::CustomMessage {
                custom_type: _,
                content,
                ..
            } => {
                let text = content.as_str().unwrap_or("").to_string();
                if text.is_empty() {
                    continue;
                }
                items.push(serde_json::json!({
                    "role": "user",
                    "content": text,
                }));
            }
        }
    }

    items
}

fn collect_text_parts(parts: &[AgentPart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let AgentPart::Text(text) | AgentPart::Thinking { text, .. } = part {
            buf.push_str(text);
        }
    }
    buf
}
