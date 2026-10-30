use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::dto::{AgentPart, XyChunk, XyStopReason, XyUsage};
use crate::hooks::{
    HeaderBag, HttpHooks, run_after_response, run_before_headers, run_before_request,
};
use crate::provider::reqwest_headers::{from_reqwest_headers, to_reqwest_headers};

use xylitol_plugin_sdk::error::XyProviderError;
use xylitol_plugin_sdk::message::XyLlmMessage;
use xylitol_plugin_sdk::model::{XyPromptCacheConfig, XyPromptCacheTtl};
use xylitol_plugin_sdk::{XyError, XyGenerateOptions, XyModel, XyStream, XyToolSchema};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const EXTENDED_CACHE_TTL_BETA: &str = "extended-cache-ttl-2025-04-11";

pub struct AnthropicMessagesAdapter {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
    hooks: Option<Arc<dyn HttpHooks>>,
    wire_policy: crate::wire_policy::WirePolicy,
    prompt_cache: Option<XyPromptCacheConfig>,
}

impl AnthropicMessagesAdapter {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self::with_wire_policy(
            api_key,
            model,
            base_url,
            hooks,
            crate::wire_policy::WirePolicy::default(),
        )
    }

    pub fn with_wire_policy(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
        wire_policy: crate::wire_policy::WirePolicy,
    ) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(crate::provider::native::DEFAULT_HTTP_USER_AGENT)
            .connect_timeout(crate::provider::native::wait_bounds::CONNECT)
            .build()
            .unwrap_or_else(|_| {
                reqwest::Client::builder()
                    .connect_timeout(crate::provider::native::wait_bounds::CONNECT)
                    .build()
                    .expect("default reqwest client with connect timeout")
            });
        Self {
            client,
            api_key,
            model,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".into()),
            max_tokens: 8192,
            hooks,
            wire_policy,
            prompt_cache: None,
        }
    }

    pub fn with_prompt_cache(mut self, prompt_cache: Option<XyPromptCacheConfig>) -> Self {
        self.prompt_cache = prompt_cache;
        self
    }

    fn prompt_cache_ttl(&self) -> Option<XyPromptCacheTtl> {
        let config = self.prompt_cache;
        let enabled = config
            .and_then(|c| c.enabled)
            .unwrap_or(self.wire_policy.compat != crate::wire_policy::Compat::Deepseek);
        if !enabled {
            return None;
        }
        Some(
            config
                .and_then(|c| c.ttl)
                .unwrap_or(XyPromptCacheTtl::FiveMinutes),
        )
    }

    fn headers_bag(&self, obs: &crate::provider::obs_session::ObsSessionContext) -> HeaderBag {
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
        crate::provider::attribution::merge_opencode_attribution_from(
            &mut headers,
            &self.base_url,
            obs,
        );
        headers
    }
}

fn apply_anthropic_prompt_cache(body: &mut Value, ttl: XyPromptCacheTtl) {
    let control = match ttl {
        XyPromptCacheTtl::FiveMinutes => serde_json::json!({"type": "ephemeral"}),
        XyPromptCacheTtl::OneHour => serde_json::json!({"type": "ephemeral", "ttl": "1h"}),
    };

    if let Some(system) = body.get_mut("system") {
        let text = system.as_str().map(str::to_string);
        match text {
            Some(text) => {
                *system = serde_json::json!([{
                    "type": "text",
                    "text": text,
                    "cache_control": control,
                }]);
            }
            None => {
                if let Some(last) = system.as_array_mut().and_then(|b| b.last_mut())
                    && let Some(obj) = last.as_object_mut()
                {
                    obj.insert("cache_control".into(), control.clone());
                }
            }
        }
    }

    if let Some(last) = body
        .get_mut("tools")
        .and_then(|t| t.as_array_mut())
        .and_then(|t| t.last_mut())
        && let Some(obj) = last.as_object_mut()
    {
        obj.insert("cache_control".into(), control.clone());
    }

    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        'outer: for message in messages.iter_mut().rev() {
            let Some(content) = message.get_mut("content").and_then(|c| c.as_array_mut()) else {
                continue;
            };
            for block in content.iter_mut().rev() {
                let Some(obj) = block.as_object_mut() else {
                    continue;
                };
                let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if block_type == "text" || block_type == "tool_result" {
                    obj.insert("cache_control".into(), control.clone());
                    break 'outer;
                }
            }
        }
    }
}

#[async_trait]
impl XyModel for AnthropicMessagesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        self.execute(messages, tools, stream, options).await
    }
}

