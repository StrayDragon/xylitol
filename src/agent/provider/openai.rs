use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::Value;

use crate::agent::error::XyError;
use crate::agent::traits::{XyModel, XyStream};
use crate::agent::types::{XyChunk, XyContent, XyFinishReason, XyPart, XyRole, XyToolSchema};

pub struct OpenAIProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIProvider {
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
            headers.insert(AUTHORIZATION, val);
        }
        headers
    }

    fn build_request_body(
        &self,
        messages: &[XyContent],
        tools: &[XyToolSchema],
        stream: bool,
    ) -> Value {
        let msgs = xy_to_openai_messages(messages);

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": msgs,
            "stream": stream,
        });

        if stream {
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }

        if !tools.is_empty() {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        body
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
        let body = self.build_request_body(&messages, tools, stream);
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| XyError::Provider(anyhow::anyhow!("OpenAI request error: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            let msg = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {}: {}", status.as_u16(), body_text));
            return Err(XyError::Provider(anyhow::anyhow!(msg)));
        }

        if stream {
            Ok(Box::pin(openai_stream(response)))
        } else {
            let json: Value = response
                .json()
                .await
                .map_err(|e| XyError::Provider(anyhow::anyhow!("parse response: {e}")))?;
            let chunks = parse_openai_response(&json);
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }
}

fn openai_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;

        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut tool_accumulators: HashMap<u32, (String, String, String)> = HashMap::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result
                .map_err(|e| XyError::Provider(anyhow::anyhow!("stream error: {e}")))?;
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

                    if let Some(reasoning) = delta
                        .get("reasoning_content")
                        .or_else(|| delta.get("reasoning"))
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    {
                        yield XyChunk::ThinkingDelta(reasoning.to_string());
                    }

                    if let Some(text) = delta.get("content").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        yield XyChunk::TextDelta(text.to_string());
                    }

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

                    if finish_reason.is_some() && !tool_accumulators.is_empty() {
                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
                            yield XyChunk::FunctionCall { name, args, id };
                        }
                        yield XyChunk::Done { finish_reason: XyFinishReason::Stop };
                    }
                }
            }
        }
    })
}

fn parse_openai_response(json: &Value) -> Vec<XyChunk> {
    let choice = match json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
    {
        Some(c) => c,
        None => {
            return vec![XyChunk::Done {
                finish_reason: XyFinishReason::Stop,
            }];
        }
    };

    let message = match choice.get("message") {
        Some(m) => m,
        None => {
            return vec![XyChunk::Done {
                finish_reason: XyFinishReason::Stop,
            }];
        }
    };

    let mut chunks = Vec::new();

    if let Some(reasoning) = message
        .get("reasoning_content")
        .or_else(|| message.get("reasoning"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        chunks.push(XyChunk::ThinkingDelta(reasoning.to_string()));
    }

    if let Some(text) = message
        .get("content")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        chunks.push(XyChunk::TextDelta(text.to_string()));
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
                chunks.push(XyChunk::FunctionCall { name, args, id });
            }
        }
    }

    chunks.push(XyChunk::Done {
        finish_reason: XyFinishReason::Stop,
    });
    chunks
}

fn xy_to_openai_messages(contents: &[XyContent]) -> Vec<Value> {
    contents
        .iter()
        .filter_map(|content| {
            let role = match content.role {
                XyRole::System => "system",
                XyRole::User => "user",
                XyRole::Assistant => "assistant",
                XyRole::Tool => "tool",
            };

            if role == "tool" {
                for part in &content.parts {
                    if let XyPart::FunctionResponse {
                        name: _,
                        result,
                        id,
                    } = part
                    {
                        return Some(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": id,
                            "content": result,
                        }));
                    }
                }
                return None;
            }

            if role == "assistant" {
                let tool_calls: Vec<Value> = content
                    .parts
                    .iter()
                    .filter_map(|p| {
                        if let XyPart::FunctionCall { name, args, id } = p {
                            Some(serde_json::json!({
                                "id": id,
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
                        if let XyPart::Text(text) = p {
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

            let text_parts: Vec<&str> = content
                .parts
                .iter()
                .filter_map(|p| match p {
                    XyPart::Text(text) => Some(text.as_str()),
                    XyPart::Thinking(thinking) => Some(thinking.as_str()),
                    _ => None,
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

fn extract_error_message(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?
        .get("message")?
        .as_str()
        .map(String::from)
}
