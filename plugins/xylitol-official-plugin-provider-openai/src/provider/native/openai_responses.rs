use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::dto::{AgentPart, Diagnostic, XyChunk, XyStopReason};
use crate::hooks::HttpHooks;
use crate::provider::native::openai_client::{OpenAiClientFactory, normalize_openai_v1_base};
use crate::wire_policy::WirePolicy;

use xylitol_plugin_sdk::message::XyLlmMessage;
use xylitol_plugin_sdk::{XyError, XyGenerateOptions, XyModel, XyStream, XyToolSchema};

pub struct OpenAiResponsesAdapter {
    client: OpenAiClientFactory,
    model: String,
    wire_policy: WirePolicy,
}

impl OpenAiResponsesAdapter {
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
        let base = base_url.map(|b| normalize_openai_v1_base(&b));
        Self {
            client: OpenAiClientFactory::new(api_key, base, hooks),
            model,
            wire_policy,
        }
    }

    pub fn wire_policy(&self) -> WirePolicy {
        self.wire_policy
    }

    fn build_body(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: &crate::thinking::XyGenerateOptions,
    ) -> Value {
        super::assembler::ResponsesAssembler::new(self.wire_policy).assemble(
            &self.model,
            messages,
            tools,
            stream,
            options,
        )
    }

    fn map_err(err: async_openai::error::OpenAIError) -> XyError {
        XyError::Provider(anyhow::anyhow!("{}", format_responses_error(&err)))
    }
}

pub fn format_responses_error(err: &impl std::fmt::Display) -> String {
    let raw = err.to_string();
    match extract_embedded_provider_error_message(&raw) {
        Some(msg) => format!("OpenAI Responses: {msg}"),
        None => format!("OpenAI Responses: {raw}"),
    }
}

pub fn extract_embedded_provider_error_message(raw: &str) -> Option<String> {
    let json_src = if let Some(rest) = raw.split_once("content:").map(|(_, r)| r) {
        extract_balanced_json_object(rest)?
    } else {
        let idx = raw.find("{\"error\"")?;
        extract_balanced_json_object(&raw[idx..])?
    };
    let value: Value = serde_json::from_str(&json_src).ok()?;
    let err = value.get("error")?;
    let message = err.get("message")?.as_str()?.trim();
    if message.is_empty() {
        return None;
    }
    match err
        .get("type")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
    {
        Some(ty) => Some(format!("{message} ({ty})")),
        None => Some(message.to_string()),
    }
}

fn extract_balanced_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn assemble_responses_body_with_diagnostics(
    model: &str,
    messages: Vec<XyLlmMessage>,
    tools: &[XyToolSchema],
    stream: bool,
    options: &crate::thinking::XyGenerateOptions,
    wire_policy: &WirePolicy,
) -> (Value, Vec<Diagnostic>) {
    let (mut input_items, replay_diagnostics) =
        messages_to_responses_input_with_diagnostics(&messages);
    if !replay_diagnostics.is_empty() {
        log::debug!(
            target: "xylitol_official_plugin_provider_openai::responses",
            "Responses assemble: {} thinkingSignature omit diagnostic(s)",
            replay_diagnostics.len()
        );
    }
    prepend_system_prompt_item(
        &mut input_items,
        options.system_prompt.as_deref(),
        &options.thinking_level,
    );
    let mut body = serde_json::json!({
        "model": model,
        "input": input_items,
        "stream": stream,
        "store": false,
    });

    if let Some(max) = options.max_output_tokens {
        body["max_output_tokens"] = serde_json::json!(max);
    }

    if !tools.is_empty() {
        let tool_defs: Vec<Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "name": crate::provider::tool_wire::to_wire_tool_name(&t.name),
                    "description": t.description,
                    "parameters": t.parameters,
                    "strict": false,
                })
            })
            .collect();
        body["tools"] = Value::Array(tool_defs);
    }

    let resolved = crate::thinking::resolve_from_options(
        options,
        crate::thinking::ThinkingAdapterKind::OpenAi,
    );
    crate::thinking::apply_thinking_openai_responses(&mut body, &resolved);

    if matches!(resolved, crate::thinking::ResolvedThinking::OpenAiEffort(_))
        && wire_policy.allows_reasoning_encrypted_include()
    {
        body["include"] = serde_json::json!(["reasoning.encrypted_content"]);
    }

    if wire_policy.allows_prompt_cache_key()
        && let Some(session_id) = options
            .obs_session
            .session_id
            .as_deref()
            .filter(|s| !s.is_empty())
    {
        body["prompt_cache_key"] = serde_json::json!(session_id);
        if let Some(retention) = wire_policy.prompt_cache_retention() {
            body["prompt_cache_retention"] = serde_json::json!(retention.as_wire_str());
        }
    }

    apply_responses_wire_policy(&mut body, wire_policy);
    (body, replay_diagnostics)
}

pub(crate) fn apply_responses_wire_policy(body: &mut Value, wire_policy: &WirePolicy) {
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    if !wire_policy.allows_previous_response_id() {
        obj.remove("previous_response_id");
    }
    if !wire_policy.allows_prompt_cache_key() {
        obj.remove("prompt_cache_key");
    }
}

impl OpenAiResponsesAdapter {
    async fn generate_sse(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        options: crate::thinking::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "openai-responses",
            &self.model,
            options.obs_parent,
            &crate::provider::obs_for_llm_request(&options.obs_session, self.client.api_base()),
        );
        let body = self.build_body(messages, tools, true, &options);
        if let Some(t) = &trace {
            t.capture_request_input(&body.to_string());
        }
        let sdk_stream = self
            .client
            .bind(&options.obs_session)
            .responses()
            .create_stream_byot::<_, Value>(body)
            .await
            .map_err(Self::map_err)?;
        Ok(Box::pin(responses_sdk_stream(
            sdk_stream,
            trace,
            self.wire_policy,
        )))
    }

    async fn generate_json(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[XyToolSchema],
        options: crate::thinking::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "openai-responses",
            &self.model,
            options.obs_parent,
            &crate::provider::obs_for_llm_request(&options.obs_session, self.client.api_base()),
        );
        let body = self.build_body(messages, tools, false, &options);
        if let Some(t) = &trace {
            t.capture_request_input(&body.to_string());
        }
        let json: Value = self
            .client
            .bind(&options.obs_session)
            .responses()
            .create_byot(body)
            .await
            .map_err(Self::map_err)?;
        if let Some(t) = &trace {
            t.emit_raw("response.json", &json.to_string());
        }
        let chunks = parse_responses_output(&json, self.wire_policy);
        if let Some(t) = &trace {
            for c in &chunks {
                t.emit_mapped_chunk(c);
            }
        }
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

