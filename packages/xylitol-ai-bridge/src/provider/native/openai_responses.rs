//! OpenAI Responses API adapter via `async-openai` [`Client`] (c1070).
//!
//! Streaming and non-streaming `/v1/responses` calls; reasoning text →
//! [`AiBridgeChunk::ThinkingDelta`], output text → [`AiBridgeChunk::TextDelta`].

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;

use crate::dto::AiBridgeStream;
use crate::dto::{AiBridgeChunk, AiBridgeToolSchema, Diagnostic};
use crate::dto::{AiBridgeMessage, AiBridgePart, AiBridgeStopReason};
use crate::error::AiBridgeError;
use crate::hooks::HttpHooks;
use crate::provider::native::openai_client::{OpenAiClientFactory, normalize_openai_v1_base};
use crate::wire_policy::WirePolicy;

use crate::provider::AiBridgeLlmAdapter;

/// Adapter for the OpenAI Responses API (`/v1/responses`).
pub struct OpenAiResponsesAdapter {
    client: OpenAiClientFactory,
    model: String,
    wire_policy: WirePolicy,
}

impl OpenAiResponsesAdapter {
    /// Create a new Responses API adapter with [`WirePolicy::default`].
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        Self::with_wire_policy(api_key, model, base_url, hooks, WirePolicy::default())
    }

    /// Create a Responses adapter with an explicit wire policy (tests / inject).
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

    /// Wire policy used for req/resp field expectations (c1880).
    pub fn wire_policy(&self) -> WirePolicy {
        self.wire_policy
    }

    fn build_body(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        stream: bool,
        options: &crate::thinking::AiBridgeGenerateOptions,
    ) -> Value {
        // Sole business-layout path for Responses bodies (c1890).
        super::assembler::ResponsesAssembler::new(self.wire_policy).assemble(
            &self.model,
            messages,
            tools,
            stream,
            options,
        )
    }

    fn map_err(err: async_openai::error::OpenAIError) -> AiBridgeError {
        AiBridgeError::Provider(anyhow::anyhow!("{}", format_responses_error(&err)))
    }
}

/// Prefer upstream `error.message` when the SDK string embeds `content:{...}` JSON
/// (e.g. integer `code` deserialize failure masking `exceed_context_size_error`).
pub fn format_responses_error(err: &impl std::fmt::Display) -> String {
    let raw = err.to_string();
    match extract_embedded_provider_error_message(&raw) {
        Some(msg) => format!("OpenAI Responses: {msg}"),
        None => format!("OpenAI Responses: {raw}"),
    }
}

/// Pull `error.message` (+ optional `type`) from an embedded JSON error body.
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

/// Assemble a Responses `/v1/responses` JSON body (pi-aligned store/strict/summary/include).
///
/// Crate-private implementation for [`super::ResponsesAssembler`] (c1890 sole public seam).
///
/// [`WirePolicy`] gates unexposed knobs (`previous_response_id`, `prompt_cache_key`).
/// Also returns full-replay omit diagnostics (illegal `thinkingSignature`).
pub(crate) fn assemble_responses_body_with_diagnostics(
    model: &str,
    messages: Vec<AiBridgeMessage>,
    tools: &[AiBridgeToolSchema],
    stream: bool,
    options: &crate::thinking::AiBridgeGenerateOptions,
    wire_policy: &WirePolicy,
) -> (Value, Vec<Diagnostic>) {
    let (mut input_items, replay_diagnostics) =
        messages_to_responses_input_with_diagnostics(&messages);
    if !replay_diagnostics.is_empty() {
        log::debug!(
            target: "xylitol_ai_bridge::responses",
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
        crate::thinking::AiBridgeThinkingAdapterKind::OpenAi,
    );
    crate::thinking::apply_thinking_openai_responses(&mut body, &resolved);

    if matches!(
        resolved,
        crate::thinking::AiBridgeResolvedThinking::OpenAiEffort(_)
    ) && wire_policy.allows_reasoning_encrypted_include()
    {
        body["include"] = serde_json::json!(["reasoning.encrypted_content"]);
    }

    apply_responses_wire_policy(&mut body, wire_policy);
    (body, replay_diagnostics)
}

/// Strip wire knobs denied by [`WirePolicy`] (c1880).
///
/// Prefer [`super::assembler::ResponsesAssembler::apply_wire_policy`] outside this module.
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

#[async_trait]
impl AiBridgeLlmAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &str {
        &self.model
    }

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "openai-responses",
            &self.model,
            options.obs_parent,
            &options.obs_session,
        );
        let body = self.build_body(messages, tools, true, &options);
        if let Some(t) = &trace {
            t.capture_request_input(&body.to_string());
        }
        // BYOT + `Value`: compatible servers (e.g. llama.cpp) may omit fields that
        // typed `ResponseStreamEvent` requires (`created_at` on `response.created`).
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

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: crate::thinking::AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError> {
        let trace = crate::provider::trace::ProviderRequestTrace::start_with_parent_obs(
            "openai-responses",
            &self.model,
            options.obs_parent,
            &options.obs_session,
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

fn responses_sdk_stream(
    mut sdk_stream: impl Stream<Item = Result<Value, async_openai::error::OpenAIError>>
    + Send
    + Unpin
    + 'static,
    trace: Option<crate::provider::trace::ProviderRequestTrace>,
    wire_policy: WirePolicy,
) -> Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>> {
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
                AiBridgeError::Provider(anyhow::anyhow!(
                    "provider SSE idle: no bytes within {}s",
                    crate::provider::native::wait_bounds::SSE_IDLE.as_secs()
                ))
            })? {
            let data = item.map_err(|e| {
                AiBridgeError::Provider(anyhow::anyhow!("{}", format_responses_error(&e)))
            })?;

            let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(t) = &trace {
                let snippet = data.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                t.emit_raw(event_type, snippet);
            }

            for chunk in map_responses_sse_event(&data, &mut state) {
                if let Some(t) = &trace {
                    t.emit_mapped_chunk(&chunk);
                }
                yield chunk;
            }
        }
    })
}

