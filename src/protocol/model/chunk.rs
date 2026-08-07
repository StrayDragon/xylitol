//! Streaming chunks and tool schemas.

use serde::Serialize;
use serde_json::Value;

use crate::protocol::message::{XyStopReason, XyUsage};

// ── Streaming Chunk ──────────────────────────────────────────────

/// A single chunk from a streaming LLM response.
#[derive(Debug, Clone)]
pub enum XyChunk {
    TextDelta(String),
    ThinkingDelta(String),
    /// Reasoning finalized with optional opaque signature for Responses replay (c1290).
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
        args_delta: String,
        args: Value,
    },
    ToolCallEnd {
        id: String,
        name: String,
        args: Value,
    },
    Done {
        finish_reason: XyStopReason,
        usage: Option<XyUsage>,
    },
}

/// JSON schema describing a tool's parameters.
#[derive(Debug, Clone, Serialize)]
pub struct XyToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