#[async_trait]
impl XyModel for OpenAiResponsesAdapter {
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
        if stream {
            self.generate_sse(messages, tools, options).await
        } else {
            self.generate_json(messages, tools, options).await
        }
    }
}

fn responses_sdk_stream(
    mut sdk_stream: impl Stream<Item = Result<Value, async_openai::error::OpenAIError>>
    + Send
    + Unpin
    + 'static,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
    wire_policy: WirePolicy,
) -> Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>> {
    Box::pin(async_stream::try_stream! {
        let mut state = ResponsesStreamState {
            wire_policy,
            ..Default::default()
        };

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
            let data = item.map_err(|e| {
                XyError::Provider(anyhow::anyhow!("{}", format_responses_error(&e)))
            })?;

            let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(t) = &trace {
                let snippet = data.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                t.emit_raw(event_type, snippet);
            }

            let chunks = map_responses_sse_event(&data, &mut state);
            if let Some(err) = state.terminal_error.take() {
                Err(err)?;
            }
            for chunk in chunks {
                if let Some(t) = &trace {
                    t.emit_mapped_chunk(&chunk);
                }
                yield chunk;
            }
        }

        if !state.done_emitted {
            Err(XyError::Provider(anyhow::anyhow!(
                "OpenAI Responses stream ended without terminal event"
            )))?;
        }
    })
}

#[derive(Default)]
pub struct ResponsesStreamState {
    function_calls: HashMap<String, (String, String, bool)>,
    reasoning_items_by_id: HashMap<String, Value>,
    usage_input: u64,
    usage_output: u64,
    usage_value: Option<Value>,
    wire_policy: WirePolicy,
    done_emitted: bool,
    terminal_error: Option<XyError>,
}

