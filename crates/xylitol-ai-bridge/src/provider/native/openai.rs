//! OpenAI Completions via `async-openai` [`Client`] (c1070).
//!
//! Transport is the official Client with [`crate::provider::openai_hooks_mw::HooksHttpService`]
//! for portable [`crate::hooks::HttpHooks`]. Request shapes use async-openai chat types.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionStreamOptions, ChatCompletionTool,
    ChatCompletionTools, CreateChatCompletionRequestArgs, CreateChatCompletionStreamResponse,
    FinishReason, FunctionCall, FunctionObject,
};
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::dto::AiBridgeStream;
use crate::dto::{AiBridgeChunk, AiBridgeToolSchema};
use crate::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason, collect_text_parts};
use crate::error::AiBridgeError;
use crate::hooks::HttpHooks;
use crate::provider::native::openai_client::OpenAiClientFactory;
use crate::wire_policy::WirePolicy;

pub struct OpenAIProvider {
    client: OpenAiClientFactory,
    model: String,
    wire_policy: WirePolicy,
}

impl OpenAIProvider {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self::with_wire_policy(api_key, model, base_url, hooks, WirePolicy::default())
    }

    pub fn with_wire_policy(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
        wire_policy: WirePolicy,
    ) -> Self {
        Self {
            client: OpenAiClientFactory::new(api_key, base_url, hooks),
            model,
            wire_policy,
        }
    }

    fn map_err(err: async_openai::error::OpenAIError) -> AiBridgeError {
        AiBridgeError::Provider(anyhow::anyhow!("OpenAI Completions: {err}"))
    }

    /// Run a chat completion, returning a stream of [`AiBridgeChunk`]s.
    pub async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: &crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "openai-completions",
            &self.model,
            options.obs_parent,
            &crate::provider::obs_for_llm_request(&options.obs_session, self.client.api_base()),
        );
        let msgs = convert_agent_messages(&messages, options.system_prompt.as_deref());
        let tool_defs = convert_tools(tools);

        let resolved = crate::thinking::resolve_from_options(
            options,
            crate::thinking::AiBridgeThinkingAdapterKind::OpenAi,
        );

        if stream {
            let request = CreateChatCompletionRequestArgs::default()
                .model(self.model.clone())
                .messages(msgs)
                .tools(tool_defs)
                .stream(true)
                .stream_options(ChatCompletionStreamOptions {
                    include_usage: Some(true),
                    include_obfuscation: None,
                })
                .build()
                .map_err(Self::map_err)?;

            let mut body = serde_json::to_value(&request).map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("serialize completions request: {e}"))
            })?;
            if let Some(max_tokens) = options.max_output_tokens {
                body["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            crate::provider::dialect::apply_completions_thinking(
                &mut body,
                &resolved,
                self.wire_policy.compat,
            );
            if let Some(t) = &trace {
                t.capture_request_input(&body.to_string());
            }

            let sdk_stream = self
                .client
                .bind(&options.obs_session)
                .chat()
                .create_stream_byot::<_, CreateChatCompletionStreamResponse>(body)
                .await
                .map_err(Self::map_err)?;

            Ok(completions_sdk_stream(sdk_stream, trace, self.wire_policy))
        } else {
            let request = CreateChatCompletionRequestArgs::default()
                .model(self.model.clone())
                .messages(msgs)
                .tools(tool_defs)
                .stream(false)
                .build()
                .map_err(Self::map_err)?;

            let mut body = serde_json::to_value(&request).map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("serialize completions request: {e}"))
            })?;
            if let Some(max_tokens) = options.max_output_tokens {
                body["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            crate::provider::dialect::apply_completions_thinking(
                &mut body,
                &resolved,
                self.wire_policy.compat,
            );
            if let Some(t) = &trace {
                t.capture_request_input(&body.to_string());
            }

            let json: Value = self
                .client
                .bind(&options.obs_session)
                .chat()
                .create_byot(body)
                .await
                .map_err(Self::map_err)?;

            if let Some(t) = &trace {
                t.emit_raw("chat.completion.json", &json.to_string());
            }
            let chunks = parse_nonstream_json(&json, self.wire_policy);
            if let Some(t) = &trace {
                for c in &chunks {
                    t.emit_mapped_chunk(c);
                }
            }
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }
}

