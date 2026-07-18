use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use serde_json::Value;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProvenance {
    Api,
    RemoteCount,
    LocalTokenizer,
    Heuristic,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}