pub fn map_responses_sse_event(data: &Value, state: &mut ResponsesStreamState) -> Vec<XyChunk> {
    let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match event_type {
        "response.reasoning_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![XyChunk::ThinkingDelta(d.to_string())])
            .unwrap_or_default(),
        "response.output_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![XyChunk::TextDelta(d.to_string())])
            .unwrap_or_default(),
        "response.output_item.added" => {
            let Some(item) = data.get("item") else {
                return Vec::new();
            };
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if item_type != "function_call" {
                return Vec::new();
            }
            let id = item
                .get("id")
                .or_else(|| item.get("call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                return Vec::new();
            }
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            state
                .function_calls
                .insert(id.clone(), (name.clone(), String::new(), true));
            vec![XyChunk::ToolCallStart { id, name }]
        }
        "response.function_call_arguments.delta" => {
            let (Some(item_id), Some(delta)) = (
                data.get("item_id").and_then(|v| v.as_str()),
                data.get("delta").and_then(|v| v.as_str()),
            ) else {
                return Vec::new();
            };
            let entry = state
                .function_calls
                .entry(item_id.to_string())
                .or_insert_with(|| (String::new(), String::new(), false));
            let mut out = Vec::new();
            if !entry.2 {
                entry.2 = true;
                out.push(XyChunk::ToolCallStart {
                    id: item_id.to_string(),
                    name: entry.0.clone(),
                });
            }
            entry.1.push_str(delta);
            let args = crate::dto::parse_streaming_json(&entry.1);
            out.push(XyChunk::ToolCallDelta {
                id: item_id.to_string(),
                name: entry.0.clone(),
                args_delta: delta.to_string(),
                args,
            });
            out
        }
        "response.output_item.done" => {
            let Some(item) = data.get("item") else {
                return Vec::new();
            };
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if item_type == "reasoning" {
                if let Some(id) = item.get("id").and_then(|v| v.as_str())
                    && !id.is_empty()
                {
                    state
                        .reasoning_items_by_id
                        .insert(id.to_string(), item.clone());
                }
                return vec![thinking_end_from_reasoning_item(item)];
            }
            if item_type != "function_call" {
                return Vec::new();
            }
            let id = item
                .get("id")
                .or_else(|| item.get("call_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name_from_item = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (name, args_str, started) = state
                .function_calls
                .remove(&id)
                .unwrap_or_else(|| (name_from_item.clone(), String::new(), false));
            let name = if name.is_empty() {
                name_from_item
            } else {
                name
            };
            let args_str = if args_str.is_empty() {
                item.get("arguments")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}")
                    .to_string()
            } else {
                args_str
            };
            let args: Value = serde_json::from_str(&args_str)
                .unwrap_or_else(|_| crate::dto::parse_streaming_json(&args_str));
            let mut out = Vec::new();
            if !started {
                out.push(XyChunk::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                });
            }
            out.push(XyChunk::ToolCallEnd { id, name, args });
            out
        }
        "response.usage" => {
            if let Some(usage) = data.get("usage") {
                state.usage_input = usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_input);
                state.usage_output = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(state.usage_output);
                state.usage_value = Some(usage.clone());
            }
            Vec::new()
        }
        "response.completed" => finish_responses_terminal(data, state, XyStopReason::Stop),
        "response.incomplete" => {
            let reason = data
                .pointer("/response/incomplete_details/reason")
                .and_then(|v| v.as_str());
            let finish = match reason {
                Some("content_filter") => XyStopReason::Error,
                _ => XyStopReason::MaxTokens,
            };
            finish_responses_terminal(data, state, finish)
        }
        "response.failed" => {
            let error = data.pointer("/response/error");
            let code = error
                .and_then(|e| e.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("server_error");
            let message = error
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("response failed");
            state.terminal_error = Some(XyError::Provider(anyhow::anyhow!(
                "OpenAI Responses failed ({code}): {message}"
            )));
            Vec::new()
        }
        "error" => {
            let code = data
                .get("code")
                .and_then(|v| v.as_str())
                .unwrap_or("server_error");
            let message = data
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("stream error");
            state.terminal_error = Some(XyError::Provider(anyhow::anyhow!(
                "OpenAI Responses stream error ({code}): {message}"
            )));
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn finish_responses_terminal(
    data: &Value,
    state: &mut ResponsesStreamState,
    finish_reason: XyStopReason,
) -> Vec<XyChunk> {
    if let Some(usage) = data
        .get("response")
        .and_then(|r| r.get("usage"))
        .or_else(|| data.get("usage"))
    {
        state.usage_input = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(state.usage_input);
        state.usage_output = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(state.usage_output);
        state.usage_value = Some(usage.clone());
    }

    let mut out = backfill_reasoning_encrypted_from_terminal(data, state);

    let usage = state
        .usage_value
        .as_ref()
        .map(|v| crate::usage::from_responses_usage_with_policy(v, state.wire_policy))
        .filter(|u| u.total_tokens > 0 || u.input + u.output > 0);
    let usage = usage.or_else(|| {
        let usage_total = state.usage_input + state.usage_output;
        if usage_total > 0 {
            let pcr = if state.wire_policy.expects_prompt_cache_usage() {
                crate::dto::PromptCacheRead::NotReported
            } else {
                crate::dto::PromptCacheRead::NotApplicable
            };
            Some(
                crate::dto::XyUsage {
                    input: state.usage_input,
                    output: state.usage_output,
                    total_tokens: usage_total,
                    ..Default::default()
                }
                .with_prompt_cache_read(pcr),
            )
        } else {
            None
        }
    });
    out.push(XyChunk::Done {
        finish_reason,
        usage,
    });
    state.done_emitted = true;
    out
}

fn backfill_reasoning_encrypted_from_terminal(
    data: &Value,
    state: &mut ResponsesStreamState,
) -> Vec<XyChunk> {
    let Some(output) = data
        .get("response")
        .and_then(|r| r.get("output"))
        .or_else(|| data.get("output"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for item in output {
        if item.get("type").and_then(|v| v.as_str()) != Some("reasoning") {
            continue;
        }
        let Some(enc) = item.get("encrypted_content").and_then(|v| v.as_str()) else {
            continue;
        };
        if enc.is_empty() {
            continue;
        }
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(stored) = state.reasoning_items_by_id.get_mut(id) else {
            continue;
        };
        let needs = match stored.get("encrypted_content").and_then(|v| v.as_str()) {
            None => true,
            Some(s) => s.is_empty(),
        };
        if !needs {
            continue;
        }
        if let Some(obj) = stored.as_object_mut() {
            obj.insert("encrypted_content".into(), Value::String(enc.to_string()));
        }
        out.push(thinking_end_from_reasoning_item(stored));
    }
    out
}

fn reasoning_item_display_text(item: &Value) -> String {
    let join_texts = |key: &str| -> String {
        item.get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            })
            .unwrap_or_default()
    };
    let summary = join_texts("summary");
    if !summary.is_empty() {
        return summary;
    }
    join_texts("content")
}

fn thinking_end_from_reasoning_item(item: &Value) -> XyChunk {
    XyChunk::ThinkingEnd {
        thinking: reasoning_item_display_text(item),
        thinking_signature: Some(item.to_string()),
    }
}

fn parse_responses_output(json: &Value, wire_policy: WirePolicy) -> Vec<XyChunk> {
    let mut chunks = Vec::new();

    if let Some(output) = json.get("output").and_then(|v| v.as_array()) {
        for item in output {
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match item_type {
                "reasoning" => {
                    chunks.push(thinking_end_from_reasoning_item(item));
                }
                "message" => {
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for block in content {
                            if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                                chunks.push(XyChunk::TextDelta(text.to_string()));
                            }
                        }
                    }
                }
                "function_call" => {
                    let id = item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let args = match item.get("arguments") {
                        Some(Value::String(s)) => crate::dto::parse_streaming_json(s),
                        Some(v) => v.clone(),
                        None => serde_json::json!({}),
                    };
                    chunks.push(XyChunk::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    chunks.push(XyChunk::ToolCallEnd { id, name, args });
                }
                _ => {}
            }
        }
    }

    let usage = json.get("usage").and_then(|u| {
        let mapped = crate::usage::from_responses_usage_with_policy(u, wire_policy);
        if mapped.total_tokens > 0 || mapped.input + mapped.output > 0 {
            Some(mapped)
        } else {
            None
        }
    });

    chunks.push(XyChunk::Done {
        finish_reason: XyStopReason::Stop,
        usage,
    });

    chunks
}

pub fn messages_to_responses_input(messages: &[XyLlmMessage]) -> Vec<Value> {
    messages_to_responses_input_with_diagnostics(messages).0
}

pub fn messages_to_responses_input_with_diagnostics(
    messages: &[XyLlmMessage],
) -> (Vec<Value>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let items = convert_messages_to_input_items(messages, &mut diagnostics);
    (items, diagnostics)
}

pub fn messages_to_responses_input_with_options(
    messages: &[XyLlmMessage],
    options: &crate::thinking::XyGenerateOptions,
) -> Vec<Value> {
    let (mut items, _) = messages_to_responses_input_with_diagnostics(messages);
    prepend_system_prompt_item(
        &mut items,
        options.system_prompt.as_deref(),
        &options.thinking_level,
    );
    items
}

fn prepend_system_prompt_item(
    items: &mut Vec<Value>,
    system_prompt: Option<&str>,
    thinking_level: &str,
) {
    let Some(sp) = system_prompt.filter(|s| !s.is_empty()) else {
        return;
    };
    let role = if thinking_level != "off" {
        "developer"
    } else {
        "system"
    };
    items.insert(
        0,
        serde_json::json!({
            "role": role,
            "content": sp,
        }),
    );
}

fn convert_messages_to_input_items(
    messages: &[XyLlmMessage],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            XyLlmMessage::UserMessage { content, .. } => {
                let parts = user_content_parts(content);
                if !parts.is_empty() {
                    items.push(serde_json::json!({
                        "type": "message",
                        "role": "user",
                        "content": parts,
                    }));
                }
            }
            XyLlmMessage::AssistantMessage { content, .. } => {
                for part in content {
                    if let AgentPart::Thinking {
                        thinking_signature: Some(sig),
                        ..
                    } = part
                    {
                        match serde_json::from_str::<Value>(sig) {
                            Ok(item) => items.push(item),
                            Err(e) => {
                                let message = format!(
                                    "omit illegal thinkingSignature JSON ({e}); full-replay requires parseable signature"
                                );
                                log::warn!(
                                    target: "xylitol_official_plugin_provider_openai::responses",
                                    "{message}"
                                );
                                diagnostics.push(Diagnostic {
                                    message,
                                    source: Some("openai-responses".into()),
                                });
                            }
                        }
                    }
                }

                let text = collect_assistant_output_text(content);
                let tool_calls: Vec<Value> = content
                    .iter()
                    .filter_map(|p| match p {
                        AgentPart::ToolCall {
                            id,
                            name,
                            arguments,
                        } => Some(serde_json::json!({
                            "type": "function_call",
                            "id": id,
                            "call_id": id,
                            "name": crate::provider::tool_wire::to_wire_tool_name(name),
                            "arguments": arguments.to_string(),
                        })),
                        _ => None,
                    })
                    .collect();

                if !text.is_empty() || !tool_calls.is_empty() {
                    if !text.is_empty() {
                        items.push(serde_json::json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": text}],
                        }));
                    }
                    items.extend(tool_calls);
                }
            }
            XyLlmMessage::ToolResultMessage {
                tool_use_id,
                content,
                ..
            } => {
                items.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": tool_use_id,
                    "output": tool_output_value(content),
                }));
            }
        }
    }

    items
}