/// Mutable state for Responses SSE → chunk mapping (streaming tool calls).
#[derive(Default)]
pub struct ResponsesStreamState {
    /// item_id → (name, partial args json, started)
    function_calls: HashMap<String, (String, String, bool)>,
    /// reasoning item id → last done item JSON (for encrypted_content backfill).
    reasoning_items_by_id: HashMap<String, Value>,
    usage_input: u64,
    usage_output: u64,
    /// Last usage JSON blob (for policy-aware cache_read mapping).
    usage_value: Option<Value>,
    /// Wire policy for usage honesty gates (c1880).
    wire_policy: WirePolicy,
}

/// Map one Responses SSE JSON payload (lenient `Value`) into zero or more chunks.
///
/// Compatible servers may emit partial objects (e.g. `response.created` without
/// `created_at`); those events are ignored rather than failing deserialization.
///
/// Public for BDD / harness (c1250 pab13).
pub fn map_responses_sse_event(
    data: &Value,
    state: &mut ResponsesStreamState,
) -> Vec<AiBridgeChunk> {
    let event_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match event_type {
        "response.reasoning_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![AiBridgeChunk::ThinkingDelta(d.to_string())])
            .unwrap_or_default(),
        "response.output_text.delta" => data
            .get("delta")
            .and_then(|v| v.as_str())
            .map(|d| vec![AiBridgeChunk::TextDelta(d.to_string())])
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
            vec![AiBridgeChunk::ToolCallStart { id, name }]
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
                out.push(AiBridgeChunk::ToolCallStart {
                    id: item_id.to_string(),
                    name: entry.0.clone(),
                });
            }
            entry.1.push_str(delta);
            let args = crate::dto::parse_streaming_json(&entry.1);
            out.push(AiBridgeChunk::ToolCallDelta {
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
                out.push(AiBridgeChunk::ToolCallStart {
                    id: id.clone(),
                    name: name.clone(),
                });
            }
            out.push(AiBridgeChunk::ToolCallEnd { id, name, args });
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
        "response.completed" => finish_responses_terminal(data, state, AiBridgeStopReason::Stop),
        "response.incomplete" => {
            finish_responses_terminal(data, state, AiBridgeStopReason::MaxTokens)
        }
        // Partial lifecycle events (`response.created`, `response.in_progress`, …)
        // must not fail the stream on compatible APIs.
        _ => Vec::new(),
    }
}

