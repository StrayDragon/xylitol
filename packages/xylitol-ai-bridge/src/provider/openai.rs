//! OpenAI Completions HTTP client — private detail for Chat Completions adapter.
//!
//! Builds chat-completions JSON (via async-openai request types for shape parity),
//! sends with **reqwest** (current transport). Provider script hooks use portable
//! header bags via [`crate::hooks`] + [`crate::provider::reqwest_bridge`]
//! (c998); transport can be swapped without changing hook contracts.
//! Converts between xylitol's internal types and OpenAI chat shapes.
//! Does **not** implement [`XyModel`](crate::runtime_protocol::XyModel);
//! the public path is `OpenAiCompletionsAdapter` → `AdapterXyModel` (c505).

use std::pin::Pin;
use std::sync::Arc;

use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionTool, ChatCompletionTools,
    CreateChatCompletionRequestArgs, FunctionCall, FunctionObject,
};
use futures::Stream;
use serde_json::Value;

use crate::dto::AiBridgeStream;
use crate::dto::{AiBridgeChunk, AiBridgeToolSchema};
use crate::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason, collect_text_parts};
use crate::error::AiBridgeError;
use crate::hooks::{
    HeaderBag, HttpHooks, run_after_response, run_before_headers, run_before_request,
};
use crate::provider::reqwest_bridge::{from_reqwest_headers, to_reqwest_headers};

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    hooks: Option<Arc<dyn HttpHooks>>,
}

impl OpenAIProvider {
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

    async fn send_request(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
    ) -> Result<reqwest::Response, AiBridgeError> {
        let msgs = convert_agent_messages(&messages, None);
        let tool_defs = convert_tools(tools);
        let request = CreateChatCompletionRequestArgs::default()
            .model(self.model.clone())
            .messages(msgs)
            .tools(tool_defs)
            .stream(stream)
            .build()
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("build request: {e}")))?;

        let mut body = serde_json::to_value(&request)
            .map_err(|e| AiBridgeError::Provider(anyhow::anyhow!("serialize request: {e}")))?;
        // Ask compatible endpoints for usage on the final stream chunk. Some
        // OpenAI-compatible servers ignore this; missing usage falls back to
        // accounting (LocalTokenizer / Heuristic) rather than fabricating Api.
        if stream {
            body["stream_options"] = serde_json::json!({ "include_usage": true });
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

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
                AiBridgeError::Provider(anyhow::anyhow!("OpenAI Completions request: {e}"))
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
            return Err(AiBridgeError::Provider(anyhow::anyhow!(
                "HTTP {status}: {body_text}"
            )));
        }

        Ok(response)
    }

    /// Run a chat completion, returning a stream of [`AiBridgeChunk`]s.
    pub async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace =
            crate::provider::trace::ProviderRequestTrace::start("openai-completions", &self.model);
        let response = self.send_request(messages, tools, stream).await?;
        if stream {
            Ok(completions_sse_stream(response, trace))
        } else {
            let json: Value = response.json().await.map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("parse Completions response: {e}"))
            })?;
            if let Some(t) = &trace {
                t.emit_raw("chat.completion.json", &json.to_string());
            }
            let chunks = parse_nonstream_json(&json);
            if let Some(t) = &trace {
                for c in &chunks {
                    t.emit_mapped_chunk(c);
                }
            }
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }
}

// ── Tool conversion ────────────────────────────────────────────────

fn convert_tools(tools: &[AiBridgeToolSchema]) -> Vec<ChatCompletionTools> {
    tools
        .iter()
        .map(|t| {
            ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: t.name.clone(),
                    description: Some(t.description.clone()),
                    parameters: Some(t.parameters.clone()),
                    strict: None,
                },
            })
        })
        .collect()
}

// ── Stream mapping ─────────────────────────────────────────────────