fn user_content_parts(content: &[AgentPart]) -> Vec<Value> {
    let mut parts = Vec::new();
    for part in content {
        match part {
            AgentPart::Text { text } if !text.is_empty() => {
                parts.push(serde_json::json!({
                    "type": "input_text",
                    "text": text,
                }));
            }
            AgentPart::Image(image) => {
                parts.push(serde_json::json!({
                    "type": "input_image",
                    "image_url": super::image_data_url(image),
                }));
            }
            _ => {}
        }
    }
    parts
}

fn tool_output_value(content: &[AgentPart]) -> Value {
    let text = collect_user_text_parts(content);
    let images: Vec<&crate::dto::ImageContent> = content
        .iter()
        .filter_map(|part| match part {
            AgentPart::Image(image) => Some(image),
            _ => None,
        })
        .collect();
    if images.is_empty() {
        return serde_json::json!(text);
    }
    let mut parts = vec![serde_json::json!({
        "type": "input_text",
        "text": if text.is_empty() { "[image]" } else { text.as_str() },
    })];
    for image in images {
        parts.push(serde_json::json!({
            "type": "input_image",
            "image_url": super::image_data_url(image),
        }));
    }
    serde_json::json!(parts)
}

fn collect_user_text_parts(parts: &[AgentPart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let Some(text) = part.as_text() {
            buf.push_str(text);
        }
    }
    buf
}

