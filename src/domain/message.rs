//! Typed agent message types — session SSOT with LLM / Env composition (c1070).
//!
//! Provides [`AgentMessage`] (`Llm` ∪ `Env`), [`LlmMessage`], [`EnvMessage`],
//! [`AgentPart`], [`XyUsage`], [`XyStopReason`], [`AgentState`], and
//! [`AgentContext`] as the canonical agent data model.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

/// Current timestamp in milliseconds since Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── LlmMessage / EnvMessage / AgentMessage (c1070 composition) ─

/// LLM-visible turn content only (user / assistant / toolResult).
///
/// Session history wraps this in [`AgentMessage::Llm`]. Provider paths take
/// [`crate::domain::llm_project::project_for_llm`] output of this type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum LlmMessage {
    #[serde(rename = "user")]
    UserMessage {
        content: Vec<AgentPart>,
        #[serde(default = "now_ms")]
        timestamp: u64,
    },
    #[serde(rename = "assistant")]
    AssistantMessage {
        content: Vec<AgentPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop_reason: Option<XyStopReason>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<XyUsage>,
        #[serde(default)]
        api: String,
        #[serde(default)]
        provider: String,
        #[serde(default)]
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
        #[serde(default = "now_ms")]
        timestamp: u64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<Diagnostic>,
    },
    #[serde(rename = "toolResult")]
    ToolResultMessage {
        #[serde(rename = "toolCallId")]
        tool_use_id: String,
        #[serde(default)]
        tool_name: String,
        content: Vec<AgentPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
        #[serde(default)]
        is_error: bool,
        #[serde(default = "now_ms")]
        timestamp: u64,
    },
}

impl LlmMessage {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::UserMessage { .. } => "user",
            Self::AssistantMessage { .. } => "assistant",
            Self::ToolResultMessage { .. } => "toolResult",
        }
    }

    pub fn content(&self) -> &[AgentPart] {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => content,
        }
    }

    pub fn text(&self) -> String {
        let mut buf = String::new();
        for part in self.content() {
            match part {
                AgentPart::Text { text } | AgentPart::Thinking { thinking: text, .. } => {
                    buf.push_str(text)
                }
                _ => {}
            }
        }
        buf
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::AssistantMessage {
                stop_reason: Some(XyStopReason::Error | XyStopReason::Aborted),
                ..
            }
        )
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::UserMessage {
            content: vec![AgentPart::text(text)],
            timestamp: now_ms(),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::AssistantMessage {
            content: vec![AgentPart::text(text)],
            stop_reason: Some(XyStopReason::Stop),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: now_ms(),
            diagnostics: Vec::new(),
        }
    }

    pub fn tool_result(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        content: Vec<AgentPart>,
        is_error: bool,
    ) -> Self {
        Self::ToolResultMessage {
            tool_use_id: id.into(),
            tool_name: tool_name.into(),
            content,
            details: None,
            is_error,
            timestamp: now_ms(),
        }
    }
}

/// Environment / session meta roles (not sent to the model as-is).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum EnvMessage {
    #[serde(rename = "bashExecution")]
    BashExecutionMessage {
        command: String,
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(default)]
        cancelled: bool,
        #[serde(default)]
        truncated: bool,
        #[serde(default)]
        exclude_from_context: bool,
    },
    #[serde(rename = "custom")]
    CustomMessage {
        custom_type: String,
        content: Value,
        #[serde(default)]
        display: Value,
        #[serde(default)]
        details: Value,
    },
    #[serde(rename = "compactionSummary")]
    CompactionSummaryMessage {
        summary: String,
        tokens_before: u64,
        tokens_after: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        read_files: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        modified_files: Option<Vec<String>>,
    },
    #[serde(rename = "branchSummary")]
    BranchSummaryMessage { summary: String, from_id: String },
}

impl EnvMessage {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::BashExecutionMessage { .. } => "bashExecution",
            Self::CustomMessage { .. } => "custom",
            Self::CompactionSummaryMessage { .. } => "compactionSummary",
            Self::BranchSummaryMessage { .. } => "branchSummary",
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::BashExecutionMessage {
                command, output, ..
            } => format!("$ {command}\n{output}"),
            Self::CustomMessage { content, .. } => content.as_str().unwrap_or("").to_string(),
            Self::CompactionSummaryMessage { summary, .. }
            | Self::BranchSummaryMessage { summary, .. } => summary.clone(),
        }
    }
}

