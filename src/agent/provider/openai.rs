//! OpenAI provider — wraps async-openai for chat completions + streaming.
//!
//! Delegates HTTP, SSE parsing, tool definitions, and error handling to
//! the [`async_openai`] crate. Converts between xylitol's internal types
//! (`AgentMessage` / `XyChunk`) and async-openai's chat types.

use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
        ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionTool, ChatCompletionTools,
        CreateChatCompletionRequestArgs, FunctionCall, FunctionObject,
    },
};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::agent::error::XyError;
use crate::agent::traits::{XyModel, XyStream};
use crate::agent::types::{XyChunk, XyFinishReason, XyToolSchema};

pub(crate) struct OpenAIProvider {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAIProvider {
    pub(crate) fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()));
        Self {
            client: Client::with_config(config),
            model,
        }
    }
}

#[async_trait]
impl XyModel for OpenAIProvider {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError> {
        let msgs = convert_agent_messages(&messages, None);
        let tool_defs = convert_tools(tools);

        if stream {
            let request = CreateChatCompletionRequestArgs::default()
                .model(self.model.clone())
                .messages(msgs)
                .tools(tool_defs)
                .stream(true)
                .build()
                .map_err(|e| XyError::Provider(anyhow::anyhow!("build request: {e}")))?;

            match self.client.chat().create_stream(request).await {
                Ok(s) => Ok(Box::pin(map_stream(s))),
                Err(e) => Err(XyError::Provider(anyhow::anyhow!("OpenAI stream: {e}"))),
            }
        } else {
            let request = CreateChatCompletionRequestArgs::default()
                .model(self.model.clone())
                .messages(msgs)
                .tools(tool_defs)
                .stream(false)
                .build()
                .map_err(|e| XyError::Provider(anyhow::anyhow!("build request: {e}")))?;

            match self.client.chat().create(request).await {
                Ok(response) => {
                    let chunks = parse_nonstream_response(&response);
                    Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
                }
                Err(e) => Err(XyError::Provider(anyhow::anyhow!("OpenAI: {e}"))),
            }
        }
    }
}

// ── Tool conversion ────────────────────────────────────────────────

fn convert_tools(tools: &[XyToolSchema]) -> Vec<ChatCompletionTools> {
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

fn map_stream(
    s: async_openai::types::chat::ChatCompletionResponseStream,
) -> impl Stream<Item = Result<XyChunk, XyError>> + Send {
    use async_openai::types::chat::FinishReason;

    async_stream::try_stream! {
        use futures::StreamExt;
        use std::collections::HashMap;

        let mut stream = s;
        let mut tool_accumulators: HashMap<u32, (String, String, String)> = HashMap::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| XyError::Provider(anyhow::anyhow!("stream chunk: {e}")))?;

            for choice in &chunk.choices {
                if let Some(ref text) = choice.delta.content
                    && !text.is_empty()
                {
                    yield XyChunk::TextDelta(text.clone());
                }

                if let Some(ref tool_calls) = choice.delta.tool_calls {
                    for tc in tool_calls {
                        let entry = tool_accumulators
                            .entry(tc.index)
                            .or_insert_with(|| (String::new(), String::new(), String::new()));

                        if let Some(ref id) = tc.id {
                            entry.0 = id.clone();
                        }
                        if let Some(ref func) = tc.function {
                            if let Some(ref name) = func.name {
                                entry.1 = name.clone();
                            }
                            if let Some(ref args) = func.arguments {
                                entry.2.push_str(args);
                            }
                        }
                    }
                }

                if let Some(ref finish_reason) = choice.finish_reason {
                    if !tool_accumulators.is_empty() {
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or(serde_json::json!({}));
                            yield XyChunk::FunctionCall { name, args, id };
                        }
                    }

                    let reason = match finish_reason {
                        FinishReason::Stop => XyFinishReason::Stop,
                        FinishReason::Length => XyFinishReason::MaxTokens,
                        _ => XyFinishReason::Stop,
                    };
                    yield XyChunk::Done {
                        finish_reason: reason,
                        usage: None,
                    };
                }
            }
        }
    }
}

// ── Non-streaming response ─────────────────────────────────────────

/// Extract Usage from OpenAI response.
fn openai_usage(usage: &Option<async_openai::types::chat::CompletionUsage>) -> Option<crate::agent::message::Usage> {
    usage.as_ref().map(|u| crate::agent::message::Usage {
        input: u.prompt_tokens as u64,
        output: u.completion_tokens as u64,
        cache_read: 0,
        cache_write: 0,
        total_tokens: u.total_tokens as u64,
        cache_write_1h: 0,
        cost: None,
    })
}

