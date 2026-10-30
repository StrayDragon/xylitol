use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessage,
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
    ChatCompletionStreamOptions, ChatCompletionTool, ChatCompletionTools,
    CreateChatCompletionRequestArgs, CreateChatCompletionStreamResponse, FinishReason,
    FunctionCall, FunctionObject, ImageUrl,
};
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::dto::XyStream;
use crate::dto::{AgentPart, ImageContent, XyLlmMessage, XyStopReason, collect_text_parts};
use crate::dto::{XyChunk, XyToolSchema};
use crate::error::XyError;
use crate::hooks::HttpHooks;
use crate::provider::native::openai_client::OpenAiClientFactory;
use crate::wire_policy::WirePolicy;
use xylitol_plugin_sdk::error::XyProviderError;

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

    fn map_err(err: async_openai::error::OpenAIError) -> XyError {
        if let async_openai::error::OpenAIError::ApiError(resp) = &err {
            return XyError::provider(XyProviderError {
                message: resp.api_error.message.clone(),
                status: Some(resp.status_code.as_u16()),
                code: resp.api_error.code.clone(),
                error_type: resp.api_error.r#type.clone(),
                retry_after: None,
            });
        }
        XyError::Provider(anyhow::anyhow!("OpenAI Completions: {err}"))
    }

    pub async fn generate_stream(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: &crate::thinking::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
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
            crate::thinking::ThinkingAdapterKind::OpenAi,
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
                XyError::Provider(anyhow::anyhow!("serialize completions request: {e}"))
            })?;
            if let Some(max_tokens) = options.max_output_tokens {
                body["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            self.apply_prompt_cache_affinity(&mut body, options);
            self.apply_structured_output(&mut body, options);
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
                XyError::Provider(anyhow::anyhow!("serialize completions request: {e}"))
            })?;
            if let Some(max_tokens) = options.max_output_tokens {
                body["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            self.apply_prompt_cache_affinity(&mut body, options);
            self.apply_structured_output(&mut body, options);
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

    fn apply_structured_output(
        &self,
        body: &mut Value,
        options: &crate::thinking::XyGenerateOptions,
    ) {
        let Some(output) = options.structured_output.as_ref() else {
            return;
        };
        body["response_format"] = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": &output.name,
                "schema": &output.schema,
                "strict": output.strict.unwrap_or(false),
            }
        });
    }

    fn apply_prompt_cache_affinity(
        &self,
        body: &mut Value,
        options: &crate::thinking::XyGenerateOptions,
    ) {
        if !self.wire_policy.allows_prompt_cache_key() {
            return;
        }
        let Some(session_id) = options
            .obs_session
            .session_id
            .as_deref()
            .filter(|s| !s.is_empty())
        else {
            return;
        };
        body["prompt_cache_key"] = serde_json::json!(session_id);
        if let Some(retention) = self.wire_policy.prompt_cache_retention() {
            body["prompt_cache_retention"] = serde_json::json!(retention.as_wire_str());
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
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        let mut tool_accumulators: HashMap<u32, (String, String, String, bool)> = HashMap::new();
        let mut pending_usage: Option<crate::dto::XyUsage> = None;

        while let Some(item) = tokio::time::timeout(
                crate::provider::native::wait_bounds::SSE_IDLE,
                sdk_stream.next(),
            )
            .await
            .map_err(|_| {
                XyError::Provider(anyhow::anyhow!(
                    "provider SSE idle: no bytes within {}s",
                    crate::provider::native::wait_bounds::SSE_IDLE.as_secs()
                ))
            })? {
            let chunk = item.map_err(|e| {
                XyError::Provider(anyhow::anyhow!("OpenAI Completions stream: {e}"))
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
                    let out = XyChunk::TextDelta(text.clone());
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
                                    let start = XyChunk::ToolCallStart {
                                        id: entry.0.clone(),
                                        name: entry.1.clone(),
                                    };
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&start);
                                    }
                                    yield start;
                                }
                                entry.2.push_str(args);
                                let delta_out = XyChunk::ToolCallDelta {
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
                                let start = XyChunk::ToolCallStart {
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
                                let start = XyChunk::ToolCallStart {
                                    id: id.clone(),
                                    name: name.clone(),
                                };
                                if let Some(t) = &trace {
                                    t.emit_mapped_chunk(&start);
                                }
                                yield start;
                            }
                            let out = XyChunk::ToolCallEnd { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&out);
                            }
                            yield out;
                        }
                    }
                    let reason = match finish_reason {
                        FinishReason::Length => XyStopReason::MaxTokens,
                        FinishReason::ToolCalls => XyStopReason::ToolUse,
                        _ => XyStopReason::Stop,
                    };
                    let out = XyChunk::Done {
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

fn convert_tools(tools: &[XyToolSchema]) -> Vec<ChatCompletionTools> {
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

fn parse_nonstream_json(response: &Value, wire_policy: WirePolicy) -> Vec<XyChunk> {
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
            chunks.push(XyChunk::TextDelta(text.to_string()));
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
                chunks.push(XyChunk::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                });
                chunks.push(XyChunk::ToolCallEnd { name, args, id });
            }
        }

        let reason = match choice.get("finish_reason").and_then(|v| v.as_str()) {
            Some("length") => XyStopReason::MaxTokens,
            _ => XyStopReason::Stop,
        };
        chunks.push(XyChunk::Done {
            finish_reason: reason,
            usage,
        });
    }

    chunks
}