impl AnthropicMessagesAdapter {
    async fn execute(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: crate::thinking::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let anthropic_msgs = convert_agent_messages_for_anthropic(&messages);
        let system_prompt = options.system_prompt.clone();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": anthropic_msgs,
            "max_tokens": options.max_output_tokens.unwrap_or(self.max_tokens),
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
                        "name": crate::provider::tool_wire::to_wire_tool_name(&t.name),
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        let resolved = crate::thinking::resolve_from_options(
            &options,
            crate::thinking::ThinkingAdapterKind::Anthropic,
        );
        if let crate::thinking::ResolvedThinking::Invalid(reason) = &resolved {
            return Err(XyError::Provider(anyhow::anyhow!(reason.clone())));
        }
        crate::provider::dialect::apply_anthropic_thinking(
            &mut body,
            &resolved,
            self.wire_policy.compat,
        );

        if let Some(output) = options.structured_output.as_ref() {
            body["output_config"] = serde_json::json!({
                "format": {
                    "type": "json_schema",
                    "schema": &output.schema,
                }
            });
        }

        let cache_ttl = self.prompt_cache_ttl();
        if let Some(ttl) = cache_ttl {
            apply_anthropic_prompt_cache(&mut body, ttl);
        }

        let url = format!("{}/v1/messages", self.base_url);

        let mut headers = self.headers_bag(&options.obs_session);
        if matches!(cache_ttl, Some(XyPromptCacheTtl::OneHour)) {
            headers.insert(
                "anthropic-beta".into(),
                Value::String(EXTENDED_CACHE_TTL_BETA.into()),
            );
        }
        run_before_headers(&self.hooks, &mut headers).await?;
        run_before_request(&self.hooks, &self.model, &mut body).await?;

        let mut request = self
            .client
            .post(&url)
            .headers(to_reqwest_headers(&headers))
            .json(&body);
        if !stream {
            request = request.timeout(crate::provider::native::wait_bounds::NON_STREAM_TOTAL);
        }
        let response = request
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
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(std::time::Duration::from_secs);
            let body_text = response.text().await.unwrap_or_default();
            let error_type = extract_error_type(&body_text);
            let message = extract_error_message(&body_text)
                .unwrap_or_else(|| format!("HTTP {status}: {body_text}"));
            return Err(XyError::provider(XyProviderError {
                message,
                status: Some(status),
                code: None,
                error_type,
                retry_after,
            }));
        }

        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "anthropic-messages",
            &self.model,
            options.obs_parent,
            &crate::provider::obs_for_llm_request(&options.obs_session, &self.base_url),
        );
        if let Some(t) = &trace {
            t.capture_request_input(&body.to_string());
        }

        if stream {
            Ok(Box::pin(anthropic_stream(response, trace)))
        } else {
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
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        use futures::StreamExt;
        use eventsource_stream::Eventsource;

        let byte_stream = response.bytes_stream();
        let mut event_stream = byte_stream.eventsource();

        let mut tool_accumulators: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut thinking_accumulators: HashMap<usize, (String, Option<String>)> = HashMap::new();
        let mut current_block_index: Option<usize> = None;
        let mut usage_fields: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut done_emitted = false;

        'sse: loop {
            let event_result = tokio::time::timeout(
                crate::provider::native::wait_bounds::SSE_IDLE,
                event_stream.next(),
            )
            .await
            .map_err(|_| {
                XyError::Provider(anyhow::anyhow!(
                    "provider SSE idle: no bytes within {}s",
                    crate::provider::native::wait_bounds::SSE_IDLE.as_secs()
                ))
            })?;
            let Some(event_result) = event_result else {
                break 'sse;
            };
            if event_result.is_err() {
                continue 'sse;
            }
            let event = event_result.expect("parse-error arm continued above");
            let event_type = event.event.as_str();
            let data: Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => continue 'sse,
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
                            tool_accumulators.insert(index, (id.clone(), name.clone(), String::new()));
                            let chunk = XyChunk::ToolCallStart { id, name };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        } else if block_type == "thinking" {
                            let thinking = content_block
                                .get("thinking")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let signature = content_block
                                .get("signature")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(str::to_string);
                            thinking_accumulators.insert(index, (thinking, signature));
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
                            "thinking_delta" | "thinking" => {
                                if let Some(thinking) = delta.get("thinking").and_then(|v| v.as_str()) {
                                    match thinking_accumulators.get_mut(&index) {
                                        Some(acc) => acc.0.push_str(thinking),
                                        None => {
                                            thinking_accumulators
                                                .insert(index, (thinking.to_string(), None));
                                        }
                                    }
                                    let chunk = XyChunk::ThinkingDelta(thinking.to_string());
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            "signature_delta" => {
                                if let Some(signature) = delta.get("signature").and_then(|v| v.as_str()) {
                                    let acc = thinking_accumulators
                                        .entry(index)
                                        .or_insert_with(|| (String::new(), None));
                                    match &mut acc.1 {
                                        Some(existing) => existing.push_str(signature),
                                        None => acc.1 = Some(signature.to_string()),
                                    }
                                }
                            }
                            "input_json_delta" => {
                                if let (Some(partial_json), Some(acc)) = (
                                    delta.get("partial_json").and_then(|v| v.as_str()),
                                    tool_accumulators.get_mut(&index),
                                ) {
                                    acc.2.push_str(partial_json);
                                    let chunk = XyChunk::ToolCallDelta {
                                        id: acc.0.clone(),
                                        name: acc.1.clone(),
                                        args_delta: partial_json.to_string(),
                                        args: crate::dto::parse_streaming_json(&acc.2),
                                    };
                                    if let Some(t) = &trace {
                                        t.emit_mapped_chunk(&chunk);
                                    }
                                    yield chunk;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                "content_block_stop" => {
                    let index = data
                        .get("index")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize)
                        .or(current_block_index);
                    if let Some(index) = index {
                        if let Some((thinking, signature)) = thinking_accumulators.remove(&index)
                            && (!thinking.is_empty() || signature.is_some())
                        {
                            let chunk = XyChunk::ThinkingEnd {
                                thinking,
                                thinking_signature: signature,
                            };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }
                        if let Some((id, name, args_str)) = tool_accumulators.remove(&index) {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
                            let chunk = XyChunk::ToolCallEnd { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }
                    }
                    current_block_index = None;
                }

                "message_delta" => {
                    let stop_reason = data
                        .get("delta")
                        .and_then(|d| d.get("stop_reason"))
                        .and_then(|v| v.as_str());

                    if let Some(u) = data.get("usage") {
                        merge_usage_fields(&mut usage_fields, u);
                    }

                    if stop_reason.is_some() {
                        let mut thinking_sorted: Vec<_> = thinking_accumulators.drain().collect();
                        thinking_sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (thinking, signature)) in thinking_sorted {
                            if thinking.is_empty() && signature.is_none() {
                                continue;
                            }
                            let chunk = XyChunk::ThinkingEnd {
                                thinking,
                                thinking_signature: signature,
                            };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }

                        let mut sorted: Vec<_> = tool_accumulators.drain().collect();
                        sorted.sort_by_key(|(idx, _)| *idx);
                        for (_, (id, name, args_str)) in sorted {
                            let args: Value = serde_json::from_str(&args_str)
                                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
                            let chunk = XyChunk::ToolCallEnd { name, args, id };
                            if let Some(t) = &trace {
                                t.emit_mapped_chunk(&chunk);
                            }
                            yield chunk;
                        }

                        let finish = map_anthropic_stop_reason(stop_reason);
                        let usage = anthropic_usage_from_fields(&usage_fields);
                        let chunk = XyChunk::Done {
                            finish_reason: finish,
                            usage,
                        };
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                        done_emitted = true;
                    }
                }

                "message_start" => {
                    if let Some(msg) = data.get("message")
                        && let Some(u) = msg.get("usage") {
                            merge_usage_fields(&mut usage_fields, u);
                        }
                }
                "message_stop" => {
                    if !done_emitted {
                        let chunk = XyChunk::Done {
                            finish_reason: XyStopReason::Stop,
                            usage: anthropic_usage_from_fields(&usage_fields),
                        };
                        if let Some(t) = &trace {
                            t.emit_mapped_chunk(&chunk);
                        }
                        yield chunk;
                        done_emitted = true;
                    }
                }
                "ping" => {}
                "error" => {
                    let error = data.get("error");
                    let kind = error
                        .and_then(|e| e.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("api_error");
                    let message = error
                        .and_then(|e| e.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Anthropic stream error");
                    Err(XyError::provider(XyProviderError {
                        message: format!("Anthropic stream error: {message}"),
                        status: None,
                        code: None,
                        error_type: Some(kind.to_string()),
                        retry_after: None,
                    }))?;
                }
                _ => {}
            }
        }

        if !done_emitted {
            Err(XyError::Provider(anyhow::anyhow!(
                "Anthropic stream ended without terminal event"
            )))?;
        }
    })
}

fn merge_usage_fields(into: &mut serde_json::Map<String, Value>, usage: &Value) {
    let Some(obj) = usage.as_object() else {
        return;
    };
    for (key, value) in obj {
        into.insert(key.clone(), value.clone());
    }
}

fn anthropic_usage_from_fields(fields: &serde_json::Map<String, Value>) -> Option<XyUsage> {
    if fields.is_empty() {
        return None;
    }
    let usage = crate::usage::from_anthropic_usage(&Value::Object(fields.clone()));
    (usage.total_tokens > 0).then_some(usage)
}

fn map_anthropic_stop_reason(stop_reason: Option<&str>) -> XyStopReason {
    match stop_reason {
        Some("max_tokens" | "model_context_window_exceeded") => XyStopReason::MaxTokens,
        Some("tool_use") => XyStopReason::ToolUse,
        Some("refusal" | "sensitive") => XyStopReason::Error,
        Some("pause_turn") => XyStopReason::Stop,
        _ => XyStopReason::Stop,
    }
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
                        chunks.push(XyChunk::ThinkingEnd {
                            thinking: thinking.to_string(),
                            thinking_signature: block
                                .get("signature")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(str::to_string),
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
                    chunks.push(XyChunk::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    chunks.push(XyChunk::ToolCallEnd { name, args, id });
                }
                _ => {}
            }
        }
    }

    let finish = map_anthropic_stop_reason(json.get("stop_reason").and_then(|v| v.as_str()));

    let usage = json.get("usage").and_then(|u| {
        let mapped = crate::usage::from_anthropic_usage(u);
        (mapped.total_tokens > 0).then_some(mapped)
    });

    chunks.push(XyChunk::Done {
        finish_reason: finish,
        usage,
    });

    chunks
}

