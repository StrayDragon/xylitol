//! OpenAI provider — wraps async-openai for chat completions + streaming.
//!
//! Delegates HTTP, SSE parsing, tool definitions, and error handling to
//! the [`async_openai`] crate. Converts between xylitol's internal types
//! (`XyContent` / `XyChunk`) and async-openai's chat types.

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
use crate::agent::types::{XyChunk, XyContent, XyFinishReason, XyPart, XyRole, XyToolSchema};

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
        messages: Vec<XyContent>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError> {
        let msgs = convert_messages(&messages);
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

// ── Message conversion ─────────────────────────────────────────────

fn convert_messages(contents: &[XyContent]) -> Vec<ChatCompletionRequestMessage> {
    contents
        .iter()
        .filter_map(|content| match content.role {
            XyRole::System => {
                let text = collect_text(&content.parts);
                if text.is_empty() {
                    return None;
                }
                Some(ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage {
                        content:
                            async_openai::types::chat::ChatCompletionRequestSystemMessageContent::Text(
                                text,
                            ),
                        name: None,
                    },
                ))
            }
            XyRole::User => {
                let text = collect_text(&content.parts);
                if text.is_empty() {
                    return None;
                }
                Some(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Text(text),
                        name: None,
                    },
                ))
            }
            XyRole::Assistant => {
                let text = collect_text(&content.parts);
                let tool_calls: Vec<ChatCompletionMessageToolCalls> = content
                    .parts
                    .iter()
                    .filter_map(|p| {
                        if let XyPart::FunctionCall { name, args, id } = p {
                            let args_str = args.to_string();
                            Some(ChatCompletionMessageToolCalls::Function(
                                async_openai::types::chat::ChatCompletionMessageToolCall {
                                    id: id.clone(),
                                    function: FunctionCall {
                                        name: name.clone(),
                                        arguments: args_str,
                                    },
                                },
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();

                let content = if text.is_empty() && tool_calls.is_empty() {
                    text_to_assistant_content(Some(" ".into()))
                } else if text.is_empty() {
                    None
                } else {
                    Some(ChatCompletionRequestAssistantMessageContent::Text(text))
                };

                Some(ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessage {
                        content,
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
                ))
            }
            XyRole::Tool => {
                for part in &content.parts {
                    if let XyPart::FunctionResponse { name: _, result, id } = part {
                        return Some(ChatCompletionRequestMessage::Tool(
                            ChatCompletionRequestToolMessage {
                                content: ChatCompletionRequestToolMessageContent::Text(
                                    result.clone(),
                                ),
                                tool_call_id: id.clone(),
                            },
                        ));
                    }
                }
                None
            }
        })
        .collect()
}

fn collect_text(parts: &[XyPart]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            XyPart::Text(t) | XyPart::Thinking(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert XyPart::Text content to `ChatCompletionRequestAssistantMessageContent`.
fn text_to_assistant_content(
    text: Option<String>,
) -> Option<ChatCompletionRequestAssistantMessageContent> {
    text.filter(|t| !t.is_empty())
        .map(ChatCompletionRequestAssistantMessageContent::Text)
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
                    };
                }
            }
        }
    }
}

// ── Non-streaming response ─────────────────────────────────────────

fn parse_nonstream_response(
    response: &async_openai::types::chat::CreateChatCompletionResponse,
) -> Vec<XyChunk> {
    let mut chunks = Vec::new();

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
        });
    }

    chunks
}
