use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{EnumString, IntoStaticStr};

use super::message::{AiBridgeStopReason, AiBridgeUsage};
use crate::error::AiBridgeError;

/// A single chunk from a streaming LLM response.
///
/// Tool calls use a start/delta/end lifecycle (aligned with pi
/// `toolcall_start|delta|end`). Adapters MUST emit [`Self::ToolCallStart`]
/// when a tool call is first identifiable, incremental
/// [`Self::ToolCallDelta`] while arguments stream, and [`Self::ToolCallEnd`]
/// when the call is complete — not only a single terminal event at stream end.
#[derive(Debug, Clone)]
pub enum AiBridgeChunk {
    TextDelta(String),
    ThinkingDelta(String),
    /// Reasoning item finalized (pi `thinking_end`); signature is opaque JSON for replay.
    ThinkingEnd {
        thinking: String,
        thinking_signature: Option<String>,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        name: String,
        /// Raw JSON fragment appended this step (may be empty if only metadata updated).
        args_delta: String,
        /// Best-effort parse of the accumulated arguments so far.
        args: Value,
    },
    ToolCallEnd {
        id: String,
        name: String,
        args: Value,
    },
    Done {
        finish_reason: AiBridgeStopReason,
        usage: Option<AiBridgeUsage>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct AiBridgeToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

pub type AiBridgeStream = Pin<Box<dyn Stream<Item = Result<AiBridgeChunk, AiBridgeError>> + Send>>;

/// Provenance of a context-token estimate.
///
/// Observation / wire keys are PascalCase (`as_str` / `from_key`) via strum.
/// Serde JSON stays snake_case and is a separate form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "PascalCase")]
pub enum TokenProvenance {
    Api,
    Heuristic,
    Unknown,
}

impl TokenProvenance {
    /// Stable PascalCase key used by observation and wire projection code.
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    /// Parse an observation / wire key. Retired or unknown names become [`Self::Unknown`].
    pub fn from_key(s: &str) -> Self {
        s.parse().unwrap_or(Self::Unknown)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_key_is_pascal_case_strum_ssot() {
        assert_eq!(TokenProvenance::Api.as_str(), "Api");
        assert_eq!(TokenProvenance::Heuristic.as_str(), "Heuristic");
        assert_eq!(TokenProvenance::Unknown.as_str(), "Unknown");
        assert_eq!(TokenProvenance::from_key("Api"), TokenProvenance::Api);
        assert_eq!(
            TokenProvenance::from_key("Heuristic"),
            TokenProvenance::Heuristic
        );
        assert_eq!(
            TokenProvenance::from_key("LocalTokenizer"),
            TokenProvenance::Unknown
        );
        assert_eq!(
            TokenProvenance::from_key("RemoteCount"),
            TokenProvenance::Unknown
        );
        assert_eq!(
            serde_json::to_value(TokenProvenance::Api).unwrap(),
            serde_json::json!("api")
        );
    }
}
