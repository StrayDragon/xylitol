use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;

use crate::agent::error::XyError;
use crate::agent::traits::{XyModel, XyStream};
use crate::agent::types::{XyChunk, XyContent, XyFinishReason, XyPart, XyRole, XyToolSchema};

const ANTHROPIC_VERSION: &str = "2023-06-01";

pub(crate) struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl AnthropicProvider {
    pub(crate) fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".into()),
            max_tokens: 8192,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Ok(val) = HeaderValue::from_str(&self.api_key) {
            headers.insert("x-api-key", val);
        }
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers
    }

    fn build_request_body(
        &self,
        messages: &[XyContent],
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Value {
        let (system_prompt, msgs) = xy_to_anthropic(messages);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": msgs,
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

        body
    }
}

#[async_trait]
impl XyModel for AnthropicProvider {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<XyContent>,
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError> {
        let body = self.build_request_body(&messages, tools, stream);
        let url = format!("{}/v1/messages", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| XyError::Provider(anyhow::anyhow!("Anthropic request error: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let msg = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status.as_u16(), body_text));
            return Err(XyError::Provider(anyhow::anyhow!(msg)));
        }

        if stream {
            Ok(Box::pin(anthropic_stream(response)))
        } else {
            let json: Value = response
                .json()
                .await
                .map_err(|e| XyError::Provider(anyhow::anyhow!("parse response: {e}")))?;
            let chunks = parse_anthropic_response(&json);
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }
}

fn anthropic_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;
        use eventsource_stream::Eventsource;

        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();

        let mut tool_accumulators: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut current_block_index: Option<usize> = None;

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
                                    yield XyChunk::TextDelta(text.to_string());
                                }
                            }
                            "thinking" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    yield XyChunk::ThinkingDelta(thinking.to_string());
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

                    if stop_reason.is_some() {
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                            yield XyChunk::FunctionCall { name, args, id };
                        }

                        let finish = match stop_reason {
                            Some("max_tokens") => XyFinishReason::MaxTokens,
                            _ => XyFinishReason::Stop,
                        };
                        yield XyChunk::Done { finish_reason: finish };
                    }
                }

                "message_stop" | "ping" | "message_start" => {}
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
        Some("max_tokens") => XyFinishReason::MaxTokens,
        _ => XyFinishReason::Stop,
    };
    chunks.push(XyChunk::Done {
        finish_reason: finish,
    });

    chunks
}

fn xy_to_anthropic(contents: &[XyContent]) -> (Option<String>, Vec<Value>) {
    let mut system_parts: Vec<String> = Vec::new();
    let mut messages: Vec<Value> = Vec::new();

    for content in contents {
        match content.role {
            XyRole::System => {
                for part in &content.parts {
                    if let XyPart::Text(text) = part {
                        system_parts.push(text.clone());
                    }
                }
            }
            _ => {
                let role = match content.role {
                    XyRole::Assistant => "assistant",
                    _ => "user",
                };

                let blocks: Vec<Value> = content
                    .parts
                    .iter()
                    .map(|part| match part {
                        XyPart::Text(text) => serde_json::json!({
                            "type": "text",
                            "text": text,
                        }),
                        XyPart::Thinking(thinking) => serde_json::json!({
                            "type": "text",
                            "text": thinking,
                        }),
                        XyPart::FunctionCall { name, args, id } => serde_json::json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": args,
                        }),
                        XyPart::FunctionResponse {
                            name: _,
                            result,
                            id,
                        } => serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": id,
                            "content": result,
                        }),
                    })
                    .collect();

                if !blocks.is_empty() {
                    messages.push(serde_json::json!({
                        "role": role,
                        "content": blocks,
                    }));
                }
            }
        }
    }

    merge_consecutive_messages(&mut messages);

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n"))
    };

    (system, messages)
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
