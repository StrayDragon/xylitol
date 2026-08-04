//! Typed bridge LLM message types (c1070: session roles live only on AgentMessage).
//!
//! Variants are limited to model-visible roles: user / assistant / toolResult.
//! Environment folding (bash, compact, branch, custom) happens in the main crate
//! via `domain::llm_project::project_for_llm` before crossing this boundary.

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
    /// Wire fields camelCase (`stopReason`, `errorMessage`); snake aliases read pre-fix JSONL.
    #[serde(rename = "assistant")]
    #[serde(rename_all = "camelCase")]
    AssistantMessage {
        content: Vec<AiBridgePart>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "stop_reason"
        )]
        stop_reason: Option<AiBridgeStopReason>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<AiBridgeUsage>,
        #[serde(default)]
        api: String,
        #[serde(default)]
        provider: String,
        #[serde(default)]
        model: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "response_id"
        )]
        response_id: Option<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            alias = "error_message"
        )]
        error_message: Option<String>,
        #[serde(default = "now_ms")]
        timestamp: u64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<Diagnostic>,
    },
    #[serde(rename = "toolResult")]
    #[serde(rename_all = "camelCase")]
    ToolResultMessage {
        #[serde(rename = "toolCallId", alias = "tool_use_id")]
        tool_use_id: String,
        #[serde(default, alias = "tool_name")]
        tool_name: String,
        content: Vec<AiBridgePart>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        details: Option<Value>,
        #[serde(default, alias = "is_error")]
        is_error: bool,
        #[serde(default = "now_ms")]
        timestamp: u64,
    },
}

impl AiBridgeMessage {
    pub fn role_name(&self) -> &'static str {
        match self {
            Self::UserMessage { .. } => "user",
            Self::AssistantMessage { .. } => "assistant",
            Self::ToolResultMessage { .. } => "toolResult",
        }
    }

    pub fn content(&self) -> &[AiBridgePart] {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => content,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Self::UserMessage { content, .. }
            | Self::AssistantMessage { content, .. }
            | Self::ToolResultMessage { content, .. } => collect_text_parts(content),
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

    /// User message with arbitrary parts (text + images).
    pub fn user_parts(content: Vec<AiBridgePart>) -> Self {
        Self::UserMessage {
            content,
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

    /// Inline image part (base64 `data` + MIME).
    pub fn image(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image(AiBridgeImageContent {
            url: None,
            data: Some(data.into()),
            media_type: media_type.into(),
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCacheRead {
    /// WirePolicy does not expect first-language cache usage fields.
    #[default]
    NotApplicable,
    /// Expected, but the response omitted cache detail fields.
    NotReported,
    /// Explicit token count from the provider (including zero).
    Tokens(u64),
}

impl PromptCacheRead {
    /// Derived `cache_read` tokens for accounting (`Tokens(n)` only).
    pub fn tokens(self) -> u64 {
        match self {
            Self::Tokens(n) => n,
            Self::NotApplicable | Self::NotReported => 0,
        }
    }

    /// Stable observation label (`xylitol.prompt_cache_read`).
    pub fn as_status_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "not_applicable",
            Self::NotReported => "not_reported",
            Self::Tokens(_) => "tokens",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBridgeUsage {
    pub input: u64,
    pub output: u64,
    #[serde(default, alias = "cache_read")]
    pub cache_read: u64,
    #[serde(default, alias = "cache_write")]
    pub cache_write: u64,
    #[serde(default, alias = "cache_write_1h")]
    pub cache_write_1h: u64,
    #[serde(skip)]
    pub total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<AiBridgeUsageCost>,
    /// Authoritative Prompt Cache read provenance (c1885). `cache_read` is derived.
    #[serde(default)]
    pub prompt_cache_read: PromptCacheRead,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiBridgeUsageCost {
    pub input: f64,
    pub output: f64,
    #[serde(alias = "cache_read")]
    pub cache_read: f64,
    #[serde(alias = "cache_write")]
    pub cache_write: f64,
    pub total: f64,
}

impl Default for AiBridgeUsage {
    fn default() -> Self {
        Self {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 0,
            cost: None,
            prompt_cache_read: PromptCacheRead::NotApplicable,
        }
    }
}

impl AiBridgeUsage {
    /// Build usage with authoritative prompt-cache provenance; syncs `cache_read`.
    pub fn with_prompt_cache_read(mut self, prompt_cache_read: PromptCacheRead) -> Self {
        self.prompt_cache_read = prompt_cache_read;
        self.cache_read = prompt_cache_read.tokens();
        self
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn assistant_wire_uses_camel_case_stop_reason() {
        let msg = AiBridgeMessage::AssistantMessage {
            content: vec![AiBridgePart::text("")],
            stop_reason: Some(AiBridgeStopReason::Error),
            usage: Some(AiBridgeUsage {
                input: 1,
                output: 2,
                cache_read: 3,
                cache_write: 4,
                cache_write_1h: 5,
                total_tokens: 0,
                ..Default::default()
            }),
            api: String::new(),
            provider: "openai".into(),
            model: "m".into(),
            response_id: Some("r1".into()),
            error_message: Some("boom".into()),
            timestamp: 1,
            diagnostics: Vec::new(),
        };
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["stopReason"], "error");
        assert_eq!(v["errorMessage"], "boom");
        assert_eq!(v["responseId"], "r1");
        assert_eq!(v["usage"]["cacheRead"], 3);
        assert!(v.get("stop_reason").is_none());
        assert!(v.get("error_message").is_none());
    }

    #[test]
    fn assistant_deserializes_legacy_snake_case() {
        let v = json!({
            "role": "assistant",
            "content": [{"type": "text", "text": ""}],
            "stop_reason": "error",
            "error_message": "legacy",
            "timestamp": 1u64,
        });
        let msg: AiBridgeMessage = serde_json::from_value(v).unwrap();
        match msg {
            AiBridgeMessage::AssistantMessage {
                stop_reason: Some(AiBridgeStopReason::Error),
                error_message: Some(m),
                ..
            } => assert_eq!(m, "legacy"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn tool_result_wire_uses_camel_case() {
        let msg = AiBridgeMessage::tool_result("c1", "bash", vec![AiBridgePart::text("ok")], true);
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["toolCallId"], "c1");
        assert_eq!(v["toolName"], "bash");
        assert_eq!(v["isError"], true);
        assert!(v.get("tool_name").is_none());
        assert!(v.get("is_error").is_none());
    }
}