fn completions_sse_stream(
    response: reqwest::Response,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use std::collections::HashMap;

        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();
        let mut tool_accumulators: HashMap<u32, (String, String, String)> = HashMap::new();
        let mut pending_usage: Option<crate::dto::AiBridgeUsage> = None;

        while let Some(event_result) = event_stream.next().await {
            let event = match event_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            if event.data.trim() == "[DONE]" {
                break;
            }
            let data: Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(t) = &trace {
                let snippet = data
                    .pointer("/choices/0/delta/content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                t.emit_raw("chat.completion.chunk", snippet);
            }
            if let Some(u) = data.get("usage") {
                pending_usage = Some(crate::usage::from_openai_usage(u));
            }
            let Some(choices) = data.get("choices").and_then(|c| c.as_array()) else {
                continue;
            };
            for choice in choices {
                if let Some(text) = choice
                    .pointer("/delta/content")
                    .and_then(|v| v.as_str())
                    .filter(|t| !t.is_empty())
                {
                    let chunk = AiBridgeChunk::TextDelta(text.to_string());
                    if let Some(t) = &trace {
                        t.emit_mapped_chunk(&chunk);
                    }
                    yield chunk;
                }

                if let Some(tool_calls) = choice
                    .pointer("/delta/tool_calls")
                    .and_then(|v| v.as_array())
                {
                    for tc in tool_calls {
                        let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                        let entry = tool_accumulators
                            .entry(index)
                            .or_insert_with(|| (String::new(), String::new(), String::new()));
                        if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                            entry.0 = id.to_string();
                        }
                        if let Some(name) = tc.pointer("/function/name").and_then(|v| v.as_str()) {
                            entry.1 = name.to_string();
                        }
                        if let Some(args) = tc.pointer("/function/arguments").and_then(|v| v.as_str())
                        {
                            entry.2.push_str(args);
                        }
                    }
                }

                if let Some(finish_reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                    if !tool_accumulators.is_empty() {
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or(serde_json::json!({}));
                            let chunk = AiBridgeChunk::FunctionCall { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }
                    }
                    let reason = match finish_reason {
                        "length" => AiBridgeStopReason::MaxTokens,
                        _ => AiBridgeStopReason::Stop,
                    };
                    let chunk = AiBridgeChunk::Done {
                        finish_reason: reason,
                        usage: pending_usage.take(),
                    };
                    if let Some(t) = &trace {
                        t.emit_mapped_chunk(&chunk);
                    }
                    yield chunk;
                }
            }
        }
    })
}

// ── Non-streaming response ─────────────────────────────────────────

