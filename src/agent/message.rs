//! Typed agent message types — fully aligned with pi coding agent's type system.
//!
//! Provides [`AgentMessage`] (7 roles), [`AgentPart`] (5 part types),
//! [`Usage`] (with cost), [`StopReason`], [`AgentState`], and
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

// ── AgentMessage ────────────────────────────────────────────────────

/// A single message in the agent conversation.
///
/// Supports all roles required by the pi coding agent pipeline:
/// user, assistant, toolResult, bashExecution, custom, compactionSummary,
/// branchSummary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum AgentMessage {
    /// A user message (text, images, or tool results intended for the
    /// assistant).
    #[serde(rename = "user")]
    UserMessage {
        content: Vec<AgentPart>,
        /// Unix timestamp in milliseconds.
        #[serde(default = "now_ms")]
        timestamp: u64,
    },

    /// An assistant message (text, thinking, tool calls).
    #[serde(rename = "assistant")]
    AssistantMessage {
        content: Vec<AgentPart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop_reason: Option<StopReason>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<Usage>,
        /// Provider API name (e.g. "openai", "anthropic").
        #[serde(default)]
        api: String,
        /// Provider name (e.g. "openai", "anthropic", "deepseek").
        #[serde(default)]
        provider: String,
        /// Model identifier used for this response.
        #[serde(default)]
        model: String,
        /// Provider-specific response identifier.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
        /// Error message when stop_reason is Error or Aborted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
        /// Unix timestamp in milliseconds.
        #[serde(default = "now_ms")]
        timestamp: u64,
        /// Provider/runtime diagnostics for failures and recoveries.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<Diagnostic>,
    },

    /// A tool result message (result of executing a tool call).
    #[serde(rename = "toolResult")]
    ToolResultMessage {
        tool_use_id: String,
        /// Name of the tool that produced this result.
        #[serde(default)]
        tool_name: String,
        content: Vec<AgentPart>,
        /// Arbitrary structured details for logs or UI.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
        #[serde(default)]
        is_error: bool,
        /// Unix timestamp in milliseconds.
        #[serde(default = "now_ms")]
        timestamp: u64,
    },

    /// A bash execution message (user-initiated `!cmd` / `!!cmd`).
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

    /// A custom message (extension-injected content).
    #[serde(rename = "custom")]
    CustomMessage {
        custom_type: String,
        content: Value,
        #[serde(default)]
        display: Value,
        #[serde(default)]
        details: Value,
    },

    /// A compaction summary message (replaces compacted entries).
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

    /// A branch summary message (summarises a forked branch).
    #[serde(rename = "branchSummary")]
    BranchSummaryMessage {
        summary: String,
        from_id: String,
    },
}

impl AgentMessage {
    /// Return the role name as a static string (user, assistant, …).
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::UserMessage { .. } => "user",
            Self::AssistantMessage { .. } => "assistant",
            Self::ToolResultMessage { .. } => "toolResult",
            Self::BashExecutionMessage { .. } => "bashExecution",
            Self::CustomMessage { .. } => "custom",
            Self::CompactionSummaryMessage { .. } => "compactionSummary",
            Self::BranchSummaryMessage { .. } => "branchSummary",
        }
    }

    /// Return the content parts (most variants carry them).
    pub fn content(&self) -> &[AgentPart] {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => content,
            Self::BashExecutionMessage { .. }
            | Self::CustomMessage { .. }
            | Self::CompactionSummaryMessage { .. }
            | Self::BranchSummaryMessage { .. } => &[],
        }
    }

    /// Collapse all text parts into a single string.
    pub fn text(&self) -> String {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => {
                let mut buf = String::new();
                for part in content {
                    match part {
                        AgentPart::Text(t) | AgentPart::Thinking { text: t, .. } => {
                            buf.push_str(t)
                        }
                        _ => {}
                    }
                }
                buf
            }
            Self::BashExecutionMessage { command, output, .. } => {
                format!("$ {command}\n{output}")
            }
            Self::CustomMessage { content, .. } => {
                content.as_str().unwrap_or("").to_string()
            }
            Self::CompactionSummaryMessage { summary, .. } => summary.clone(),
            Self::BranchSummaryMessage { summary, .. } => summary.clone(),
        }
    }

    /// Return `true` if this is an assistant message with an error stop reason.
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::AssistantMessage {
                stop_reason: Some(StopReason::Error | StopReason::Aborted),
                ..
            }
        )
    }
}

// ── AgentPart ───────────────────────────────────────────────────────

/// A single content part within an [`AgentMessage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentPart {
    /// Plain text content.
    Text(String),
    /// An image (url or base64 data).
    Image(ImageContent),
    /// Thinking / reasoning text.
    Thinking {
        text: String,
        /// Whether the thinking content was redacted by safety filters.
        #[serde(default)]
        redacted: bool,
        /// Opaque signature for multi-turn thinking continuity.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// A tool call request (assistant → tool).
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    /// A tool result (tool → assistant).
    ToolResult {
        tool_use_id: String,
        content: Vec<AgentPart>,
        #[serde(default)]
        is_error: bool,
    },
}