/// Terminal `completed` / `incomplete`: backfill encrypted signatures then Done.
fn finish_responses_terminal(
    data: &Value,
    state: &mut ResponsesStreamState,
    finish_reason: AiBridgeStopReason,
) -> Vec<AiBridgeChunk> {
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
                crate::dto::AiBridgeUsage {
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
    out.push(AiBridgeChunk::Done {
        finish_reason,
        usage,
    });
    out
}

/// Merge non-empty `encrypted_content` from terminal `response.output` into stored
/// reasoning items (pi Azure / store:false backfill). Emits ThinkingEnd updates.
fn backfill_reasoning_encrypted_from_terminal(
    data: &Value,
    state: &mut ResponsesStreamState,
) -> Vec<AiBridgeChunk> {
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

fn thinking_end_from_reasoning_item(item: &Value) -> AiBridgeChunk {
    AiBridgeChunk::ThinkingEnd {
        thinking: reasoning_item_display_text(item),
        thinking_signature: Some(item.to_string()),
    }
}

fn parse_responses_output(json: &Value, wire_policy: WirePolicy) -> Vec<AiBridgeChunk> {
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
                                chunks.push(AiBridgeChunk::TextDelta(text.to_string()));
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
                    chunks.push(AiBridgeChunk::ToolCallStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    chunks.push(AiBridgeChunk::ToolCallEnd { id, name, args });
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

    chunks.push(AiBridgeChunk::Done {
        finish_reason: AiBridgeStopReason::Stop,
        usage,
    });

    chunks
}

// ── AiBridgeMessage conversion ────────────────────────────────────

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI Responses `input` items.
pub fn messages_to_responses_input(messages: &[AiBridgeMessage]) -> Vec<Value> {
    messages_to_responses_input_with_diagnostics(messages).0
}

/// Like [`messages_to_responses_input`], also returning omit/replay diagnostics
/// (e.g. illegal `thinkingSignature` JSON omitted under full-replay).
pub fn messages_to_responses_input_with_diagnostics(
    messages: &[AiBridgeMessage],
) -> (Vec<Value>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let items = convert_messages_to_input_items(messages, &mut diagnostics);
    (items, diagnostics)
}

/// Build Responses `input` with optional system/developer prepend (c1270 / pi align).
pub fn messages_to_responses_input_with_options(
    messages: &[AiBridgeMessage],
    options: &crate::thinking::AiBridgeGenerateOptions,
) -> Vec<Value> {
    let (mut items, _) = messages_to_responses_input_with_diagnostics(messages);
    prepend_system_prompt_item(
        &mut items,
        options.system_prompt.as_deref(),
        &options.thinking_level,
    );
    items
}

/// Prepend system prompt as `developer` (thinking on) or `system` (off), matching pi.
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
    // pi uses bare `{role, content: string}` for system/developer; keep that shape.
    items.insert(
        0,
        serde_json::json!({
            "role": role,
            "content": sp,
        }),
    );
}

/// Convert a slice of [`AiBridgeMessage`] values to OpenAI Responses `input` items.
fn convert_messages_to_input_items(
    messages: &[AiBridgeMessage],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Value> {
    let mut items: Vec<Value> = Vec::new();

    for msg in messages {
        match msg {
            AiBridgeMessage::UserMessage { content, .. } => {
                let text = collect_user_text_parts(content);
                if !text.is_empty() {
                    // Responses API requires each input item to declare its
                    // `type`; a bare {role, content} object yields
                    // "Cannot determine type of 'item'" once non-message items
                    // (function_call / function_call_output) are mixed in.
                    items.push(serde_json::json!({
                        "type": "message",
                        "role": "user",
                        "content": [{"type": "input_text", "text": text}],
                    }));
                }
            }
            AiBridgeMessage::AssistantMessage { content, .. } => {
                // Full-replay only: opaque thinkingSignature → reasoning item (pi).
                for part in content {
                    if let AiBridgePart::Thinking {
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
                                    target: "xylitol_ai_bridge::responses",
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
                        AiBridgePart::ToolCall {
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
                    // Only emit a message item when there is assistant text;
                    // a tool-call round may have no text (pure function_call).
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
            AiBridgeMessage::ToolResultMessage {
                tool_use_id,
                content,
                ..
            } => {
                let text = collect_user_text_parts(content);
                items.push(serde_json::json!({
                    "type": "function_call_output",
                    "call_id": tool_use_id,
                    "output": text,
                }));
            }
        }
    }

    items
}

/// User / tool-result text (Text + Thinking for tool output payloads).
fn collect_user_text_parts(parts: &[AiBridgePart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let Some(text) = part.as_text() {
            buf.push_str(text);
        }
    }
    buf
}

/// Assistant `output_text` MUST be Text-only (c1270): never merge Thinking.
fn collect_assistant_output_text(parts: &[AiBridgePart]) -> String {
    let mut buf = String::new();
    for part in parts {
        if let AiBridgePart::Text { text } = part {
            buf.push_str(text);
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{AiBridgeMessage, AiBridgePart};

    /// Every Responses API input item MUST declare a `type`, or OpenAI rejects
    /// the request with "Cannot determine type of 'item'" once non-message
    /// items (function_call / function_call_output) are mixed in.
    #[test]
    fn all_items_have_type_field() {
        let msgs = vec![
            AiBridgeMessage::user("hello"),
            AiBridgeMessage::assistant("hi there"),
        ];
        let items = messages_to_responses_input(&msgs);
        for item in &items {
            assert!(item.get("type").is_some(), "item missing `type`: {item}");
        }
    }

    /// Regression: a tool-calling continuation round (user → assistant with a
    /// tool_call → tool result) must produce well-typed items: message,
    /// function_call, function_call_output. This is exactly the sequence that
    /// failed before c375 (no continuation) and then hit the Responses API
    /// "Cannot determine type of 'item'" error after c375 enabled it.
    #[test]
    fn tool_round_items_are_well_typed() {
        let msgs = vec![
            AiBridgeMessage::user("list files"),
            AiBridgeMessage::AssistantMessage {
                content: vec![
                    AiBridgePart::text("let me check"),
                    AiBridgePart::ToolCall {
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
            AiBridgeMessage::tool_result(
                "call-1",
                "ls",
                vec![AiBridgePart::text("file.txt")],
                false,
            ),
        ];
        let items = messages_to_responses_input(&msgs);
        // user message, assistant message, function_call, function_call_output
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
        // function_call_output must carry the call_id matching the call.
        let output = items
            .iter()
            .find(|i| i.get("type").and_then(|v| v.as_str()) == Some("function_call_output"))
            .unwrap();
        assert_eq!(output["call_id"], "call-1");
        assert_eq!(output["output"], "file.txt");
    }

    #[test]
    fn assistant_tool_only_round_emits_function_call_not_empty_message() {
        // Assistant round with a tool_call and NO text must not emit an empty
        // message item (would confuse the API); only the function_call item.
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![AiBridgePart::ToolCall {
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
        // llama.cpp (and other compatible servers) may omit `created_at` on
        // `response.created`; typed ResponseStreamEvent would fail here.
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
        assert!(matches!(&chunks[0], AiBridgeChunk::TextDelta(t) if t == "hello"));
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
            AiBridgeChunk::Done { usage: Some(u), .. } => {
                assert_eq!(u.input, 3);
                assert_eq!(u.output, 5);
                assert_eq!(u.total_tokens, 8);
            }
            other => panic!("expected Done with usage, got {other:?}"),
        }
    }

    #[test]
    fn function_call_args_stream_before_done() {
        // t0718-shaped: item.added → many args deltas → item.done
        let mut state = ResponsesStreamState::default();
        let added = serde_json::json!({
            "type": "response.output_item.added",
            "item": { "type": "function_call", "id": "fc_1", "name": "ls", "arguments": "" }
        });
        let start = map_responses_sse_event(&added, &mut state);
        assert!(matches!(
            &start[..],
            [AiBridgeChunk::ToolCallStart { id, name }] if id == "fc_1" && name == "ls"
        ));

        let d1 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "{\"path\":"
        });
        let mid = map_responses_sse_event(&d1, &mut state);
        assert!(
            matches!(&mid[..], [AiBridgeChunk::ToolCallDelta { id, args_delta, .. }] if id == "fc_1" && args_delta == "{\"path\":"),
            "got {mid:?}"
        );

        let d2 = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "delta": "\"/tmp\"}"
        });
        let mid2 = map_responses_sse_event(&d2, &mut state);
        assert!(matches!(&mid2[..], [AiBridgeChunk::ToolCallDelta { .. }]));

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
            [AiBridgeChunk::ToolCallEnd { id, name, args }] => {
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
            vec![AiBridgeMessage::user("hi")],
            &[],
            false,
            &crate::thinking::AiBridgeGenerateOptions::default(),
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
        apply_responses_wire_policy(&mut body, &WirePolicy::default());
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
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let body = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &opts);
        assert_eq!(body["reasoning"]["effort"], "medium");
        assert_eq!(body["reasoning"]["summary"], "auto");
        assert_eq!(body["store"], false);
        assert_eq!(
            body["include"],
            serde_json::json!(["reasoning.encrypted_content"])
        );

        let off = crate::thinking::AiBridgeGenerateOptions::default();
        let body_off = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &off);
        assert!(body_off.get("reasoning").is_none());
        assert_eq!(body_off["store"], false);
        assert!(body_off.get("include").is_none());

        let mut map = std::collections::HashMap::new();
        map.insert("high".into(), Some("max".into()));
        let mapped = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "high".into(),
            level_map: map,
            thinking_budgets: None,
            system_prompt: None,
            obs_parent: None,
            obs_session: Default::default(),
        };
        let body_map = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &mapped);
        assert_eq!(body_map["reasoning"]["effort"], "max");
        assert_eq!(body_map["reasoning"]["summary"], "auto");
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
        let opts = crate::thinking::AiBridgeGenerateOptions {
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let body = adapter.build_body(vec![AiBridgeMessage::user("hi")], &[], false, &opts);
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
            AiBridgeToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            AiBridgeToolSchema {
                name: "mcp__context7__resolve-library-id".into(),
                description: "docs".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
            AiBridgeToolSchema {
                name: "mcp__lspz__get_diagnostics".into(),
                description: "lsp".into(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let body = adapter.build_body(
            vec![AiBridgeMessage::user("hi")],
            &tools,
            false,
            &crate::thinking::AiBridgeGenerateOptions {
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
        let tools = [AiBridgeToolSchema {
            name: "bash".into(),
            description: "run".into(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let body = adapter.build_body(
            vec![AiBridgeMessage::user("hi")],
            &tools,
            false,
            &crate::thinking::AiBridgeGenerateOptions {
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
                AiBridgeChunk::ThinkingEnd {
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
        let msgs = vec![AiBridgeMessage::user("hi")];
        let opts = crate::thinking::AiBridgeGenerateOptions {
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
        let msgs = vec![AiBridgeMessage::user("hi")];
        let opts = crate::thinking::AiBridgeGenerateOptions {
            system_prompt: Some("sys".into()),
            ..Default::default()
        };
        let items = messages_to_responses_input_with_options(&msgs, &opts);
        assert_eq!(items[0]["role"], "system");
    }

    #[test]
    fn assistant_thinking_without_signature_not_in_output_text() {
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "secret chain of thought".into(),
                    redacted: false,
                    thinking_signature: None,
                },
                AiBridgePart::text("visible"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
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
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "plan".into(),
                    redacted: false,
                    thinking_signature: Some(reasoning.to_string()),
                },
                AiBridgePart::text("ok"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
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
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "t".into(),
                    redacted: false,
                    thinking_signature: Some(reasoning.to_string()),
                },
                AiBridgePart::text("hi"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
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
        let msgs = vec![AiBridgeMessage::AssistantMessage {
            content: vec![
                AiBridgePart::Thinking {
                    thinking: "should-not-leak".into(),
                    redacted: false,
                    thinking_signature: Some("not-json{{{{".into()),
                },
                AiBridgePart::text("visible"),
            ],
            stop_reason: Some(AiBridgeStopReason::Stop),
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

    /// c27 seam (bridge): post-compact working history has no summarized-away
    /// thinkingSignature — assemble MUST NOT invent old reasoning ids.
    #[test]
    fn compact_shaped_history_does_not_invent_old_reasoning_signature() {
        let msgs = vec![
            AiBridgeMessage::user("[Context summary: prior turns summarized]"),
            AiBridgeMessage::user("continue after compact"),
            AiBridgeMessage::AssistantMessage {
                content: vec![AiBridgePart::text("kept reply")],
                stop_reason: Some(AiBridgeStopReason::Stop),
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

    /// Positive control: kept assistant with signature still full-replays after
    /// a compaction summary user row (c27 must not strip retained signatures).
    #[test]
    fn kept_signature_after_compact_summary_still_replays() {
        let reasoning = serde_json::json!({
            "type": "reasoning",
            "id": "rs_kept",
            "summary": [{"type": "summary_text", "text": "plan"}]
        });
        let msgs = vec![
            AiBridgeMessage::user("[Context summary: prior turns summarized]"),
            AiBridgeMessage::AssistantMessage {
                content: vec![
                    AiBridgePart::Thinking {
                        thinking: "plan".into(),
                        redacted: false,
                        thinking_signature: Some(reasoning.to_string()),
                    },
                    AiBridgePart::text("kept"),
                ],
                stop_reason: Some(AiBridgeStopReason::Stop),
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
            [AiBridgeChunk::ThinkingEnd {
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
                AiBridgeChunk::ThinkingEnd {
                    thinking_signature: Some(sig),
                    ..
                },
                AiBridgeChunk::Done { .. },
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
                AiBridgeChunk::ThinkingEnd {
                    thinking_signature: Some(sig),
                    ..
                },
                AiBridgeChunk::Done {
                    finish_reason: AiBridgeStopReason::MaxTokens,
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
}
