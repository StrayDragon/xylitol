//! Typed bridge message types — serde-compatible with domain AgentMessage.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "camelCase")]
pub enum AiBridgeMessage {
    #[serde(rename = "user")]
    UserMessage {
        content: Vec<AiBridgePart>,
        #[serde(default = "now_ms")]
        timestamp: u64,
    },
    #[serde(rename = "assistant")]
    AssistantMessage {
        content: Vec<AiBridgePart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop_reason: Option<AiBridgeStopReason>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<AiBridgeUsage>,
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
        content: Vec<AiBridgePart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
        #[serde(default)]
        is_error: bool,
        #[serde(default = "now_ms")]
        timestamp: u64,
    },
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

impl AiBridgeMessage {
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

    pub fn content(&self) -> &[AiBridgePart] {
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

    pub fn text(&self) -> String {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => collect_text_parts(content),
            Self::BashExecutionMessage {
                command, output, ..
            } => format!("$ {command}\n{output}"),
            Self::CustomMessage { content, .. } => content.as_str().unwrap_or("").to_string(),
            Self::CompactionSummaryMessage { summary, .. } => summary.clone(),
            Self::BranchSummaryMessage { summary, .. } => summary.clone(),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::AssistantMessage {
                stop_reason: Some(AiBridgeStopReason::Error | AiBridgeStopReason::Aborted),
                ..
            }
        )
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::UserMessage {
            content: vec![AiBridgePart::text(text)],
            timestamp: now_ms(),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::AssistantMessage {
            content: vec![AiBridgePart::text(text)],
            stop_reason: Some(AiBridgeStopReason::Stop),
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
        content: Vec<AiBridgePart>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AiBridgePart {
    Text {
        text: String,
    },
    Image(AiBridgeImageContent),
    Thinking {
        thinking: String,
        #[serde(default)]
        redacted: bool,
        #[serde(
            default,
            rename = "thinkingSignature",
            skip_serializing_if = "Option::is_none"
        )]
        thinking_signature: Option<String>,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
}

impl AiBridgePart {
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text { text: s.into() }
    }

    pub fn thinking(s: impl Into<String>) -> Self {
        Self::Thinking {
            thinking: s.into(),
            redacted: false,
            thinking_signature: None,
        }
    }

    pub fn is_tool_call(&self) -> bool {
        matches!(self, Self::ToolCall { .. })
    }

    pub fn is_text_content(&self) -> bool {
        matches!(self, Self::Text { .. } | Self::Thinking { .. })
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text.as_str()),
            Self::Thinking { thinking, .. } => Some(thinking.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBridgeImageContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(rename = "mimeType")]
    pub media_type: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AiBridgeUsage {
    pub input: u64,
    pub output: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_write: u64,
    #[serde(default)]
    pub cache_write_1h: u64,
    #[serde(skip)]
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<AiBridgeUsageCost>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AiBridgeUsageCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl AiBridgeUsage {
    pub fn compute_total(&mut self) {
        self.total_tokens = self.input + self.output;
    }

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
        self.cost = Some(AiBridgeUsageCost {
            input: input_c,
            output: output_c,
            cache_read: cache_read_c,
            cache_write: cache_write_c,
            total: input_c + output_c + cache_read_c + cache_write_c,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiBridgeStopReason {
    Stop,
    #[serde(rename = "length")]
    MaxTokens,
    Error,
    Aborted,
    ToolUse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub fn collect_text_parts(parts: &[AiBridgePart]) -> String {
    parts
        .iter()
        .filter_map(|p| match p {
            AiBridgePart::Text { text } | AiBridgePart::Thinking { thinking: text, .. } => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
