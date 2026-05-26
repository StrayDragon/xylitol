use std::collections::HashMap;
use std::pin::Pin;

use adk_core::{
    AdkError, Content, ErrorCategory, ErrorComponent, Llm, LlmRequest, LlmResponse,
    LlmResponseStream, Part,
};
use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;

pub(crate) struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
    pub(crate) fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
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
            headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    fn build_request_body(&self, req: &LlmRequest, stream: bool) -> Value {
        let messages = contents_to_openai_messages(&req.contents);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": stream,
        });

        if stream {
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|(name, schema)| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": name,
                            "parameters": schema,
                        }
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
            if let Some(max_tokens) = config.max_output_tokens {
                body["max_completion_tokens"] = serde_json::json!(max_tokens);
            }
            if let Some(ref response_schema) = config.response_schema {
                body["response_format"] = serde_json::json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "response",
                        "schema": response_schema,
                        "strict": true,
                    }
                });
            }
        }

        body
    }
}

#[async_trait]
impl Llm for OpenAIProvider {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_content(
        &self,
        req: LlmRequest,
        stream: bool,
    ) -> Result<LlmResponseStream, AdkError> {
        let body = self.build_request_body(&req, stream);
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| openai_request_error(&e))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(openai_http_error(status.as_u16(), &body_text));
        }

        if stream {
            Ok(Box::pin(openai_stream(response)))
        } else {
            let json: Value = response
                .json()
                .await
                .map_err(|e| openai_request_error(&e))?;
            let llm_response = parse_openai_response(&json);
            Ok(Box::pin(futures::stream::once(
                async move { Ok(llm_response) },
            )))
        }
    }
}

fn openai_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<LlmResponse, AdkError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut tool_accumulators: HashMap<u32, (String, String, String)> = HashMap::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| openai_request_error(&e))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                let data = match line.strip_prefix("data: ") {
                    Some(d) => d,
                    None => continue,
                };

                let json: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let choices = match json.get("choices").and_then(|c| c.as_array()) {
                    Some(c) => c,
                    None => continue,
                };

                for choice in choices {
                    let delta = match choice.get("delta") {
                        Some(d) => d,
                        None => continue,
                    };
                    let finish_reason = choice.get("finish_reason").and_then(|f| f.as_str());

                    // Reasoning content (thinking)
                    if let Some(reasoning) = delta
                        .get("reasoning_content")
                        .or_else(|| delta.get("reasoning"))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        yield LlmResponse {
                            content: Some(Content {
                                role: "model".into(),
                                parts: vec![Part::Thinking {
                                    thinking: reasoning.to_string(),
                                    signature: None,
                                }],
                            }),
                            partial: true,
                            turn_complete: false,
                            ..Default::default()
                        };
                    }

                    // Text content
                    if let Some(text) = delta.get("content").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
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

                    // Tool call deltas — accumulate
                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                        for tc in tool_calls {
                            let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                            let entry = tool_accumulators.entry(index).or_insert_with(|| {
                                (String::new(), String::new(), String::new())
                            });
                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                entry.0 = id.to_string();
                            }
                            if let Some(func) = tc.get("function") {
                                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                    entry.1 = name.to_string();
                                }
                                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                    entry.2.push_str(args);
                                }
                            }
                        }
                    }

                    // On finish_reason, emit accumulated tool calls
                    if finish_reason.is_some() && !tool_accumulators.is_empty() {
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
                        yield LlmResponse {
                            content: Some(Content {
                                role: "model".into(),
                                parts,
                            }),
                            partial: false,
                            turn_complete: true,
                            finish_reason: Some(adk_core::FinishReason::Stop),
                            ..Default::default()
                        };
                    }
                }

                // Usage (final chunk)
                if let Some(usage) = json.get("usage") {
                    let _prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64());
                    let _completion = usage.get("completion_tokens").and_then(|v| v.as_u64());
                }
            }
        }
    })
}