fn parse_nonstream_json(response: &Value) -> Vec<AiBridgeChunk> {
    let mut chunks = Vec::new();
    let usage = response.get("usage").map(|u| crate::dto::AiBridgeUsage {
        input: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        output: u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read: 0,
        cache_write: 0,
        total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        cache_write_1h: 0,
        cost: None,
    });

    let Some(choices) = response.get("choices").and_then(|c| c.as_array()) else {
        return chunks;
    };

    for choice in choices {
        if let Some(text) = choice
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .filter(|t| !t.is_empty())
        {
            chunks.push(AiBridgeChunk::TextDelta(text.to_string()));
        }

        if let Some(tool_calls) = choice
            .pointer("/message/tool_calls")
            .and_then(|v| v.as_array())
        {
            for tc in tool_calls {
                let id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = tc
                    .pointer("/function/name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = tc
                    .pointer("/function/arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                chunks.push(AiBridgeChunk::FunctionCall { name, args, id });
            }
        }

        let reason = match choice.get("finish_reason").and_then(|v| v.as_str()) {
            Some("length") => AiBridgeStopReason::MaxTokens,
            _ => AiBridgeStopReason::Stop,
        };
        chunks.push(AiBridgeChunk::Done {
            finish_reason: reason,
            usage,
        });
    }

    chunks
}

// ── AiBridgeMessage conversion ────────────────────────────────────

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI chat request
/// messages.
pub fn convert_agent_messages(
    messages: &[AiBridgeMessage],
    system_prompt: Option<&str>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut out = Vec::new();

    if let Some(sp) = system_prompt
        && !sp.is_empty()
    {
        out.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: async_openai::types::chat::ChatCompletionRequestSystemMessageContent::Text(
                    sp.to_string(),
                ),
                name: None,
            },
        ));
    }

    for msg in messages {
        match msg {
            AiBridgeMessage::UserMessage { content, .. } => {
                let text = collect_text_parts(content);
                if text.is_empty() {
                    continue;
                }
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
            }
            AiBridgeMessage::AssistantMessage { content, .. } => {
                let text = collect_text_parts(content);
                let tool_calls: Vec<ChatCompletionMessageToolCalls> = content
                    .iter()
                    .filter_map(|p| match p {
                        AiBridgePart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => Some(ChatCompletionMessageToolCalls::Function(
                            async_openai::types::chat::ChatCompletionMessageToolCall {
                                id: id.clone(),
                                function: FunctionCall {
                                    name: name.clone(),
                                    arguments: arguments.to_string(),
                                },
                            },
                        )),
                        _ => None,
                    })
                    .collect();

                let msg_content = if text.is_empty() {
                    None
                } else {
                    Some(ChatCompletionRequestAssistantMessageContent::Text(text))
                };

                out.push(ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessage {
                        content: msg_content,
                        refusal: None,
                        name: None,
                        audio: None,
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        ..Default::default()
                    },
                ));
            }
            AiBridgeMessage::ToolResultMessage {
                tool_use_id,
                content,
                ..
            } => {
                let text = collect_text_parts(content);
                out.push(ChatCompletionRequestMessage::Tool(
                    ChatCompletionRequestToolMessage {
                        content: ChatCompletionRequestToolMessageContent::Text(text),
                        tool_call_id: tool_use_id.clone(),
                    },
                ));
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
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
            }
            AiBridgeMessage::CompactionSummaryMessage { summary, .. }
            | AiBridgeMessage::BranchSummaryMessage { summary, .. } => {
                let text = format!("[Context summary: {summary}]");
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
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
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AiBridgeMessage, AiBridgeStopReason};

    #[test]
    fn convert_user_message() {
        let msgs = vec![AiBridgeMessage::user("hello world")];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::User(m) => match &m.content {
                ChatCompletionRequestUserMessageContent::Text(t) => {
                    assert_eq!(t, "hello world");
                }
                _ => panic!("expected Text content"),
            },
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn convert_system_prompt() {
        let msgs = vec![AiBridgeMessage::user("hi")];
        let result = convert_agent_messages(&msgs, Some("You are helpful"));
        assert_eq!(result.len(), 2);
        match &result[0] {
            ChatCompletionRequestMessage::System(m) => match &m.content {
                async_openai::types::chat::ChatCompletionRequestSystemMessageContent::Text(t) => {
                    assert_eq!(t, "You are helpful");
                }
                _ => panic!("expected Text content"),
            },
            _ => panic!("expected System message"),
        }
    }

    #[test]
    fn convert_assistant_with_tool_call() {
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::text("Let me check"),
                AiBridgePart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({}),
                },
            ],
            stop_reason: Some(AiBridgeStopReason::ToolUse),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::Assistant(m) => {
                assert!(m.tool_calls.is_some());
                assert_eq!(m.tool_calls.as_ref().unwrap().len(), 1);
            }
            _ => panic!("expected Assistant message"),
        }
    }

    #[test]
    fn convert_tool_result() {
        let msgs = vec![AiBridgeMessage::tool_result(
            "call-1",
            "",
            vec![AiBridgePart::text("done")],
            false,
        )];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::Tool(m) => {
                assert_eq!(m.tool_call_id, "call-1");
            }
            _ => panic!("expected Tool message"),
        }
    }

    #[test]
    fn convert_bash_execution() {
        let msgs = vec![AiBridgeMessage::bash("ls -la", "total 42", Some(0))];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::User(m) => match &m.content {
                ChatCompletionRequestUserMessageContent::Text(t) => {
                    assert!(t.contains("ls -la"));
                    assert!(t.contains("total 42"));
                }
                _ => panic!("expected Text content"),
            },
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn convert_custom_message() {
        let msgs = vec![AiBridgeMessage::CustomMessage {
            custom_type: "diag".into(),
            content: serde_json::json!("diagnostic info"),
            display: serde_json::json!({}),
            details: serde_json::json!({}),
        }];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn convert_compaction_summary() {
        let msgs = vec![AiBridgeMessage::CompactionSummaryMessage {
            summary: "Compressed 50 entries".into(),
            tokens_before: 100000,
            tokens_after: 5000,
            read_files: None,
            modified_files: None,
        }];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
    }
}