pub fn convert_agent_messages_for_anthropic(messages: &[XyLlmMessage]) -> Vec<Value> {
    let mut msgs: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            XyLlmMessage::UserMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "user",
                        "content": blocks,
                    }));
                }
            }
            XyLlmMessage::AssistantMessage { content, .. } => {
                let blocks = agent_parts_to_anthropic_blocks(content);
                if !blocks.is_empty() {
                    msgs.push(serde_json::json!({
                        "role": "assistant",
                        "content": blocks,
                    }));
                }
            }
            XyLlmMessage::ToolResultMessage {
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
        }
    }

    merge_consecutive_messages(&mut msgs);
    msgs
}

fn agent_parts_to_anthropic_blocks(parts: &[AgentPart]) -> Vec<Value> {
    parts
        .iter()
        .map(|part| match part {
            AgentPart::Text { text } => serde_json::json!({
                "type": "text",
                "text": text,
            }),
            AgentPart::Thinking {
                thinking,
                redacted,
                thinking_signature,
            } => match thinking_signature.as_deref().filter(|s| !s.is_empty()) {
                Some(signature) if *redacted => serde_json::json!({
                    "type": "redacted_thinking",
                    "data": signature,
                }),
                Some(signature) => serde_json::json!({
                    "type": "thinking",
                    "thinking": thinking,
                    "signature": signature,
                }),
                None => {
                    log::debug!(
                        target: "xylitol_official_plugin_provider_anthropic::messages",
                        "replay: thinking without signature degraded to text block"
                    );
                    serde_json::json!({
                        "type": "text",
                        "text": thinking,
                    })
                }
            },
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
                "name": crate::provider::tool_wire::to_wire_tool_name(name),
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

fn extract_error_type(body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    json.get("error")?.get("type")?.as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AgentPart, XyLlmMessage};

    #[test]
    fn convert_user_message() {
        let msgs = vec![XyLlmMessage::user("hello")];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn convert_assistant_with_tool_call() {
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::text("Let me check"),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "/tmp"}),
                },
            ],
            stop_reason: Some(crate::dto::XyStopReason::ToolUse),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["id"], "call-1");
    }

    #[test]
    fn convert_tool_result() {
        let msgs = vec![XyLlmMessage::tool_result(
            "call-1",
            "",
            vec![AgentPart::text("result here")],
            false,
        )];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        let content = msgs[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_result");
        assert_eq!(content[0]["tool_use_id"], "call-1");
    }

    #[test]
    fn convert_projected_env_as_user() {
        let msgs = vec![XyLlmMessage::user("$ ls\noutput")];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[tokio::test]
    async fn anthropic_body_omits_thinking_when_off() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
                Ok(())
            }

            async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let _ = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                false,
                crate::thinking::XyGenerateOptions::default(),
            )
            .await
            .expect("generate off");

        let body = captured.lock().unwrap().clone().expect("body");
        assert!(
            body.get("thinking").is_none(),
            "off must omit thinking block, got {body}"
        );
    }

    #[tokio::test]
    async fn anthropic_body_max_tokens_follows_options_budget() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
                Ok(())
            }

            async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let _ = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                false,
                crate::thinking::XyGenerateOptions {
                    max_output_tokens: Some(819),
                    ..Default::default()
                },
            )
            .await
            .expect("generate with budget");

        let body = captured.lock().unwrap().clone().expect("body");
        assert_eq!(
            body["max_tokens"], 819,
            "options budget must override: {body}"
        );
    }

    #[tokio::test]
    async fn anthropic_body_includes_budget_for_medium() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
                Ok(())
            }

            async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let opts = crate::thinking::XyGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let _ = adapter
            .generate_stream(vec![XyLlmMessage::user("hi")], &[], false, opts)
            .await
            .expect("generate medium");

        let body = captured.lock().unwrap().clone().expect("body");
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], 8192);
    }

    #[tokio::test]
    async fn freeform_thinking_without_map_fails_before_request() {
        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some("http://127.0.0.1:1".into()),
            None,
        );
        let result = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                false,
                crate::thinking::XyGenerateOptions {
                    thinking_level: "vendor-max".into(),
                    ..Default::default()
                },
            )
            .await;
        let Err(err) = result else {
            panic!("freeform Anthropic level must fail before request");
        };
        assert!(
            err.to_string()
                .contains("requires a numeric or known-level"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn deepseek_compat_omits_budget_tokens() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
                Ok(())
            }

            async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::with_wire_policy(
            "sk-test".into(),
            "deepseek-v4-flash".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
            crate::wire_policy::WirePolicy::for_compat(crate::wire_policy::Compat::Deepseek),
        );
        let opts = crate::thinking::XyGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let _ = adapter
            .generate_stream(vec![XyLlmMessage::user("hi")], &[], false, opts)
            .await
            .expect("generate medium deepseek");

        let body = captured.lock().unwrap().clone().expect("body");
        assert_eq!(body["thinking"]["type"], "enabled");
        assert!(
            body["thinking"].get("budget_tokens").is_none(),
            "DeepSeek Anthropic dialect must omit budget_tokens, got {body}"
        );
    }

    #[tokio::test]
    async fn anthropic_mcp_tool_names_are_provider_safe() {
        use std::sync::Arc;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use crate::hooks::{HeaderBag, HttpHooks};

        struct CaptureHooks {
            body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        }

        #[async_trait]
        impl HttpHooks for CaptureHooks {
            async fn before_headers(&self, _headers: &mut HeaderBag) -> Result<(), XyError> {
                Ok(())
            }

            async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
                *self.body.lock().unwrap() = Some(body.clone());
                Ok(())
            }

            async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
        }

        let captured = Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CaptureHooks {
                body: captured.clone(),
            })),
        );
        let tools = [
            crate::dto::XyToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            crate::dto::XyToolSchema {
                name: "mcp__context7__resolve-library-id".into(),
                description: "docs".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            crate::dto::XyToolSchema {
                name: "mcp:legacy:colon".into(),
                description: "legacy".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let _ = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &tools,
                false,
                Default::default(),
            )
            .await
            .expect("generate with tools");

        let body = captured.lock().unwrap().clone().expect("body");
        let arr = body["tools"].as_array().expect("tools");
        assert_eq!(arr.len(), 3);
        for (i, t) in arr.iter().enumerate() {
            let name = t["name"].as_str().unwrap_or("");
            assert!(
                crate::provider::tool_wire::is_provider_safe_tool_name(name),
                "tools[{i}].name={name:?}"
            );
            assert!(!name.contains(':'), "colon on wire: {name}");
            assert!(!name.contains('.'), "dot on wire: {name}");
        }
        assert_eq!(arr[2]["name"].as_str(), Some("mcp__legacy__colon"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sse_idle_bound_fires_classifiable_error() {
        use futures::StreamExt;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let mut buf = vec![0u8; 16 * 1024];
            let _ = sock.read(&mut buf).await;
            let head =
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
            if sock.write_all(head).await.is_err() {
                return;
            }
            let first = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"model\":\"idle-test\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n";
            if sock.write_all(first.as_bytes()).await.is_err() {
                return;
            }
            let _ = sock.flush().await;
            let () = futures::future::pending().await;
        });

        let adapter = AnthropicMessagesAdapter::new(
            "k".into(),
            "idle-test".into(),
            Some(format!("http://{addr}")),
            None,
        );
        let mut stream = adapter
            .generate_stream(vec![], &[], true, Default::default())
            .await
            .expect("stream established");
        tokio::time::pause();
        let mut idle_err = None;
        while let Some(item) = stream.next().await {
            if let Err(e) = item {
                idle_err = Some(e);
                break;
            }
        }
        let e = idle_err.expect("SSE idle bound must fire a timeout error");
        assert!(
            e.to_string().contains("provider SSE idle"),
            "idle timeout must be classifiable by message: {e}"
        );
    }

    #[tokio::test]
    async fn stream_thinking_signature_delta_emits_thinking_end() {
        use futures::StreamExt;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let sse = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan \"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"ahead\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig-abc\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        );

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse.to_string(), "text/event-stream"),
            )
            .mount(&server)
            .await;

        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            None,
        );
        let mut stream = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                true,
                Default::default(),
            )
            .await
            .expect("stream established");

        let mut chunks = Vec::new();
        while let Some(item) = stream.next().await {
            chunks.push(item.expect("chunk"));
        }
        let thinking_end = chunks
            .iter()
            .find_map(|c| match c {
                XyChunk::ThinkingEnd {
                    thinking,
                    thinking_signature,
                } => Some((thinking.clone(), thinking_signature.clone())),
                _ => None,
            })
            .expect("stream must emit ThinkingEnd for a thinking block");
        assert_eq!(thinking_end.0, "plan ahead");
        assert_eq!(thinking_end.1.as_deref(), Some("sig-abc"));
        let thinking_deltas: Vec<_> = chunks
            .iter()
            .filter_map(|c| match c {
                XyChunk::ThinkingDelta(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(thinking_deltas, vec!["plan ", "ahead"]);
    }

    #[test]
    fn non_stream_thinking_block_carries_signature() {
        let json = serde_json::json!({
            "content": [
                {"type": "thinking", "thinking": "weigh options", "signature": "sig-xyz"},
                {"type": "text", "text": "done"}
            ],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 3, "output_tokens": 4}
        });
        let chunks = parse_anthropic_response(&json);
        let thinking_end = chunks
            .iter()
            .find_map(|c| match c {
                XyChunk::ThinkingEnd {
                    thinking,
                    thinking_signature,
                } => Some((thinking.clone(), thinking_signature.clone())),
                _ => None,
            })
            .expect("non-stream thinking block must emit ThinkingEnd");
        assert_eq!(thinking_end.0, "weigh options");
        assert_eq!(thinking_end.1.as_deref(), Some("sig-xyz"));
    }

    fn assistant_with_thinking(thinking: AgentPart) -> XyLlmMessage {
        XyLlmMessage::AssistantMessage {
            content: vec![
                thinking,
                AgentPart::text("answer"),
                AgentPart::ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "/tmp"}),
                },
            ],
            stop_reason: Some(crate::dto::XyStopReason::ToolUse),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn replay_thinking_with_signature_uses_thinking_block_and_keeps_order() {
        let msgs = vec![assistant_with_thinking(AgentPart::Thinking {
            thinking: "plan".into(),
            redacted: false,
            thinking_signature: Some("sig-1".into()),
        })];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        let content = msgs[0]["content"].as_array().expect("content");
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "plan");
        assert_eq!(content[0]["signature"], "sig-1");
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[2]["type"], "tool_use");
    }

    #[test]
    fn replay_redacted_thinking_uses_redacted_block() {
        let msgs = vec![assistant_with_thinking(AgentPart::Thinking {
            thinking: String::new(),
            redacted: true,
            thinking_signature: Some("opaque-data".into()),
        })];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        let content = msgs[0]["content"].as_array().expect("content");
        assert_eq!(content[0]["type"], "redacted_thinking");
        assert_eq!(content[0]["data"], "opaque-data");
        assert!(content[0].get("thinking").is_none());
    }

    async fn run_sse(sse: &str) -> Vec<Result<XyChunk, XyError>> {
        use futures::StreamExt;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse.to_string(), "text/event-stream"),
            )
            .mount(&server)
            .await;
        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            None,
        );
        let mut stream = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                true,
                Default::default(),
            )
            .await
            .expect("stream established");
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        items
    }

    fn sse_terminal(stop_reason: &str) -> String {
        concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"REASON\"},\"usage\":{\"output_tokens\":3}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        )
        .replace("REASON", stop_reason)
    }

    fn done_of(items: &[Result<XyChunk, XyError>]) -> &XyChunk {
        items
            .iter()
            .filter_map(|item| item.as_ref().ok())
            .find(|chunk| matches!(chunk, XyChunk::Done { .. }))
            .expect("Done chunk")
    }

    #[tokio::test]
    async fn stream_error_event_terminates_with_provider_error() {
        let sse = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n",
        );
        let items = run_sse(sse).await;
        let err = items
            .last()
            .expect("stream items")
            .as_ref()
            .expect_err("error event must surface Err");
        assert!(err.to_string().contains("overloaded_error"), "{err}");
        let detail = err
            .provider_detail()
            .expect("stream error must carry structured detail");
        assert_eq!(detail.status, None);
        assert_eq!(detail.error_type.as_deref(), Some("overloaded_error"));
        assert!(
            !items
                .iter()
                .any(|item| matches!(item, Ok(XyChunk::Done { .. }))),
            "error event must not be followed by Done"
        );
    }

    #[tokio::test]
    async fn http_error_carries_status_type_and_retry_after() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(529)
                    .insert_header("retry-after", "7")
                    .set_body_json(serde_json::json!({
                        "type": "error",
                        "error": {"type": "overloaded_error", "message": "Overloaded"}
                    })),
            )
            .mount(&server)
            .await;
        let adapter = AnthropicMessagesAdapter::new(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            None,
        );
        let Err(err) = adapter
            .generate_stream(
                vec![XyLlmMessage::user("hi")],
                &[],
                false,
                crate::thinking::XyGenerateOptions::default(),
            )
            .await
        else {
            panic!("529 must surface Err");
        };
        let detail = err.provider_detail().expect("structured detail");
        assert_eq!(detail.status, Some(529));
        assert_eq!(detail.error_type.as_deref(), Some("overloaded_error"));
        assert_eq!(detail.message, "Overloaded");
        assert_eq!(detail.retry_after, Some(std::time::Duration::from_secs(7)));
    }

    #[tokio::test]
    async fn stream_message_stop_without_delta_emits_done() {
        let sse = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        );
        let items = run_sse(sse).await;
        match done_of(&items) {
            XyChunk::Done {
                finish_reason,
                usage,
            } => {
                assert_eq!(*finish_reason, XyStopReason::Stop);
                assert_eq!(usage.expect("usage from message_start").input, 5);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stream_eof_without_terminal_event_errors() {
        let sse = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n",
        );
        let items = run_sse(sse).await;
        let err = items
            .last()
            .expect("stream items")
            .as_ref()
            .expect_err("truncated stream must surface Err");
        assert!(err.to_string().contains("without terminal event"), "{err}");
    }

    #[tokio::test]
    async fn stream_stop_reason_mapping() {
        let cases = [
            ("model_context_window_exceeded", XyStopReason::MaxTokens),
            ("max_tokens", XyStopReason::MaxTokens),
            ("tool_use", XyStopReason::ToolUse),
            ("refusal", XyStopReason::Error),
            ("pause_turn", XyStopReason::Stop),
            ("end_turn", XyStopReason::Stop),
        ];
        for (reason, expected) in cases {
            let items = run_sse(&sse_terminal(reason)).await;
            match done_of(&items) {
                XyChunk::Done {
                    finish_reason,
                    usage: _,
                } => assert_eq!(*finish_reason, expected, "stop_reason={reason}"),
                other => panic!("expected Done for {reason}, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn stream_usage_reports_cache_fields() {
        let sse = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"role\":\"assistant\",\"content\":[],\"usage\":{\"input_tokens\":100,\"output_tokens\":0,\"cache_read_input_tokens\":40,\"cache_creation_input_tokens\":10,\"cache_creation\":{\"ephemeral_5m_input_tokens\":6,\"ephemeral_1h_input_tokens\":4}}}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":20}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        );
        let items = run_sse(sse).await;
        match done_of(&items) {
            XyChunk::Done { usage, .. } => {
                let usage = usage.as_ref().expect("usage");
                assert_eq!(usage.input, 100);
                assert_eq!(usage.output, 20);
                assert_eq!(usage.cache_read, 40);
                assert_eq!(usage.cache_write, 10);
                assert_eq!(usage.cache_write_1h, 4);
                assert_eq!(usage.total_tokens, 170);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn non_stream_usage_reports_cache_fields() {
        let json = serde_json::json!({
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 20,
                "cache_read_input_tokens": 40,
                "cache_creation_input_tokens": 10
            }
        });
        let chunks = parse_anthropic_response(&json);
        let usage = chunks
            .iter()
            .find_map(|c| match c {
                XyChunk::Done { usage, .. } => usage.as_ref(),
                _ => None,
            })
            .expect("usage");
        assert_eq!(usage.cache_read, 40);
        assert_eq!(usage.cache_write, 10);
        assert_eq!(usage.total_tokens, 170);
    }

    #[test]
    fn non_stream_context_window_stop_maps_to_max_tokens() {
        let json = serde_json::json!({
            "content": [{"type": "text", "text": "ok"}],
            "stop_reason": "model_context_window_exceeded",
            "usage": {"input_tokens": 1, "output_tokens": 1}
        });
        let chunks = parse_anthropic_response(&json);
        match chunks.last().expect("done") {
            XyChunk::Done { finish_reason, .. } => {
                assert_eq!(*finish_reason, XyStopReason::MaxTokens);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn replay_thinking_without_signature_degrades_to_text() {
        let msgs = vec![assistant_with_thinking(AgentPart::Thinking {
            thinking: "old plan".into(),
            redacted: false,
            thinking_signature: None,
        })];
        let msgs = convert_agent_messages_for_anthropic(&msgs);
        let content = msgs[0]["content"].as_array().expect("content");
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "old plan");
        assert!(content[0].get("signature").is_none());
    }

    struct CacheCaptureHooks {
        body: std::sync::Arc<std::sync::Mutex<Option<Value>>>,
        headers: std::sync::Arc<std::sync::Mutex<Option<HeaderBag>>>,
    }

    #[async_trait]
    impl HttpHooks for CacheCaptureHooks {
        async fn before_headers(&self, headers: &mut HeaderBag) -> Result<(), XyError> {
            *self.headers.lock().unwrap() = Some(headers.clone());
            Ok(())
        }

        async fn before_request(&self, _model: &str, body: &mut Value) -> Result<(), XyError> {
            *self.body.lock().unwrap() = Some(body.clone());
            Ok(())
        }

        async fn after_response(&self, _status: u16, _headers: &HeaderBag) {}
    }

    async fn capture_anthropic_body(
        wire_policy: crate::wire_policy::WirePolicy,
        prompt_cache: Option<XyPromptCacheConfig>,
    ) -> (Value, HeaderBag) {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let body = std::sync::Arc::new(std::sync::Mutex::new(None));
        let headers = std::sync::Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "hi"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;
        let adapter = AnthropicMessagesAdapter::with_wire_policy(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CacheCaptureHooks {
                body: body.clone(),
                headers: headers.clone(),
            })),
            wire_policy,
        )
        .with_prompt_cache(prompt_cache);
        let tools = [
            crate::dto::XyToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            crate::dto::XyToolSchema {
                name: "bash".into(),
                description: "b".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let opts = crate::thinking::XyGenerateOptions {
            system_prompt: Some("system prompt".into()),
            ..Default::default()
        };
        let _ = adapter
            .generate_stream(vec![XyLlmMessage::user("hi")], &tools, false, opts)
            .await
            .expect("generate");
        let body = body.lock().unwrap().clone().expect("body");
        let headers = headers.lock().unwrap().clone().expect("headers");
        (body, headers)
    }

    fn cache_control_count(body: &Value) -> usize {
        body.to_string().matches("\"cache_control\"").count()
    }

    #[tokio::test]
    async fn prompt_cache_attaches_three_breakpoints_for_generic() {
        let (body, _) =
            capture_anthropic_body(crate::wire_policy::WirePolicy::default(), None).await;
        let system = body["system"].as_array().expect("system blocks");
        assert_eq!(system.len(), 1);
        assert_eq!(system[0]["type"], "text");
        assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
        let tools = body["tools"].as_array().expect("tools");
        assert!(
            tools[0].get("cache_control").is_none(),
            "only the last tool carries a breakpoint"
        );
        assert_eq!(tools.last().unwrap()["cache_control"]["type"], "ephemeral");
        let messages = body["messages"].as_array().expect("messages");
        let last_block = messages.last().unwrap()["content"]
            .as_array()
            .unwrap()
            .last()
            .unwrap();
        assert_eq!(last_block["cache_control"]["type"], "ephemeral");
        assert_eq!(cache_control_count(&body), 3);
    }

    #[tokio::test]
    async fn prompt_cache_disabled_for_deepseek_compat() {
        let policy =
            crate::wire_policy::WirePolicy::for_compat(crate::wire_policy::Compat::Deepseek);
        let (body, _) = capture_anthropic_body(policy, None).await;
        assert_eq!(cache_control_count(&body), 0);
        assert!(
            body["system"].is_string(),
            "uncached system must stay a plain string"
        );
    }

    #[tokio::test]
    async fn prompt_cache_explicit_enable_wins_over_deepseek_default() {
        let policy =
            crate::wire_policy::WirePolicy::for_compat(crate::wire_policy::Compat::Deepseek);
        let config = XyPromptCacheConfig {
            enabled: Some(true),
            ttl: None,
            retention: None,
        };
        let (body, _) = capture_anthropic_body(policy, Some(config)).await;
        assert_eq!(cache_control_count(&body), 3);
    }

    #[tokio::test]
    async fn prompt_cache_explicit_disable_wins_over_generic_default() {
        let config = XyPromptCacheConfig {
            enabled: Some(false),
            ttl: None,
            retention: None,
        };
        let (body, _) =
            capture_anthropic_body(crate::wire_policy::WirePolicy::default(), Some(config)).await;
        assert_eq!(cache_control_count(&body), 0);
    }

    #[tokio::test]
    async fn prompt_cache_one_hour_ttl_sets_beta_header() {
        let config = XyPromptCacheConfig {
            enabled: Some(true),
            ttl: Some(XyPromptCacheTtl::OneHour),
            retention: None,
        };
        let (body, headers) =
            capture_anthropic_body(crate::wire_policy::WirePolicy::default(), Some(config)).await;
        assert_eq!(body["system"][0]["cache_control"]["ttl"], "1h");
        assert_eq!(
            headers.get("anthropic-beta").and_then(|v| v.as_str()),
            Some("extended-cache-ttl-2025-04-11")
        );
    }

    #[tokio::test]
    async fn structured_output_maps_to_output_config_format() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let body_slot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let headers_slot = std::sync::Arc::new(std::sync::Mutex::new(None));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "content": [{"type": "text", "text": "{\"n\":1}"}],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 1}
            })))
            .mount(&server)
            .await;
        let adapter = AnthropicMessagesAdapter::with_wire_policy(
            "sk-test".into(),
            "claude-test".into(),
            Some(server.uri()),
            Some(Arc::new(CacheCaptureHooks {
                body: body_slot.clone(),
                headers: headers_slot.clone(),
            })),
            crate::wire_policy::WirePolicy::default(),
        );
        let opts = crate::thinking::XyGenerateOptions {
            structured_output: Some(xylitol_plugin_sdk::XyStructuredOutput {
                name: "answer".into(),
                schema: serde_json::json!({"type": "object", "properties": {"n": {"type": "number"}}}),
                strict: Some(true),
                description: None,
            }),
            ..Default::default()
        };
        let _ = adapter
            .generate_stream(vec![XyLlmMessage::user("hi")], &[], false, opts)
            .await
            .expect("generate");
        let body = body_slot.lock().unwrap().clone().expect("body");
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["output_config"]["format"]["schema"]["type"], "object");
        assert!(
            body["output_config"]["format"].get("name").is_none(),
            "Anthropic shape has no name field"
        );
    }

    #[tokio::test]
    async fn structured_output_absent_leaves_body_unchanged() {
        let (body, _) =
            capture_anthropic_body(crate::wire_policy::WirePolicy::default(), None).await;
        assert!(body.get("output_config").is_none());
    }
}