impl AgentPart {
    /// Returns `true` if this part is a [`ToolCall`](AgentPart::ToolCall).
    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { .. })
    }

    /// Returns `true` if this part is a [`ToolResult`](AgentPart::ToolResult).
    pub fn is_tool_result(&self) -> bool {
        matches!(self, Self::ToolResult { .. })
    }

    /// Returns `true` if this part is text or thinking content.
    pub fn is_text_content(&self) -> bool {
        matches!(self, Self::Text(_) | Self::Thinking { .. })
    }

    /// Return the text content if this is a text or thinking part.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(t.as_str()),
            Self::Thinking { text, .. } => Some(text.as_str()),
            _ => None,
        }
    }
}

// ── ImageContent ────────────────────────────────────────────────────

/// An image attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    /// Public URL of the image (if available).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Base64-encoded image data (inline).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// MIME type (e.g. `image/png`, `image/jpeg`).
    pub media_type: String,
}

// ── Usage & UsageCost ───────────────────────────────────────────────

/// Token usage statistics for a model invocation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Usage {
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
    pub cost: Option<UsageCost>,
}

/// Estimated cost breakdown in USD.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UsageCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl Usage {
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
        self.cost = Some(UsageCost {
            input: input_c,
            output: output_c,
            cache_read: cache_read_c,
            cache_write: cache_write_c,
            total: input_c + output_c + cache_read_c + cache_write_c,
        });
    }
}

// ── StopReason ──────────────────────────────────────────────────────

/// Why the assistant stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
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

// ── Convenience constructors ────────────────────────────────────────

impl AgentMessage {
    /// Create a simple user text message.
    pub fn user(text: impl Into<String>) -> Self {
        Self::UserMessage {
            content: vec![AgentPart::Text(text.into())],
            timestamp: now_ms(),
        }
    }

    /// Create a simple assistant text message.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::AssistantMessage {
            content: vec![AgentPart::Text(text.into())],
            stop_reason: Some(StopReason::Stop),
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

    /// Create a tool result message.
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

    /// Create a bash execution message.
    pub fn bash(
        command: impl Into<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self::BashExecutionMessage {
            command: command.into(),
            output: output.into(),
            exit_code,
            cancelled: false,
            truncated: false,
            exclude_from_context: false,
        }
    }
}

// ── LlmMessage conversion trait ─────────────────────────────────────

/// Trait for converting [`AgentMessage`] lists into provider-native request
/// message formats.
pub trait LlmMessageConverter: Send + Sync {
    type Output;

    fn convert_to_llm(
        messages: &[AgentMessage],
        system_prompt: Option<&str>,
    ) -> Self::Output;
}

/// Helper: collect text content from a slice of [`AgentPart`], skipping
/// non-text parts.
pub fn collect_text_parts(parts: &[AgentPart]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            AgentPart::Text(t) | AgentPart::Thinking { text: t, .. } => Some(t.as_str()),
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
        let msg = AgentMessage::AssistantMessage {
            content: vec![AgentPart::Text("response".into())],
            stop_reason: Some(StopReason::Stop),
            usage: Some(Usage {
                input: 100,
                output: 50,
                cache_read: 0,
                cache_write: 0,
                cache_write_1h: 0,
                total_tokens: 150,
                cost: Some(UsageCost {
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
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role_name(), "assistant");
        assert_eq!(deserialized.text(), "response");
    }

    #[test]
    fn assistant_error_message() {
        let msg = AgentMessage::AssistantMessage {
            content: vec![],
            stop_reason: Some(StopReason::Error),
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
        };
        assert!(msg.is_error());
        assert_eq!(
            msg.text(),
            ""
        );
    }

    #[test]
    fn tool_result_message_has_tool_name() {
        let msg = AgentMessage::ToolResultMessage {
            tool_use_id: "call-1".into(),
            tool_name: "read_file".into(),
            content: vec![AgentPart::Text("file contents".into())],
            details: Some(serde_json::json!({"path": "src/main.rs", "lines": 42})),
            is_error: false,
            timestamp: 1000,
        };
        assert_eq!(msg.role_name(), "toolResult");
    }

    #[test]
    fn usage_with_cost() {
        let mut usage = Usage {
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
            text: "Let me reason...".into(),
            redacted: false,
            signature: Some("sig-abc".into()),
        };
        let json = serde_json::to_string(&part).unwrap();
        let deserialized: AgentPart = serde_json::from_str(&json).unwrap();
        match deserialized {
            AgentPart::Thinking { text, redacted, signature } => {
                assert_eq!(text, "Let me reason...");
                assert!(!redacted);
                assert_eq!(signature, Some("sig-abc".into()));
            }
            _ => panic!("expected Thinking"),
        }
    }

    #[test]
    fn all_seven_roles_serialize_and_deserialize() {
        let messages = vec![
            AgentMessage::user("hi"),
            AgentMessage::assistant("hello"),
            AgentMessage::tool_result("t1", "read_file", vec![AgentPart::Text("done".into())], false),
            AgentMessage::bash("pwd", "/home", Some(0)),
            AgentMessage::CustomMessage {
                custom_type: "x".into(),
                content: serde_json::json!({}),
                display: serde_json::json!({}),
                details: serde_json::json!({}),
            },
            AgentMessage::CompactionSummaryMessage {
                summary: "s".into(),
                tokens_before: 10,
                tokens_after: 2,
                read_files: None,
                modified_files: None,
            },
            AgentMessage::BranchSummaryMessage {
                summary: "s".into(),
                from_id: "e-1".into(),
            },
        ];

        for msg in messages {
            let json = serde_json::to_string(&msg).unwrap();
            let deserialized: AgentMessage = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.role_name(), msg.role_name());
        }
    }
}