/// Session/agent transcript entry: LLM turn **or** environment meta.
///
/// Wire format stays flat `{ "role": … }` via `untagged` + inner tagged enums.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentMessage {
    Llm(LlmMessage),
    Env(EnvMessage),
}

impl From<LlmMessage> for AgentMessage {
    fn from(value: LlmMessage) -> Self {
        Self::Llm(value)
    }
}

impl From<EnvMessage> for AgentMessage {
    fn from(value: EnvMessage) -> Self {
        Self::Env(value)
    }
}

impl AgentMessage {
    pub fn as_llm(&self) -> Option<&LlmMessage> {
        match self {
            Self::Llm(m) => Some(m),
            Self::Env(_) => None,
        }
    }

    pub fn as_env(&self) -> Option<&EnvMessage> {
        match self {
            Self::Env(m) => Some(m),
            Self::Llm(_) => None,
        }
    }

    pub fn role_name(&self) -> &'static str {
        match self {
            Self::Llm(m) => m.role_name(),
            Self::Env(m) => m.role_name(),
        }
    }

    pub fn content(&self) -> &[AgentPart] {
        match self {
            Self::Llm(m) => m.content(),
            Self::Env(_) => &[],
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::Llm(m) => m.text(),
            Self::Env(m) => m.text(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.as_llm().is_some_and(LlmMessage::is_error)
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::Llm(LlmMessage::user(text))
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::Llm(LlmMessage::assistant(text))
    }

    pub fn tool_result(
        id: impl Into<String>,
        tool_name: impl Into<String>,
        content: Vec<AgentPart>,
        is_error: bool,
    ) -> Self {
        Self::Llm(LlmMessage::tool_result(id, tool_name, content, is_error))
    }

    pub fn bash(
        command: impl Into<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self::Env(EnvMessage::BashExecutionMessage {
            command: command.into(),
            output: output.into(),
            exit_code,
            cancelled: false,
            truncated: false,
            exclude_from_context: false,
        })
    }
}

// ── AgentPart ───────────────────────────────────────────────────────

/// A single content part within an [`AgentMessage`].
///
/// Wire (c646 / pi): internally tagged with `"type"`. Tool results MUST NOT appear
/// as content parts — use [`LlmMessage::ToolResultMessage`] / [`AgentMessage::tool_result`]
/// rows instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentPart {
    /// Plain text content (`{"type":"text","text":…}`).
    Text { text: String },
    /// An image (url or base64 data).
    Image(ImageContent),
    /// Thinking / reasoning (`{"type":"thinking","thinking":…}`).
    Thinking {
        thinking: String,
        /// Whether the thinking content was redacted by safety filters.
        #[serde(default)]
        redacted: bool,
        /// Opaque signature for multi-turn thinking continuity (wire: `thinkingSignature`).
        #[serde(
            default,
            rename = "thinkingSignature",
            skip_serializing_if = "Option::is_none"
        )]
        thinking_signature: Option<String>,
    },
    /// A tool call request (assistant → tool).
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
}

impl AgentPart {
    /// Construct a text part.
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text { text: s.into() }
    }

    /// Construct a thinking part (no signature).
    pub fn thinking(s: impl Into<String>) -> Self {
        Self::Thinking {
            thinking: s.into(),
            redacted: false,
            thinking_signature: None,
        }
    }

    /// Returns `true` if this part is a [`ToolCall`](AgentPart::ToolCall).
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { .. })
    }

    /// Returns `true` if this part is text or thinking content.
    pub fn is_text_content(&self) -> bool {
        matches!(self, Self::Text { .. } | Self::Thinking { .. })
    }

    /// Return the text content if this is a text or thinking part.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text.as_str()),
            Self::Thinking { thinking, .. } => Some(thinking.as_str()),
            _ => None,
        }
    }
}

// ── ImageContent ────────────────────────────────────────────────────

/// An image attachment (flattened under `AgentPart::Image` with `type: "image"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// Public URL of the image (if available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Base64-encoded image data (inline).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// MIME type (e.g. `image/png`, `image/jpeg`). Wire key: `mimeType`.
    #[serde(rename = "mimeType")]
    pub media_type: String,
}

// ── XyUsage & XyUsageCost ───────────────────────────────────────────────