pub fn convert_agent_messages(
    messages: &[XyLlmMessage],
    system_prompt: Option<&str>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut out = Vec::new();
    let mut pending_tool_images: Vec<(String, Vec<ImageContent>)> = Vec::new();

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
            XyLlmMessage::UserMessage { content, .. } => {
                flush_tool_images(&mut out, &mut pending_tool_images);
                let Some(content) = user_message_content(content) else {
                    continue;
                };
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content,
                        name: None,
                    },
                ));
            }
            XyLlmMessage::AssistantMessage { content, .. } => {
                flush_tool_images(&mut out, &mut pending_tool_images);
                let text = collect_text_parts(content);
                let tool_calls: Vec<ChatCompletionMessageToolCalls> = content
                    .iter()
                    .filter_map(|p| match p {
                        AgentPart::ToolCall {
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
            XyLlmMessage::ToolResultMessage {
                tool_use_id,
                content,
                ..
            } => {
                let (message, images) = tool_result_message(tool_use_id, content);
                out.push(message);
                pending_tool_images.extend(images);
            }
        }
    }

    flush_tool_images(&mut out, &mut pending_tool_images);

    out
}

fn tool_result_message(
    tool_use_id: &str,
    content: &[AgentPart],
) -> (
    ChatCompletionRequestMessage,
    Option<(String, Vec<ImageContent>)>,
) {
    let text = collect_text_parts(content);
    let images: Vec<ImageContent> = content
        .iter()
        .filter_map(|part| match part {
            AgentPart::Image(image) => Some(image.clone()),
            _ => None,
        })
        .collect();
    let tool_text = if text.is_empty() && !images.is_empty() {
        "[image]".to_string()
    } else {
        text
    };
    let message = ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
        content: ChatCompletionRequestToolMessageContent::Text(tool_text),
        tool_call_id: tool_use_id.to_string(),
    });
    let pending = (!images.is_empty()).then(|| (tool_use_id.to_string(), images));
    (message, pending)
}

fn user_message_content(content: &[AgentPart]) -> Option<ChatCompletionRequestUserMessageContent> {
    let has_image = content
        .iter()
        .any(|part| matches!(part, AgentPart::Image(_)));
    if !has_image {
        let text = collect_text_parts(content);
        return (!text.is_empty()).then_some(ChatCompletionRequestUserMessageContent::Text(text));
    }

    let mut parts = Vec::new();
    for part in content {
        match part {
            AgentPart::Text { text } if !text.is_empty() => {
                parts.push(ChatCompletionRequestUserMessageContentPart::Text(
                    ChatCompletionRequestMessageContentPartText {
                        text: text.clone(),
                        prompt_cache_breakpoint: None,
                    },
                ));
            }
            AgentPart::Image(image) => {
                parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                    ChatCompletionRequestMessageContentPartImage {
                        image_url: ImageUrl {
                            url: super::image_data_url(image),
                            detail: None,
                        },
                        prompt_cache_breakpoint: None,
                    },
                ));
            }
            _ => {}
        }
    }
    (!parts.is_empty()).then_some(ChatCompletionRequestUserMessageContent::Array(parts))
}

