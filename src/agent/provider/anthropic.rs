use std::collections::HashMap;
use std::pin::Pin;

use adk_core::{
    AdkError, Content, ErrorCategory, ErrorComponent, Llm, LlmRequest, LlmResponse,
    LlmResponseStream, Part,
};
use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;

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

    fn build_request_body(&self, req: &LlmRequest, stream: bool) -> Value {
        let (system_prompt, messages) = contents_to_anthropic(&req.contents);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "stream": stream,
        });

        if let Some(system) = system_prompt {
            body["system"] = Value::String(system);
        }

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|(name, schema)| {
                    serde_json::json!({
                        "name": name,
                        "input_schema": schema,
                    })
                })
                .collect();
            body["tools"] = Value::Array(tools);
        }

        if let Some(ref config) = req.config {
            if let Some(temp) = config.temperature {
                body["temperature"] = serde_json::json!(temp);
            }
            if let Some(top_p) = config.top_p {
                body["top_p"] = serde_json::json!(top_p);
            }
            if let Some(top_k) = config.top_k {
                body["top_k"] = serde_json::json!(top_k);
            }
            if let Some(max_tokens) = config.max_output_tokens {
                body["max_tokens"] = serde_json::json!(max_tokens);
            }
        }

        body
    }
}

#[async_trait]
impl Llm for AnthropicProvider {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_content(
        &self,
        req: LlmRequest,
        stream: bool,
    ) -> Result<LlmResponseStream, AdkError> {
        let body = self.build_request_body(&req, stream);
        let url = format!("{}/v1/messages", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anthropic_request_error(&e))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(anthropic_http_error(status.as_u16(), &body_text));
        }

        if stream {
            Ok(Box::pin(anthropic_stream(response)))
        } else {
            let json: Value = response
                .json()
                .await
                .map_err(|e| anthropic_request_error(&e))?;
            let llm_response = parse_anthropic_response(&json);
            Ok(Box::pin(futures::stream::once(
                async move { Ok(llm_response) },
            )))
        }
    }
}

fn anthropic_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<LlmResponse, AdkError>> + Send>> {
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
                                    yield LlmResponse {
                                        content: Some(Content {
                                            role: "model".into(),
                                            parts: vec![Part::Text { text: text.to_string() }],
                                        }),
                                        partial: true,
                                        turn_complete: false,
                                        ..Default::default()
                                    };
                                }
                            }
                            "thinking" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    yield LlmResponse {
                                        content: Some(Content {
                                            role: "model".into(),
                                            parts: vec![Part::Thinking {
                                                thinking: thinking.to_string(),
                                                signature: None,
                                            }],
                                        }),
                                        partial: true,
                                        turn_complete: false,
                                        ..Default::default()
                                    };
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
                        let mut parts: Vec<Part> = Vec::new();

                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                            parts.push(Part::FunctionCall {
                                name,
                                args,
                                id: Some(id),
                                thought_signature: None,
                            });
                        }

                        let finish_reason = match stop_reason {
                            Some("max_tokens") => Some(adk_core::FinishReason::MaxTokens),
                            _ => Some(adk_core::FinishReason::Stop),
                        };

                        if !parts.is_empty() {
                            yield LlmResponse {
                                content: Some(Content {
                                    role: "model".into(),
                                    parts,
                                }),
                                partial: false,
                                turn_complete: true,
                                finish_reason,
                                ..Default::default()
                            };
                        } else {
                            yield LlmResponse {
                                partial: false,
                                turn_complete: true,
                                finish_reason,
                                ..Default::default()
                            };
                        }
                    }
                }

                "message_stop" | "ping" | "message_start" => {}

                _ => {}
            }
        }
    })
}

fn parse_anthropic_response(json: &Value) -> LlmResponse {
    let mut parts = Vec::new();

    if let Some(content) = json.get("content").and_then(|v| v.as_array()) {
        for block in content {
            let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        parts.push(Part::Text {
                            text: text.to_string(),
                        });
                    }
                }
                "thinking" => {
                    if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                        let signature = block
                            .get("signature")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(String::from);
                        parts.push(Part::Thinking {
                            thinking: thinking.to_string(),
                            signature,
                        });
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
                    parts.push(Part::FunctionCall {
                        name,
                        args,
                        id: Some(id),
                        thought_signature: None,
                    });
                }
                _ => {}
            }
        }
    }

    let finish_reason = match json.get("stop_reason").and_then(|v| v.as_str()) {
        Some("max_tokens") => Some(adk_core::FinishReason::MaxTokens),
        _ => Some(adk_core::FinishReason::Stop),
    };

    LlmResponse {
        content: Some(Content {
            role: "model".into(),
            parts,
        }),
        partial: false,
        turn_complete: true,
        finish_reason,
        ..Default::default()
    }
}

fn contents_to_anthropic(contents: &[Content]) -> (Option<String>, Vec<Value>) {
    let mut system_parts: Vec<String> = Vec::new();
    let mut messages: Vec<Value> = Vec::new();

    for content in contents {
        match content.role.as_str() {
            "system" => {
                for part in &content.parts {
                    if let Part::Text { text } = part {
                        system_parts.push(text.clone());
                    }
                }
            }
            _ => {
                let role = match content.role.as_str() {
                    "model" | "assistant" => "assistant",
                    _ => "user",
                };

                let blocks: Vec<Value> = content
                    .parts
                    .iter()
                    .filter_map(|part| match part {
                        Part::Text { text } => Some(serde_json::json!({
                            "type": "text",
                            "text": text,
                        })),
                        Part::Thinking { thinking, .. } => Some(serde_json::json!({
                            "type": "text",
                            "text": thinking,
                        })),
                        Part::FunctionCall { name, args, id, .. } => Some(serde_json::json!({
                            "type": "tool_use",
                            "id": id.as_deref().unwrap_or(&format!("call_{name}")),
                            "name": name,
                            "input": args,
                        })),
                        Part::FunctionResponse { function_response, id } => {
                            let content_str = match &function_response.response {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            Some(serde_json::json!({
                                "type": "tool_result",
                                "tool_use_id": id.as_deref().unwrap_or(&format!("call_{}", function_response.name)),
                                "content": content_str,
                            }))
                        }
                        _ => None,
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

    // Merge consecutive same-role messages (required for parallel tool use)
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

fn anthropic_request_error(e: &reqwest::Error) -> AdkError {
    AdkError::new(
        ErrorComponent::Model,
        ErrorCategory::Internal,
        "anthropic.request_error",
        e.to_string(),
    )
}

fn anthropic_http_error(status: u16, body: &str) -> AdkError {
    let message = extract_error_message(body).unwrap_or_else(|| format!("HTTP {status}: {body}"));

    let category = match status {
        401 => ErrorCategory::InvalidInput,
        429 => ErrorCategory::RateLimited,
        529 => ErrorCategory::Unavailable,
        500..=599 => ErrorCategory::Unavailable,
        _ => ErrorCategory::Internal,
    };

    AdkError::new(
        ErrorComponent::Model,
        category,
        "anthropic.http_error",
        message,
    )
}

fn extract_error_message(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?
        .get("message")?
        .as_str()
        .map(String::from)
}