/// Token usage statistics for a model invocation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct XyUsage {
    pub input: u64,
    pub output: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_write: u64,
    /// Anthropic 1-hour cache write subset.
    #[serde(default)]
    pub cache_write_1h: u64,
    #[serde(skip)]
    pub total_tokens: u64,
    /// Estimated cost in USD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<XyUsageCost>,
}

/// Estimated cost breakdown in USD.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct XyUsageCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl XyUsage {
    /// Recompute `total_tokens` from input + output.
    pub fn compute_total(&mut self) {
        self.total_tokens = self.input + self.output;
    }

    /// Estimate cost from per-token rates (cost per million tokens).
    pub fn compute_cost(
        &mut self,
        per_m_input: f64,
        per_m_output: f64,
        per_m_cache_read: f64,
        per_m_cache_write: f64,
    ) {
        let to_cost = |tokens: u64, rate: f64| (tokens as f64) * rate / 1_000_000.0;
        let input_c = to_cost(self.input, per_m_input);
        let output_c = to_cost(self.output, per_m_output);
        let cache_read_c = to_cost(self.cache_read, per_m_cache_read);
        let cache_write_c = to_cost(self.cache_write, per_m_cache_write);
        self.cost = Some(XyUsageCost {
            input: input_c,
            output: output_c,
            cache_read: cache_read_c,
            cache_write: cache_write_c,
            total: input_c + output_c + cache_read_c + cache_write_c,
        });
    }
}

// ── XyStopReason ──────────────────────────────────────────────────────

/// Why the assistant stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum XyStopReason {
    /// Normal stop — model finished its response.
    Stop,
    /// Hit the max_tokens limit.
    #[serde(rename = "length")]
    MaxTokens,
    /// An error occurred during generation.
    Error,
    /// Generation was aborted (user cancel).
    Aborted,
    /// Model issued tool call(s) and is waiting for results.
    ToolUse,
}

/// Provider/runtime diagnostic for failures and recoveries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional source identifier (e.g. provider name, tool name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

// ── AgentState ──────────────────────────────────────────────────────

/// Public agent state snapshot.
#[derive(Debug, Clone)]
pub struct AgentState {
    /// System prompt sent with each model request.
    pub system_prompt: String,
    /// Active model name.
    pub model: String,
    /// Thinking/reasoning level.
    pub thinking_level: String,
    /// Available tool names.
    pub tool_names: Vec<String>,
    /// Conversation transcript.
    pub messages: Vec<AgentMessage>,
    /// True while the agent is processing a prompt or continuation.
    pub is_streaming: bool,
    /// Partial assistant message for the current streamed response, if any.
    pub streaming_message: Option<AgentMessage>,
    /// Tool call ids currently executing.
    pub pending_tool_calls: Vec<String>,
    /// Error message from the most recent failed or aborted assistant turn.
    pub error_message: Option<String>,
}

// ── AgentContext ────────────────────────────────────────────────────

/// Context snapshot passed into the low-level agent loop before each LLM call.
#[derive(Debug, Clone)]
pub struct AgentContext {
    /// System prompt included with the request.
    pub system_prompt: String,
    /// Transcript visible to the model.
    pub messages: Vec<AgentMessage>,
    /// Tool names available for this run.
    pub tool_names: Vec<String>,
}

// ── Convenience helpers ─────────────────────────────────────────────

