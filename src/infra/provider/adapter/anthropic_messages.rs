use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::domain::error::XyError;
use crate::domain::message::XyStopReason;
use crate::domain::types::{XyChunk, XyToolSchema};
use crate::infra::hooks::HookDispatcher;
use crate::infra::hooks::http::{
    HeaderBag, run_after_response, run_before_headers, run_before_request,
};
use crate::infra::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};
use crate::runtime_protocol::XyStream;

use super::LlmAdapter;

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Messages API adapter.
pub struct AnthropicMessagesAdapter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    hooks: Option<Arc<HookDispatcher>>,
}

impl AnthropicMessagesAdapter {
    /// Create a new Anthropic Messages adapter.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<HookDispatcher>>,
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
impl LlmAdapter for AnthropicMessagesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        self.execute(messages, tools, true).await
    }

    async fn generate(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
    ) -> Result<XyStream, XyError> {
        self.execute(messages, tools, false).await
    }
}

impl AnthropicMessagesAdapter {
    async fn execute(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError> {
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
            .map_err(|e| XyError::Provider(anyhow::anyhow!("Anthropic request error: {e}")))?;

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
            return Err(XyError::Provider(anyhow::anyhow!(msg)));
        }

        if stream {
            let trace = crate::infra::provider::trace::ProviderRequestTrace::start(
                "anthropic-messages",
                &self.model,
            );
            Ok(Box::pin(anthropic_stream(response, trace)))
        } else {
            let trace = crate::infra::provider::trace::ProviderRequestTrace::start(
                "anthropic-messages",
                &self.model,
            );
            let json: Value = response
                .json()
                .await
                .map_err(|e| XyError::Provider(anyhow::anyhow!("parse response: {e}")))?;
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
    trace: Option<crate::infra::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
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
                            tool_accumulators.insert(index, (id, name, String::new()));
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
                                    let chunk = XyChunk::TextDelta(text.to_string());
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            "thinking" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    let chunk = XyChunk::ThinkingDelta(thinking.to_string());
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
                                }
                            }
                            _ => {}
                        }
                    }
                }

                "content_block_stop" => {
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
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                            yield XyChunk::FunctionCall { name, args, id };
                        }

                        let finish = match stop_reason {
                            Some("max_tokens") => XyStopReason::MaxTokens,
                            _ => XyStopReason::Stop,
                        };

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
                        yield XyChunk::Done { finish_reason: finish, usage };
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

fn parse_anthropic_response(json: &Value) -> Vec<XyChunk> {
    let mut chunks = Vec::new();

    if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
        for block in content {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        chunks.push(XyChunk::TextDelta(text.to_string()));
                    }
                }
                "thinking" => {
                    if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                        chunks.push(XyChunk::ThinkingDelta(thinking.to_string()));
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
                    chunks.push(XyChunk::FunctionCall { name, args, id });
                }
                _ => {}
            }
        }
    }

    let finish = match json.get("stop_reason").and_then(|v| v.as_str()) {
        Some("max_tokens") => XyStopReason::MaxTokens,
        _ => XyStopReason::Stop,
    };

    // Parse usage from non-streaming response
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
        finish_reason: finish,
        usage,
    });

    chunks
}

// ── AgentMessage conversion ────────────────────────────────────

use crate::domain::message::{AgentMessage, AgentPart};

/// Convert a slice of [`AgentMessage`] values to Anthropic request body
/// (returns `(system_prompt, messages)` tuple).
pub fn convert_agent_messages_for_anthropic(
    messages: &[AgentMessage],
) -> (Option<String>, Vec<Value>) {
    let system_parts: Vec<String> = Vec::new();
    let mut msgs: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            AgentMessage::UserMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": blocks,
                    }));
                }
            }
            AgentMessage::AssistantMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "assistant",
                        "content": blocks,
                    }));
                }
            }
            AgentMessage::ToolResultMessage {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                let inner: Vec<Value> = content
                    .iter()
                    .map(|p| match p {
                        AgentPart::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        AgentPart::Image(img) => serde_json::json!({
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
                msgs.push(serde_json::json!({
                    "role": "user",
                    "content": [{"type": "text", "text": text}],
                }));
            }
            AgentMessage::CompactionSummaryMessage { summary, .. }
            | AgentMessage::BranchSummaryMessage { summary, .. } => {
                msgs.push(serde_json::json!({
                    "role": "user",
                    "content": [{
                        "type": "text",
                        "text": format!("[Context summary: {summary}]")
                    }],
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
                msgs.push(serde_json::json!({
                    "role": "user",
                    "content": [{"type": "text", "text": text}],
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

fn agent_parts_to_anthropic_blocks(parts: &[AgentPart]) -> Vec<Value> {
    parts
        .iter()
        .map(|part| match part {
            AgentPart::Text { text } => serde_json::json!({
                "type": "text",
                "text": text,
            }),
            AgentPart::Thinking { thinking, .. } => serde_json::json!({
                "type": "text",
                "text": thinking,
            }),
            AgentPart::Image(img) => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": img.media_type,
                    "data": img.data.as_deref().unwrap_or(""),
                },
            }),
            AgentPart::ToolCall {
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
    use crate::domain::message::{AgentMessage, AgentPart};

    #[test]
    fn convert_user_message() {
        let msgs = vec![AgentMessage::user("hello")];
        let (system, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert!(system.is_none());
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn convert_assistant_with_tool_call() {
        let msgs = vec![AgentMessage::AssistantMessage {
            content: vec![
                AgentPart::text("Let me check"),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "/tmp"}),
                },
            ],
            stop_reason: Some(crate::domain::message::XyStopReason::ToolUse),
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
        let msgs = vec![AgentMessage::tool_result(
            "call-1",
            "",
            vec![AgentPart::text("result here")],
            false,
        )];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "call-1");
    }

    #[test]
    fn convert_bash_execution() {
        let msgs = vec![AgentMessage::bash("ls", "output", Some(0))];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn convert_compaction_summary() {
        let msgs = vec![AgentMessage::CompactionSummaryMessage {
            summary: "Compressed".into(),
            tokens_before: 100,
            tokens_after: 10,
            read_files: None,
            modified_files: None,
        }];
        let (_, msgs) = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
    }
}