fn parse_openai_response(json: &Value) -> LlmResponse {
    let choice = match json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
    {
        Some(c) => c,
        None => return LlmResponse::default(),
    };

    let message = match choice.get("message") {
        Some(m) => m,
        None => return LlmResponse::default(),
    };

    let mut parts = Vec::new();

    if let Some(reasoning) = message
        .get("reasoning_content")
        .or_else(|| message.get("reasoning"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        parts.push(Part::Thinking {
            thinking: reasoning.to_string(),
            signature: None,
        });
    }

    if let Some(text) = message
        .get("content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        parts.push(Part::Text {
            text: text.to_string(),
        });
    }

    if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
        for tc in tool_calls {
            let id = tc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(func) = tc.get("function") {
                let name = func
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = func
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let args: Value = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                parts.push(Part::FunctionCall {
                    name,
                    args,
                    id: Some(id),
                    thought_signature: None,
                });
            }
        }
    }

    LlmResponse {
        content: Some(Content {
            role: "model".into(),
            parts,
        }),
        partial: false,
        turn_complete: true,
        finish_reason: Some(adk_core::FinishReason::Stop),
        ..Default::default()
    }
}

fn contents_to_openai_messages(contents: &[Content]) -> Vec<Value> {
    contents
        .iter()
        .filter_map(|content| {
            let role = match content.role.as_str() {
                "system" => "system",
                "user" => "user",
                "model" | "assistant" => "assistant",
                "function" | "tool" => "tool",
                _ => "user",
            };

            // Tool messages need special handling
            if role == "tool" {
                for part in &content.parts {
                    if let Part::FunctionResponse {
                        function_response,
                        id,
                    } = part
                    {
                        let fallback_id = format!("call_{}", function_response.name);
                        let tool_call_id = id.as_deref().unwrap_or(&fallback_id);
                        let content_str = match &function_response.response {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        return Some(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "content": content_str,
                        }));
                    }
                }
                return None;
            }

            // Assistant messages with tool calls
            if role == "assistant" {
                let tool_calls: Vec<Value> = content
                    .parts
                    .iter()
                    .filter_map(|p| {
                        if let Part::FunctionCall { name, args, id, .. } = p {
                            Some(serde_json::json!({
                                "id": id.as_deref().unwrap_or(&format!("call_{name}")),
                                "type": "function",
                                "function": {
                                    "name": name,
                                    "arguments": args.to_string(),
                                }
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();

                let text_parts: Vec<&str> = content
                    .parts
                    .iter()
                    .filter_map(|p| {
                        if let Part::Text { text } = p {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect();

                let text = if text_parts.is_empty() {
                    if tool_calls.is_empty() {
                        " ".to_string()
                    } else {
                        String::new()
                    }
                } else {
                    text_parts.join("\n")
                };

                let mut msg = serde_json::json!({"role": "assistant"});
                if !text.is_empty() {
                    msg["content"] = Value::String(text);
                } else {
                    msg["content"] = Value::Null;
                }
                if !tool_calls.is_empty() {
                    msg["tool_calls"] = Value::Array(tool_calls);
                }
                return Some(msg);
            }

            // System / user messages
            let text_parts: Vec<&str> = content
                .parts
                .iter()
                .filter_map(|p| {
                    if let Part::Text { text } = p {
                        Some(text.as_str())
                    } else if let Part::Thinking { thinking, .. } = p {
                        Some(thinking.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            if text_parts.is_empty() {
                return None;
            }

            Some(serde_json::json!({
                "role": role,
                "content": text_parts.join("\n"),
            }))
        })
        .collect()
}

fn openai_request_error(e: &reqwest::Error) -> AdkError {
    AdkError::new(
        ErrorComponent::Model,
        ErrorCategory::Internal,
        "openai.request_error",
        e.to_string(),
    )
}

fn openai_http_error(status: u16, body: &str) -> AdkError {
    let message = extract_error_message(body).unwrap_or_else(|| format!("HTTP {status}: {body}"));

    let category = match status {
        401 => ErrorCategory::InvalidInput,
        429 => ErrorCategory::RateLimited,
        500..=599 => ErrorCategory::Unavailable,
        _ => ErrorCategory::Internal,
    };

    AdkError::new(
        ErrorComponent::Model,
        category,
        "openai.http_error",
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