fn parse_nonstream_response(
    response: &async_openai::types::chat::CreateChatCompletionResponse,
) -> Vec<XyChunk> {
    let mut chunks = Vec::new();
    let usage = openai_usage(&response.usage);

    for choice in &response.choices {
        let msg = &choice.message;

        if let Some(ref text) = msg.content
            && !text.is_empty()
        {
            chunks.push(XyChunk::TextDelta(text.clone()));
        }

        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                match tc {
                    ChatCompletionMessageToolCalls::Function(f) => {
                        let args: Value = serde_json::from_str(&f.function.arguments)
                            .unwrap_or(serde_json::json!({}));
                        let name = f.function.name.clone();
                        chunks.push(XyChunk::FunctionCall {
                            name,
                            args,
                            id: f.id.clone(),
                        });
                    }
                    ChatCompletionMessageToolCalls::Custom(_) => {}
                }
            }
        }

        let reason = match choice.finish_reason {
            Some(async_openai::types::chat::FinishReason::Stop) => XyFinishReason::Stop,
            Some(async_openai::types::chat::FinishReason::Length) => XyFinishReason::MaxTokens,
            _ => XyFinishReason::Stop,
        };
        chunks.push(XyChunk::Done {
            finish_reason: reason,
            usage: usage.clone(),
        });
    }

    chunks
}

// ── AgentMessage conversion ────────────────────────────────────

use crate::agent::message::{AgentMessage, AgentPart, collect_text_parts};

/// Convert a slice of [`AgentMessage`] values to OpenAI chat request
/// messages.
pub fn convert_agent_messages(
    messages: &[AgentMessage],
    system_prompt: Option<&str>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut out = Vec::new();

    if let Some(sp) = system_prompt
        && !sp.is_empty()
    {
        out.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content:
                    async_openai::types::chat::ChatCompletionRequestSystemMessageContent::Text(
                        sp.to_string(),
                    ),
                name: None,
            },
        ));
    }

    for msg in messages {
        match msg {
            AgentMessage::UserMessage { content, .. } => {
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
            AgentMessage::AssistantMessage {
                content,
                ..
            } => {
                let text = collect_text_parts(content);
                let tool_calls: Vec<ChatCompletionMessageToolCalls> = content
                    .iter()
                    .filter_map(|p| match p {
                        AgentPart::ToolCall { id, name, arguments } => {
                            Some(ChatCompletionMessageToolCalls::Function(
                                async_openai::types::chat::ChatCompletionMessageToolCall {
                                    id: id.clone(),
                                    function: FunctionCall {
                                        name: name.clone(),
                                        arguments: arguments.to_string(),
                                    },
                                },
                            ))
                        }
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
            AgentMessage::ToolResultMessage {
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
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
            }
            AgentMessage::CompactionSummaryMessage { summary, .. }
            | AgentMessage::BranchSummaryMessage { summary, .. } => {
                let text = format!("[Context summary: {summary}]");
                out.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ));
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
    use crate::agent::message::{AgentMessage, StopReason};

    #[test]
    fn convert_user_message() {
        let msgs = vec![AgentMessage::user("hello world")];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::User(m) => {
                match &m.content {
                    ChatCompletionRequestUserMessageContent::Text(t) => {
                        assert_eq!(t, "hello world");
                    }
                    _ => panic!("expected Text content"),
                }
            }
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn convert_system_prompt() {
        let msgs = vec![AgentMessage::user("hi")];
        let result = convert_agent_messages(&msgs, Some("You are helpful"));
        assert_eq!(result.len(), 2);
        match &result[0] {
            ChatCompletionRequestMessage::System(m) => {
                match &m.content {
                    async_openai::types::chat::ChatCompletionRequestSystemMessageContent::Text(t) => {
                        assert_eq!(t, "You are helpful");
                    }
                    _ => panic!("expected Text content"),
                }
            }
            _ => panic!("expected System message"),
        }
    }

    #[test]
    fn convert_assistant_with_tool_call() {
        let msgs = vec![AgentMessage::AssistantMessage {
            content: vec![
                AgentPart::Text("Let me check".into()),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({}),
                },
            ],
            stop_reason: Some(StopReason::ToolUse),
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
        let msgs = vec![AgentMessage::tool_result(
            "call-1",
            "",
            vec![AgentPart::Text("done".into())],
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
        let msgs = vec![AgentMessage::bash("ls -la", "total 42", Some(0))];
        let result = convert_agent_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        match &result[0] {
            ChatCompletionRequestMessage::User(m) => {
                match &m.content {
                    ChatCompletionRequestUserMessageContent::Text(t) => {
                        assert!(t.contains("ls -la"));
                        assert!(t.contains("total 42"));
                    }
                    _ => panic!("expected Text content"),
                }
            }
            _ => panic!("expected User message"),
        }
    }

    #[test]
    fn convert_custom_message() {
        let msgs = vec![AgentMessage::CustomMessage {
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
        let msgs = vec![AgentMessage::CompactionSummaryMessage {
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