fn completions_sdk_stream(
    mut sdk_stream: impl Stream<
        Item = Result<CreateChatCompletionStreamResponse, async_openai::error::OpenAIError>,
    > + Send
    + Unpin
    + 'static,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
    wire_policy: WirePolicy,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        let mut tool_accumulators: HashMap<u32, (String, String, String, bool)> = HashMap::new();
        let mut pending_usage: Option<crate::dto::AiBridgeUsage> = None;

        while let Some(item) = tokio::time::timeout(
                crate::provider::native::wait_bounds::SSE_IDLE,
                sdk_stream.next(),
            )
            .await
            .map_err(|_| {
                AiBridgeError::Provider(anyhow::anyhow!(
                    "provider SSE idle: no bytes within {}s",
                    crate::provider::native::wait_bounds::SSE_IDLE.as_secs()
                ))
            })? {
            let chunk = item.map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("OpenAI Completions stream: {e}"))
            })?;

            if let Some(t) = &trace {
                let snippet = chunk
                    .choices
                    .first()
                    .and_then(|c| c.delta.content.as_deref())
                    .unwrap_or("");
                t.emit_raw("chat.completion.chunk", snippet);
            }

            if let Some(u) = &chunk.usage {
                let usage_json = serde_json::to_value(u).unwrap_or(Value::Null);
                pending_usage = Some(crate::usage::from_openai_usage_with_policy(
                    &usage_json,
                    wire_policy,
                ));
            }

            for choice in &chunk.choices {
                if let Some(text) = choice.delta.content.as_ref().filter(|t| !t.is_empty()) {
                    let out = AiBridgeChunk::TextDelta(text.clone());
                    if let Some(t) = &trace {
                        t.emit_mapped_chunk(&out);
                    }
                    yield out;
                }

                if let Some(tool_calls) = &choice.delta.tool_calls {
                    for tc in tool_calls {
                        let entry = tool_accumulators
                            .entry(tc.index)
                            .or_insert_with(|| (String::new(), String::new(), String::new(), false));
                        if let Some(id) = &tc.id {
                            entry.0 = id.clone();
                        }
                        if let Some(func) = &tc.function {
                            if let Some(name) = &func.name {
                                entry.1 = name.clone();
                            }
                            if let Some(args) = &func.arguments {
                                if !entry.3 {
                                    entry.3 = true;
                                    let start = AiBridgeChunk::ToolCallStart {
                                        id: entry.0.clone(),
                                        name: entry.1.clone(),
                                    };
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&start);
                                    }
                                    yield start;
                                }
                                entry.2.push_str(args);
                                let delta_out = AiBridgeChunk::ToolCallDelta {
                                    id: entry.0.clone(),
                                    name: entry.1.clone(),
                                    args_delta: args.clone(),
                                    args: crate::dto::parse_streaming_json(&entry.2),
                                };
                                if let Some(t) = &trace {
                                    t.emit_mapped_chunk(&delta_out);
                                }
                                yield delta_out;
                            } else if !entry.3 && (!entry.0.is_empty() || !entry.1.is_empty()) {
                                entry.3 = true;
                                let start = AiBridgeChunk::ToolCallStart {
                                    id: entry.0.clone(),
                                    name: entry.1.clone(),
                                };
                                if let Some(t) = &trace {
                                    t.emit_mapped_chunk(&start);
                                }
                                yield start;
                            }
                        }
                    }
                }

                if let Some(finish_reason) = choice.finish_reason {
                    if !tool_accumulators.is_empty() {
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str, started)) in sorted {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
                            if !started {
                                let start = AiBridgeChunk::ToolCallStart {
                                    id: id.clone(),
                                    name: name.clone(),
                                };
                                if let Some(t) = &trace {
                                    t.emit_mapped_chunk(&start);
                                }
                                yield start;
                            }
                            let out = AiBridgeChunk::ToolCallEnd { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&out);
                            }
                            yield out;
                        }
                    }
                    let reason = match finish_reason {
                        FinishReason::Length => AiBridgeStopReason::MaxTokens,
                        FinishReason::ToolCalls => AiBridgeStopReason::ToolUse,
                        _ => AiBridgeStopReason::Stop,
                    };
                    let out = AiBridgeChunk::Done {
                        finish_reason: reason,
                        usage: pending_usage.take(),
                    };
                    if let Some(t) = &trace {
                        t.emit_mapped_chunk(&out);
                    }
                    yield out;
                }
            }
        }
    })
}