fn flush_tool_images(
    out: &mut Vec<ChatCompletionRequestMessage>,
    pending: &mut Vec<(String, Vec<ImageContent>)>,
) {
    if pending.is_empty() {
        return;
    }
    let ids: Vec<&str> = pending.iter().map(|(id, _)| id.as_str()).collect();
    let mut parts = vec![ChatCompletionRequestUserMessageContentPart::Text(
        ChatCompletionRequestMessageContentPartText {
            text: format!("[tool result images: {}]", ids.join(", ")),
            prompt_cache_breakpoint: None,
        },
    )];
    for (_, images) in pending.drain(..) {
        for image in images {
            parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                ChatCompletionRequestMessageContentPartImage {
                    image_url: ImageUrl {
                        url: super::image_data_url(&image),
                        detail: None,
                    },
                    prompt_cache_breakpoint: None,
                },
            ));
        }
    }
    out.push(ChatCompletionRequestMessage::User(
        ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Array(parts),
            name: None,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{PromptCacheRead, XyLlmMessage, XyStopReason};
    use crate::wire_policy::Compat;

    #[test]
    fn convert_user_message() {
        let msgs = vec![XyLlmMessage::user("hello world")];
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
        let msgs = vec![XyLlmMessage::user("hi")];
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
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::text("Let me check"),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({}),
                },
            ],
            stop_reason: Some(XyStopReason::ToolUse),
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
        let msgs = vec![XyLlmMessage::tool_result(
            "call-1",
            "",
            vec![AgentPart::text("done")],
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
    fn convert_user_image_uses_content_array() {
        let msgs = vec![XyLlmMessage::UserMessage {
            content: vec![
                AgentPart::text("look"),
                AgentPart::image("image/png", "AAA"),
            ],
            timestamp: 0,
        }];
        let result = convert_agent_messages(&msgs, None);
        match &result[0] {
            ChatCompletionRequestMessage::User(message) => match &message.content {
                ChatCompletionRequestUserMessageContent::Array(parts) => {
                    assert!(
                        parts.iter().any(|part| matches!(
                            part,
                            ChatCompletionRequestUserMessageContentPart::ImageUrl(_)
                        )),
                        "user image must be projected, got {parts:?}"
                    );
                }
                _ => panic!("expected array content for an image message"),
            },
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn convert_tool_image_moves_to_trailing_user_message() {
        let msgs = vec![XyLlmMessage::tool_result(
            "call-1",
            "read",
            vec![AgentPart::text("see"), AgentPart::image("image/png", "AAA")],
            false,
        )];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 2, "tool text + trailing image user message");
        match &result[0] {
            ChatCompletionRequestMessage::Tool(message) => {
                assert_eq!(message.tool_call_id, "call-1");
            }
            _ => panic!("expected Tool message first"),
        }
        match &result[1] {
            ChatCompletionRequestMessage::User(message) => match &message.content {
                ChatCompletionRequestUserMessageContent::Array(parts) => {
                    assert!(
                        parts.iter().any(|part| matches!(
                            part,
                            ChatCompletionRequestUserMessageContentPart::ImageUrl(_)
                        )),
                        "tool image must ride in a trailing user message: {parts:?}"
                    );
                }
                _ => panic!("expected array content for the trailing image message"),
            },
            _ => panic!("expected trailing User message"),
        }
    }

    #[test]
    fn convert_projected_bash_as_user() {
        let msgs = vec![XyLlmMessage::user("$ ls -la\ntotal 42")];
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

    fn done_usage(chunks: &[XyChunk]) -> crate::dto::XyUsage {
        chunks
            .iter()
            .find_map(|c| match c {
                XyChunk::Done { usage, .. } => *usage,
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

    #[test]
    fn api_error_maps_to_structured_detail() {
        let err =
            async_openai::error::OpenAIError::ApiError(async_openai::error::ApiErrorResponse {
                status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                api_error: async_openai::error::ApiError {
                    message: "boom".into(),
                    r#type: Some("server_error".into()),
                    param: None,
                    code: Some("server_error".into()),
                    misalignment: None,
                },
            });
        let xy = OpenAIProvider::map_err(err);
        let detail = xy.provider_detail().expect("structured provider detail");
        assert_eq!(detail.status, Some(500));
        assert_eq!(detail.error_type.as_deref(), Some("server_error"));
    }
}