fn collect_assistant_output_text(parts: &[AgentPart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let AgentPart::Text { text } = part {
            buf.push_str(text);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AgentPart, XyLlmMessage};

    #[test]
    fn all_items_have_type_field() {
        let msgs = vec![
            XyLlmMessage::user("hello"),
            XyLlmMessage::assistant("hi there"),
        ];
        let items = messages_to_responses_input(&msgs);
        for item in &items {
            assert!(item.get("type").is_some(), "item missing `type`: {item}");
        }
    }

    #[test]
    fn tool_round_items_are_well_typed() {
        let msgs = vec![
            XyLlmMessage::user("list files"),
            XyLlmMessage::AssistantMessage {
                content: vec![
                    AgentPart::text("let me check"),
                    AgentPart::ToolCall {
                        id: "call-1".into(),
                        name: "ls".into(),
                        arguments: serde_json::json!({"path": "."}),
                    },
                ],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
            XyLlmMessage::tool_result("call-1", "ls", vec![AgentPart::text("file.txt")], false),
        ];
        let items = messages_to_responses_input(&msgs);
        let types: Vec<&str> = items
            .iter()
            .map(|i| i.get("type").and_then(|v| v.as_str()).unwrap_or("(none)"))
            .collect();
        assert_eq!(
            types,
            vec![
                "message",
                "message",
                "function_call",
                "function_call_output"
            ],
            "item types in order: {types:?}"
        );
        let output = items
            .iter()
            .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("function_call_output"))
            .unwrap();
        assert_eq!(output["call_id"], "call-1");
        assert_eq!(output["output"], "file.txt");
    }

    #[test]
    fn user_image_uses_input_image() {
        let msgs = vec![XyLlmMessage::UserMessage {
            content: vec![
                AgentPart::text("look"),
                AgentPart::image("image/png", "AAA"),
            ],
            timestamp: 0,
        }];
        let items = messages_to_responses_input(&msgs);
        let content = items[0]["content"].as_array().expect("content array");
        assert!(
            content.iter().any(|part| part["type"] == "input_image"),
            "user image must be projected: {content:?}"
        );
    }

    #[test]
    fn tool_image_uses_structured_output_array() {
        let msgs = vec![XyLlmMessage::tool_result(
            "call-1",
            "read",
            vec![AgentPart::text("see"), AgentPart::image("image/png", "AAA")],
            false,
        )];
        let items = messages_to_responses_input(&msgs);
        let output = items[0]["output"].as_array().expect("output array");
        assert!(
            output.iter().any(|part| part["type"] == "input_image"),
            "tool image must be projected into function_call_output: {output:?}"
        );
    }

    #[test]
    fn assistant_tool_only_round_emits_function_call_not_empty_message() {
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "c1".into(),
                name: "ls".into(),
                arguments: serde_json::json!({}),
            }],
            stop_reason: None,
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let items = messages_to_responses_input(&msgs);
        assert_eq!(
            items.len(),
            1,
            "only the function_call item, no empty message"
        );
        assert_eq!(items[0]["type"], "function_call");
    }

    #[test]
    fn partial_response_created_is_ignored_not_fatal() {
        let data: Value = serde_json::from_str(
            r#"{"type":"response.created","response":{"id":"resp_1","object":"response","status":"in_progress"}}"#,
        )
        .unwrap();
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert!(chunks.is_empty());
    }

    #[test]
    fn output_text_delta_maps_to_text_chunk() {
        let data = serde_json::json!({
            "type": "response.output_text.delta",
            "delta": "hello"
        });
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(&chunks[0], XyChunk::TextDelta(t) if t == "hello"));
    }

    #[test]
    fn completed_with_nested_usage_emits_done() {
        let data = serde_json::json!({
            "type": "response.completed",
            "response": {
                "usage": { "input_tokens": 3, "output_tokens": 5 }
            }
        });
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(&data, &mut state);
        assert_eq!(chunks.len(), 1);
        match &chunks[0] {
            XyChunk::Done { usage: Some(u), .. } => {
                assert_eq!(u.input, 3);
                assert_eq!(u.output, 5);
                assert_eq!(u.total_tokens, 8);
            }
            other => panic!("expected Done with usage, got {other:?}"),
        }
    }

    #[test]
    fn function_call_args_stream_before_done() {
        let mut state = ResponsesStreamState::default();
        let added = serde_json::json!({
            "type": "response.output_item.added",
            "item": { "type": "function_call", "id": "fc_1", "name": "ls", "arguments": "" }
        });
        let start = map_responses_sse_event(&added, &mut state);
        assert!(matches!(
            &start[..],
            [XyChunk::ToolCallStart { id, name }] if id == "fc_1" && name == "ls"
        ));

        let d1 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "{\"path\":"
        });
        let mid = map_responses_sse_event(&d1, &mut state);
        assert!(
            matches!(&mid[..], [XyChunk::ToolCallDelta { id, args_delta, .. }] if id == "fc_1" && args_delta == "{\"path\":"),
            "got {mid:?}"
        );

        let d2 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "\"/tmp\"}"
        });
        let mid2 = map_responses_sse_event(&d2, &mut state);
        assert!(matches!(&mid2[..], [XyChunk::ToolCallDelta { .. }]));

        let done = serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "function_call",
                "id": "fc_1",
                "name": "ls",
                "arguments": "{\"path\":\"/tmp\"}"
            }
        });
        let end = map_responses_sse_event(&done, &mut state);
        match &end[..] {
            [XyChunk::ToolCallEnd { id, name, args }] => {
                assert_eq!(id, "fc_1");
                assert_eq!(name, "ls");
                assert_eq!(args["path"], "/tmp");
            }
            other => panic!("expected ToolCallEnd only, got {other:?}"),
        }
    }

    #[test]
    fn new_adapter_carries_default_wire_policy() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let p = adapter.wire_policy();
        assert_eq!(p, crate::wire_policy::WirePolicy::default());
        assert!(p.expects_prompt_cache_usage());
        assert!(!p.allows_previous_response_id());
    }

    #[test]
    fn with_wire_policy_override_is_retained() {
        let policy = crate::wire_policy::WirePolicy {
            compat: crate::wire_policy::Compat::Generic,
            extra_policy: crate::wire_policy::ExtraPolicy {
                prompt_cache_usage: true,
                prompt_cache_key: false,
                prompt_cache_retention: None,
                previous_response_id: false,
            },
        };
        let adapter =
            OpenAiResponsesAdapter::with_wire_policy("sk".into(), "gpt".into(), None, None, policy);
        assert!(adapter.wire_policy().expects_prompt_cache_usage());
    }

    #[test]
    fn assemble_omits_wire_knobs_under_default_policy() {
        let (body, _) = assemble_responses_body_with_diagnostics(
            "m",
            vec![XyLlmMessage::user("hi")],
            &[],
            false,
            &crate::thinking::XyGenerateOptions::default(),
            &WirePolicy::default(),
        );
        assert!(body.get("previous_response_id").is_none());
        assert!(body.get("prompt_cache_key").is_none());
    }

    #[test]
    fn wire_policy_strips_denied_knobs() {
        let mut body = serde_json::json!({
            "model": "m",
            "previous_response_id": "resp_1",
            "prompt_cache_key": "ck",
        });
        let deny = WirePolicy {
            compat: crate::wire_policy::Compat::Generic,
            extra_policy: crate::wire_policy::ExtraPolicy {
                prompt_cache_usage: true,
                prompt_cache_key: false,
                prompt_cache_retention: None,
                previous_response_id: false,
            },
        };
        apply_responses_wire_policy(&mut body, &deny);
        assert!(body.get("previous_response_id").is_none());
        assert!(body.get("prompt_cache_key").is_none());
    }

    #[test]
    fn wire_policy_keeps_knobs_when_allowed() {
        let policy = WirePolicy {
            compat: crate::wire_policy::Compat::Generic,
            extra_policy: crate::wire_policy::ExtraPolicy {
                prompt_cache_usage: false,
                prompt_cache_key: true,
                prompt_cache_retention: None,
                previous_response_id: true,
            },
        };
        let mut body = serde_json::json!({
            "previous_response_id": "resp_1",
            "prompt_cache_key": "ck",
        });
        apply_responses_wire_policy(&mut body, &policy);
        assert_eq!(body["previous_response_id"], "resp_1");
        assert_eq!(body["prompt_cache_key"], "ck");
    }

    #[test]
    fn build_body_injects_reasoning_effort() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let opts = crate::thinking::XyGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let body = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &opts);
        assert_eq!(body["reasoning"]["effort"], "medium");
        assert_eq!(body["reasoning"]["summary"], "auto");
        assert_eq!(body["store"], false);
        assert_eq!(
            body["include"],
            serde_json::json!(["reasoning.encrypted_content"])
        );

        let off = crate::thinking::XyGenerateOptions::default();
        let body_off = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &off);
        assert!(body_off.get("reasoning").is_none());
        assert_eq!(body_off["store"], false);
        assert!(body_off.get("include").is_none());

        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let mapped = crate::thinking::XyGenerateOptions {
            thinking_level: "high".into(),
            level_map: map,
            thinking_budgets: None,
            system_prompt: None,
            obs_parent: None,
            obs_session: Default::default(),
            max_output_tokens: None,
        };
        let body_map = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &mapped);
        assert_eq!(body_map["reasoning"]["effort"], "max");
        assert_eq!(body_map["reasoning"]["summary"], "auto");
    }

    #[test]
    fn build_body_includes_max_output_tokens_when_configured() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let opts = crate::thinking::XyGenerateOptions {
            max_output_tokens: Some(1234),
            ..Default::default()
        };
        let body = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &opts);
        assert_eq!(body["max_output_tokens"], 1234);

        let none = crate::thinking::XyGenerateOptions::default();
        let body_none = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &none);
        assert!(body_none.get("max_output_tokens").is_none());
    }

    fn session_options() -> crate::thinking::XyGenerateOptions {
        crate::thinking::XyGenerateOptions {
            obs_session: xylitol_plugin_sdk::ObsSessionContext {
                session_id: Some("sess-1".into()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn build_body_includes_prompt_cache_key_from_session() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &[],
            false,
            &session_options(),
        );
        assert_eq!(body["prompt_cache_key"], "sess-1");
        assert!(body.get("prompt_cache_retention").is_none());
    }

    #[test]
    fn build_body_omits_prompt_cache_key_without_session() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &[],
            false,
            &crate::thinking::XyGenerateOptions::default(),
        );
        assert!(body.get("prompt_cache_key").is_none());
    }

    #[test]
    fn build_body_includes_prompt_cache_retention_when_configured() {
        let policy = crate::wire_policy::WirePolicy {
            compat: crate::wire_policy::Compat::Generic,
            extra_policy: crate::wire_policy::ExtraPolicy {
                prompt_cache_usage: true,
                prompt_cache_key: true,
                prompt_cache_retention: Some(
                    crate::wire_policy::XyPromptCacheRetention::TwentyFourHours,
                ),
                previous_response_id: false,
            },
        };
        let adapter =
            OpenAiResponsesAdapter::with_wire_policy("sk".into(), "gpt".into(), None, None, policy);
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &[],
            false,
            &session_options(),
        );
        assert_eq!(body["prompt_cache_key"], "sess-1");
        assert_eq!(body["prompt_cache_retention"], "24h");
    }

    #[test]
    fn build_body_omits_prompt_cache_key_for_deepseek() {
        let adapter = OpenAiResponsesAdapter::with_wire_policy(
            "sk".into(),
            "deepseek".into(),
            None,
            None,
            crate::wire_policy::WirePolicy::for_compat(crate::wire_policy::Compat::Deepseek),
        );
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &[],
            false,
            &session_options(),
        );
        assert!(body.get("prompt_cache_key").is_none());
        assert!(body.get("prompt_cache_retention").is_none());
    }

    #[test]
    fn deepseek_compat_omits_encrypted_include() {
        let adapter = OpenAiResponsesAdapter::with_wire_policy(
            "sk".into(),
            "deepseek-v4-flash".into(),
            None,
            None,
            crate::wire_policy::WirePolicy::for_compat(crate::wire_policy::Compat::Deepseek),
        );
        let opts = crate::thinking::XyGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let body = adapter.build_body(vec![XyLlmMessage::user("hi")], &[], false, &opts);
        assert_eq!(body["reasoning"]["effort"], "medium");
        assert_eq!(body["store"], false);
        assert!(
            body.get("include").is_none(),
            "deepseek Responses must not send include encrypted_content, got {body}"
        );
    }

    #[test]
    fn build_body_mcp_tool_names_are_provider_safe() {
        let adapter =
            OpenAiResponsesAdapter::new("sk".into(), "deepseek-v4-flash".into(), None, None);
        let tools = [
            XyToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            XyToolSchema {
                name: "mcp__context7__resolve-library-id".into(),
                description: "docs".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            XyToolSchema {
                name: "mcp__lspz__get_diagnostics".into(),
                description: "lsp".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &tools,
            false,
            &crate::thinking::XyGenerateOptions {
                thinking_level: "high".into(),
                ..Default::default()
            },
        );
        let arr = body["tools"].as_array().expect("tools array");
        assert_eq!(arr.len(), 3);
        for (i, t) in arr.iter().enumerate() {
            let name = t["name"].as_str().unwrap_or("");
            assert!(
                crate::provider::tool_wire::is_provider_safe_tool_name(name),
                "tools[{i}].name={name:?} must match ^[a-zA-Z0-9_-]+$"
            );
            assert!(
                !name.contains(':'),
                "tools[{i}].name must not contain colon: {name}"
            );
            assert!(
                !name.contains('.'),
                "tools[{i}].name must not contain dot: {name}"
            );
        }
    }

    #[test]
    fn build_body_tools_strict_false() {
        let adapter = OpenAiResponsesAdapter::new("sk".into(), "gpt".into(), None, None);
        let tools = [XyToolSchema {
            name: "bash".into(),
            description: "run".into(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let body = adapter.build_body(
            vec![XyLlmMessage::user("hi")],
            &tools,
            false,
            &crate::thinking::XyGenerateOptions {
                thinking_level: "medium".into(),
                ..Default::default()
            },
        );
        assert_eq!(body["tools"][0]["strict"], false);
    }

    #[test]
    fn reasoning_output_item_done_emits_thinking_end_with_signature() {
        let mut state = ResponsesStreamState::default();
        let item = serde_json::json!({
            "type": "reasoning",
            "id": "rs_1",
            "summary": [{"type": "summary_text", "text": "plan"}],
            "encrypted_content": "enc"
        });
        let event = serde_json::json!({
            "type": "response.output_item.done",
            "item": item,
        });
        let chunks = map_responses_sse_event(&event, &mut state);
        match &chunks[..] {
            [
                XyChunk::ThinkingEnd {
                    thinking,
                    thinking_signature: Some(sig),
                },
            ] => {
                assert_eq!(thinking, "plan");
                let parsed: Value = serde_json::from_str(sig).unwrap();
                assert_eq!(parsed["id"], "rs_1");
                assert_eq!(parsed["encrypted_content"], "enc");
            }
            other => panic!("expected ThinkingEnd, got {other:?}"),
        }
    }

    #[test]
    fn system_prompt_prepends_developer_when_thinking_on() {
        let msgs = vec![XyLlmMessage::user("hi")];
        let opts = crate::thinking::XyGenerateOptions {
            thinking_level: "medium".into(),
            system_prompt: Some("You are xylitol".into()),
            ..Default::default()
        };
        let items = messages_to_responses_input_with_options(&msgs, &opts);
        assert_eq!(items[0]["role"], "developer");
        assert_eq!(items[0]["content"], "You are xylitol");
        assert_eq!(items[1]["role"], "user");
    }

    #[test]
    fn system_prompt_prepends_system_when_thinking_off() {
        let msgs = vec![XyLlmMessage::user("hi")];
        let opts = crate::thinking::XyGenerateOptions {
            system_prompt: Some("sys".into()),
            ..Default::default()
        };
        let items = messages_to_responses_input_with_options(&msgs, &opts);
        assert_eq!(items[0]["role"], "system");
    }

    #[test]
    fn assistant_thinking_without_signature_not_in_output_text() {
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::Thinking {
                    thinking: "secret chain of thought".into(),
                    redacted: false,
                    thinking_signature: None,
                },
                AgentPart::text("visible"),
            ],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: "openai-responses".into(),
            provider: "test".into(),
            model: "m".into(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let items = messages_to_responses_input(&msgs);
        let text_item = items
            .iter()
            .find(|i| i.get("role") == Some(&serde_json::json!("assistant")))
            .expect("assistant message");
        let out = text_item["content"][0]["text"].as_str().unwrap();
        assert_eq!(out, "visible");
        assert!(!out.contains("secret"));
    }

    #[test]
    fn assistant_thinking_with_signature_replays_reasoning_item() {
        let reasoning = serde_json::json!({
            "type": "reasoning",
            "id": "rs_1",
            "summary": [{"type": "summary_text", "text": "plan"}]
        });
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::Thinking {
                    thinking: "plan".into(),
                    redacted: false,
                    thinking_signature: Some(reasoning.to_string()),
                },
                AgentPart::text("ok"),
            ],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: "openai-responses".into(),
            provider: "test".into(),
            model: "m".into(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let items = messages_to_responses_input(&msgs);
        assert_eq!(items[0]["type"], "reasoning");
        assert_eq!(items[0]["id"], "rs_1");
        assert_eq!(items[1]["role"], "assistant");
    }

    #[test]
    fn empty_encrypted_signature_still_full_replays() {
        let reasoning = serde_json::json!({
            "type": "reasoning",
            "id": "rs_empty",
            "encrypted_content": "",
            "summary": []
        });
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::Thinking {
                    thinking: "t".into(),
                    redacted: false,
                    thinking_signature: Some(reasoning.to_string()),
                },
                AgentPart::text("hi"),
            ],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: "openai-responses".into(),
            provider: "test".into(),
            model: "m".into(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let items = messages_to_responses_input(&msgs);
        assert_eq!(items[0]["type"], "reasoning");
        assert_eq!(items[0]["encrypted_content"], "");
        assert_eq!(items[0]["id"], "rs_empty");
    }

    #[test]
    fn illegal_thinking_signature_omitted_not_merged_into_text() {
        let msgs = vec![XyLlmMessage::AssistantMessage {
            content: vec![
                AgentPart::Thinking {
                    thinking: "should-not-leak".into(),
                    redacted: false,
                    thinking_signature: Some("not-json{{{{".into()),
                },
                AgentPart::text("visible"),
            ],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: "openai-responses".into(),
            provider: "test".into(),
            model: "m".into(),
            response_id: None,
            error_message: None,
            timestamp: 0,
            diagnostics: Vec::new(),
        }];
        let (items, diags) = messages_to_responses_input_with_diagnostics(&msgs);
        assert!(
            items
                .iter()
                .all(|i| i.get("type") != Some(&serde_json::json!("reasoning"))),
            "illegal signature must not invent reasoning item: {items:?}"
        );
        let text = items
            .iter()
            .find(|i| i.get("role") == Some(&serde_json::json!("assistant")))
            .expect("assistant")["content"][0]["text"]
            .as_str()
            .unwrap();
        assert_eq!(text, "visible");
        assert!(!text.contains("should-not-leak"));
        assert_eq!(diags.len(), 1, "expect omit diagnostic: {diags:?}");
        assert!(
            diags[0].message.contains("omit illegal thinkingSignature"),
            "{diags:?}"
        );
        assert_eq!(diags[0].source.as_deref(), Some("openai-responses"));
    }

    #[test]
    fn compact_shaped_history_does_not_invent_old_reasoning_signature() {
        let msgs = vec![
            XyLlmMessage::user("[Context summary: prior turns summarized]"),
            XyLlmMessage::user("continue after compact"),
            XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::text("kept reply")],
                stop_reason: Some(XyStopReason::Stop),
                usage: None,
                api: "openai-responses".into(),
                provider: "test".into(),
                model: "m".into(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
        ];
        let items = messages_to_responses_input(&msgs);
        assert!(
            items
                .iter()
                .all(|i| i.get("id") != Some(&serde_json::json!("rs_summarized_away"))),
            "must not invent summarized-away reasoning id: {items:?}"
        );
        assert!(
            items
                .iter()
                .all(|i| i.get("type") != Some(&serde_json::json!("reasoning"))),
            "compact-shaped history has no thinkingSignature to replay: {items:?}"
        );
    }

    #[test]
    fn kept_signature_after_compact_summary_still_replays() {
        let reasoning = serde_json::json!({
            "type": "reasoning",
            "id": "rs_kept",
            "summary": [{"type": "summary_text", "text": "plan"}]
        });
        let msgs = vec![
            XyLlmMessage::user("[Context summary: prior turns summarized]"),
            XyLlmMessage::AssistantMessage {
                content: vec![
                    AgentPart::Thinking {
                        thinking: "plan".into(),
                        redacted: false,
                        thinking_signature: Some(reasoning.to_string()),
                    },
                    AgentPart::text("kept"),
                ],
                stop_reason: Some(XyStopReason::Stop),
                usage: None,
                api: "openai-responses".into(),
                provider: "test".into(),
                model: "m".into(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
        ];
        let items = messages_to_responses_input(&msgs);
        assert_eq!(
            items
                .iter()
                .filter(|i| i.get("type") == Some(&serde_json::json!("reasoning")))
                .count(),
            1
        );
        assert_eq!(
            items
                .iter()
                .find(|i| i.get("type") == Some(&serde_json::json!("reasoning")))
                .unwrap()["id"],
            "rs_kept"
        );
    }

    #[test]
    fn completed_backfills_encrypted_before_done() {
        let mut state = ResponsesStreamState::default();
        let done = serde_json::json!({
            "type": "response.output_item.done",
            "item": {
                "type": "reasoning",
                "id": "rs_bf",
                "summary": [{"type": "summary_text", "text": "plan"}],
                "encrypted_content": ""
            }
        });
        let chunks_done = map_responses_sse_event(&done, &mut state);
        assert!(matches!(
            &chunks_done[..],
            [XyChunk::ThinkingEnd {
                thinking_signature: Some(_),
                ..
            }]
        ));

        let completed = serde_json::json!({
            "type": "response.completed",
            "response": {
                "output": [{
                    "type": "reasoning",
                    "id": "rs_bf",
                    "summary": [{"type": "summary_text", "text": "plan"}],
                    "encrypted_content": "enc_blob"
                }],
                "usage": {"input_tokens": 10, "output_tokens": 2}
            }
        });
        let chunks = map_responses_sse_event(&completed, &mut state);
        assert!(chunks.len() >= 2, "backfill ThinkingEnd + Done: {chunks:?}");
        match &chunks[..] {
            [
                XyChunk::ThinkingEnd {
                    thinking_signature: Some(sig),
                    ..
                },
                XyChunk::Done { .. },
            ] => {
                let parsed: Value = serde_json::from_str(sig).unwrap();
                assert_eq!(parsed["encrypted_content"], "enc_blob");
            }
            other => panic!("expected ThinkingEnd then Done, got {other:?}"),
        }
    }

    #[test]
    fn incomplete_also_backfills_encrypted() {
        let mut state = ResponsesStreamState::default();
        let _ = map_responses_sse_event(
            &serde_json::json!({
                "type": "response.output_item.done",
                "item": {
                    "type": "reasoning",
                    "id": "rs_inc",
                    "encrypted_content": null
                }
            }),
            &mut state,
        );
        let chunks = map_responses_sse_event(
            &serde_json::json!({
                "type": "response.incomplete",
                "response": {
                    "output": [{
                        "type": "reasoning",
                        "id": "rs_inc",
                        "encrypted_content": "later_enc"
                    }]
                }
            }),
            &mut state,
        );
        match &chunks[..] {
            [
                XyChunk::ThinkingEnd {
                    thinking_signature: Some(sig),
                    ..
                },
                XyChunk::Done {
                    finish_reason: XyStopReason::MaxTokens,
                    ..
                },
            ] => {
                let parsed: Value = serde_json::from_str(sig).unwrap();
                assert_eq!(parsed["encrypted_content"], "later_enc");
            }
            other => panic!("expected backfill+MaxTokens Done, got {other:?}"),
        }
    }

    #[test]
    fn extract_embedded_error_prefers_upstream_message_over_deserialize_noise() {
        let raw = r#"error decoding response body: error decoding response body: data did not match any variant of untagged enum ErrorSource at line 1 column 200 content:{"error":{"code":400,"message":"Your input exceeds the available context size.","type":"exceed_context_size_error"}}"#;
        let msg = extract_embedded_provider_error_message(raw).expect("extract");
        assert!(msg.contains("exceeds the available context size"), "{msg}");
        assert!(msg.contains("exceed_context_size_error"), "{msg}");
        let formatted = format_responses_error(&raw);
        assert!(formatted.starts_with("OpenAI Responses:"));
        assert!(
            formatted.contains("exceeds the available context size"),
            "{formatted}"
        );
        assert!(
            !formatted.contains("did not match any variant"),
            "{formatted}"
        );
    }

    #[test]
    fn response_failed_event_surfaces_error_with_code() {
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(
            &serde_json::json!({
                "type": "response.failed",
                "response": {
                    "status": "failed",
                    "error": {"code": "server_error", "message": "boom"}
                }
            }),
            &mut state,
        );
        assert!(chunks.is_empty());
        let err = state
            .terminal_error
            .take()
            .expect("failed event must surface error");
        let text = err.to_string();
        assert!(text.contains("server_error"), "{text}");
        assert!(text.contains("boom"), "{text}");
        assert!(!state.done_emitted);
    }

    #[test]
    fn stream_error_event_surfaces_error() {
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(
            &serde_json::json!({
                "type": "error",
                "code": "rate_limit_exceeded",
                "message": "slow down",
                "param": null
            }),
            &mut state,
        );
        assert!(chunks.is_empty());
        let err = state
            .terminal_error
            .take()
            .expect("error event must surface error");
        let text = err.to_string();
        assert!(text.contains("rate_limit_exceeded"), "{text}");
        assert!(text.contains("slow down"), "{text}");
    }

    #[test]
    fn incomplete_content_filter_maps_to_error_stop_reason() {
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(
            &serde_json::json!({
                "type": "response.incomplete",
                "response": {
                    "incomplete_details": {"reason": "content_filter"},
                    "usage": {"input_tokens": 3, "output_tokens": 2}
                }
            }),
            &mut state,
        );
        match chunks.last().expect("done") {
            XyChunk::Done { finish_reason, .. } => assert_eq!(*finish_reason, XyStopReason::Error),
            other => panic!("expected Done, got {other:?}"),
        }
        assert!(state.done_emitted);
    }

    #[test]
    fn incomplete_max_output_tokens_maps_to_max_tokens() {
        let mut state = ResponsesStreamState::default();
        let chunks = map_responses_sse_event(
            &serde_json::json!({
                "type": "response.incomplete",
                "response": {
                    "incomplete_details": {"reason": "max_output_tokens"},
                    "usage": {"input_tokens": 3, "output_tokens": 2}
                }
            }),
            &mut state,
        );
        match chunks.last().expect("done") {
            XyChunk::Done { finish_reason, .. } => {
                assert_eq!(*finish_reason, XyStopReason::MaxTokens);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn eof_without_terminal_event_errors() {
        use futures::StreamExt;
        let events: Vec<Result<Value, async_openai::error::OpenAIError>> =
            vec![Ok(serde_json::json!({
                "type": "response.output_text.delta",
                "delta": "partial"
            }))];
        let mut stream =
            responses_sdk_stream(futures::stream::iter(events), None, WirePolicy::default());
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            items.push(item);
        }
        let err = items
            .last()
            .expect("items")
            .as_ref()
            .expect_err("EOF must surface error");
        assert!(err.to_string().contains("without terminal event"), "{err}");
    }
}