// ── Tool conversion ────────────────────────────────────────────────

fn convert_tools(tools: &[AiBridgeToolSchema]) -> Vec<ChatCompletionTools> {
    tools
        .iter()
        .map(|t| {
            ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: crate::provider::tool_wire::to_wire_tool_name(&t.name),
                    description: Some(t.description.clone()),
                    parameters: Some(t.parameters.clone()),
                    strict: None,
                },
            })
        })
        .collect()
}

// ── Non-streaming response ─────────────────────────────────────────

fn parse_nonstream_json(response: &Value, wire_policy: WirePolicy) -> Vec<AiBridgeChunk> {
    let mut chunks = Vec::new();
    let usage = response
        .get("usage")
        .map(|u| crate::usage::from_openai_usage_with_policy(u, wire_policy));

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
                chunks.push(AiBridgeChunk::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                });
                chunks.push(AiBridgeChunk::ToolCallEnd { name, args, id });
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
                                    name: crate::provider::tool_wire::to_wire_tool_name(name),
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
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AiBridgeMessage, AiBridgeStopReason, PromptCacheRead};
    use crate::wire_policy::Compat;

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
    fn convert_projected_bash_as_user() {
        let msgs = vec![AiBridgeMessage::user("$ ls -la\ntotal 42")];
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

    fn done_usage(chunks: &[AiBridgeChunk]) -> crate::dto::AiBridgeUsage {
        chunks
            .iter()
            .find_map(|c| match c {
                AiBridgeChunk::Done { usage, .. } => *usage,
                _ => None,
            })
            .expect("Done usage")
    }

    #[test]
    fn parse_nonstream_prefers_prompt_cache_hit_tokens() {
        let json = serde_json::json!({
            "choices": [{
                "message": { "content": "ok" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 754,
                "completion_tokens": 10,
                "total_tokens": 764,
                "prompt_cache_hit_tokens": 640,
                "prompt_tokens_details": { "cached_tokens": 1 }
            }
        });
        let chunks = parse_nonstream_json(&json, WirePolicy::for_compat(Compat::Deepseek));
        let u = done_usage(&chunks);
        assert_eq!(u.input, 754);
        assert_eq!(u.output, 10);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(640));
    }

    #[test]
    fn parse_nonstream_falls_back_to_cached_tokens() {
        let json = serde_json::json!({
            "choices": [{
                "message": { "content": "ok" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 754,
                "completion_tokens": 10,
                "total_tokens": 764,
                "prompt_tokens_details": { "cached_tokens": 640 }
            }
        });
        let chunks = parse_nonstream_json(&json, WirePolicy::for_compat(Compat::Deepseek));
        assert_eq!(
            done_usage(&chunks).prompt_cache_read,
            PromptCacheRead::Tokens(640)
        );
    }

    #[test]
    fn parse_nonstream_missing_cache_fields_are_not_reported() {
        let json = serde_json::json!({
            "choices": [{
                "message": { "content": "ok" },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 5,
                "total_tokens": 105
            }
        });
        let chunks = parse_nonstream_json(&json, WirePolicy::for_compat(Compat::Deepseek));
        let u = done_usage(&chunks);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::NotReported);
        assert_eq!(u.cache_read, 0);
    }
}