/// Helper: collect text content from a slice of [`AgentPart`], skipping
/// non-text parts.
pub fn collect_text_parts(parts: &[AgentPart]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            AgentPart::Text { text } | AgentPart::Thinking { thinking: text, .. } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Display ─────────────────────────────────────────────────────────

impl std::fmt::Display for AgentMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.role_name(), self.text())
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_message_has_new_fields() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::text("response")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(XyUsage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: 0,
                total_tokens: 150,
                cost: Some(XyUsageCost {
                    input: 0.001,
                    output: 0.002,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    total: 0.003,
                }),
            }),
            api: "openai".into(),
            provider: "openai".into(),
            model: "gpt-4".into(),
            response_id: Some("resp-123".into()),
            error_message: None,
            timestamp: 1000,
            diagnostics: vec![],
        });
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role_name(), "assistant");
        assert_eq!(deserialized.text(), "response");
        assert!(matches!(deserialized, AgentMessage::Llm(_)));
    }

    #[test]
    fn assistant_error_message() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![],
            stop_reason: Some(XyStopReason::Error),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: Some("Rate limit exceeded".into()),
            timestamp: 1000,
            diagnostics: vec![Diagnostic {
                message: "Retried 3 times".into(),
                source: Some("openai".into()),
            }],
        });
        assert!(msg.is_error());
        assert_eq!(msg.text(), "");
    }

    #[test]
    fn tool_result_message_has_tool_name() {
        let msg = AgentMessage::Llm(LlmMessage::ToolResultMessage {
            tool_use_id: "call-1".into(),
            tool_name: "read_file".into(),
            content: vec![AgentPart::text("file contents")],
            details: Some(serde_json::json!({"path": "src/main.rs", "lines": 42})),
            is_error: false,
            timestamp: 1000,
        });
        assert_eq!(msg.role_name(), "toolResult");
    }

    #[test]
    fn usage_with_cost() {
        let mut usage = XyUsage {
            input: 1000,
            output: 500,
            cache_read: 200,
            cache_write: 100,
            cache_write_1h: 50,
            total_tokens: 0,
            cost: None,
        };
        usage.compute_total();
        assert_eq!(usage.total_tokens, 1500);

        usage.compute_cost(10.0, 30.0, 1.0, 5.0);
        let cost = usage.cost.unwrap();
        assert!((cost.total - (0.010 + 0.015 + 0.0002 + 0.0005)).abs() < 1e-6);
    }

    #[test]
    fn thinking_part_with_metadata() {
        let part = AgentPart::Thinking {
            thinking: "Let me reason...".into(),
            redacted: false,
            thinking_signature: Some("sig-abc".into()),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(
            json.contains(r#""type":"thinking""#) && json.contains(r#""thinking":"#),
            "tagged thinking wire: {json}"
        );
        assert!(
            json.contains("thinkingSignature"),
            "signature key must be thinkingSignature: {json}"
        );
        let deserialized: AgentPart = serde_json::from_str(&json).unwrap();
        match deserialized {
            AgentPart::Thinking {
                thinking,
                redacted,
                thinking_signature,
            } => {
                assert_eq!(thinking, "Let me reason...");
                assert!(!redacted);
                assert_eq!(thinking_signature, Some("sig-abc".into()));
            }
            _ => panic!("expected Thinking"),
        }
        assert!(serde_json::from_str::<AgentPart>(r#""bare""#).is_err());
        assert!(serde_json::from_str::<AgentPart>(r#"{"redacted":false,"text":"x"}"#).is_err());
    }

    #[test]
    fn text_part_is_tagged_not_bare_string() {
        let json = serde_json::to_string(&AgentPart::text("hi")).unwrap();
        assert_eq!(json, r#"{"type":"text","text":"hi"}"#);
    }

    #[test]
    fn tool_result_message_uses_tool_call_id_key() {
        let msg = AgentMessage::tool_result("c1", "read", vec![AgentPart::text("ok")], false);
        let v = serde_json::to_value(&msg).unwrap();
        assert!(v.get("toolCallId").is_some(), "{v}");
        assert!(v.get("toolUseId").is_none(), "{v}");
    }

    #[test]
    fn all_seven_roles_serialize_and_deserialize() {
        let messages = vec![
            AgentMessage::user("hi"),
            AgentMessage::assistant("hello"),
            AgentMessage::tool_result("t1", "read_file", vec![AgentPart::text("done")], false),
            AgentMessage::bash("pwd", "/home", Some(0)),
            AgentMessage::Env(EnvMessage::CustomMessage {
                custom_type: "x".into(),
                content: serde_json::json!({}),
                display: serde_json::json!({}),
                details: serde_json::json!({}),
            }),
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "s".into(),
                tokens_before: 10,
                tokens_after: 2,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                summary: "s".into(),
                from_id: "e-1".into(),
            }),
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.role_name(), msg.role_name());
        }
    }

    #[test]
    fn wire_user_deserializes_as_llm_variant() {
        let json = r#"{"role":"user","content":[{"type":"text","text":"hi"}],"timestamp":1}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(
            msg,
            AgentMessage::Llm(LlmMessage::UserMessage { .. })
        ));
    }

    #[test]
    fn wire_bash_deserializes_as_env_variant() {
        let json = r#"{"role":"bashExecution","command":"ls","output":"a","cancelled":false,"truncated":false,"exclude_from_context":false}"#;
        let msg: AgentMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(
            msg,
            AgentMessage::Env(EnvMessage::BashExecutionMessage { .. })
        ));
    }
}
