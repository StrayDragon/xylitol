//! Typed lifecycle events for the agent — aligns with pi's AgentSessionEvent.
//!
//! Pure vocabulary: every phase of the agent lifecycle emits a typed event
//! with a meaningful payload. The `EventBus` runtime (dispatch/subscription)
//! lives in `infra::event`; this module holds only the event enum and its
//! handler type alias so both `agent` and `infra` can reference them without
//! a cross-layer reach. Zero crate-internal deps beyond `protocol::message`.
//!
//! **Closed set:** `XyEvent` is the agent lifecycle vocabulary (Agent/Turn/Message/
//! Tool/Compaction/Queue…), not a dump of provider SSE names. Provider streams map
//! to [`crate::protocol::types::XyChunk`] in adapters, then the ReAct loop emits
//! standard `XyEvent`s. See
//! `llmanspec/changes/archive/2026-07-11-c520-update-xy-event-extensibility/design.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::message::AgentMessage;

// ── XyEvent ─────────────────────────────────────────────

/// All possible events emitted during agent execution.
///
/// Each variant carries a typed payload — no generic `Value` here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum XyEvent {
    // ── Agent lifecycle ──────────────────────────────────────────
    AgentStart {
        session_id: String,
        model: String,
    },
    AgentEnd {
        messages: Vec<AgentMessage>,
    },

    // ── Turn lifecycle ───────────────────────────────────────────
    TurnStart {
        turn_index: u32,
    },
    TurnEnd {
        turn_index: u32,
    },

    // ── Message lifecycle ────────────────────────────────────────
    MessageStart {
        role: String,
        /// Full agent message, when available.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        /// Partial agent message with current streaming state.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    MessageEnd {
        role: String,
        /// Complete agent message after streaming finishes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<AgentMessage>,
    },
    TextDelta(String),
    ThinkingDelta(String),

    // ── Tool execution ───────────────────────────────────────────
    ToolExecutionStart {
        id: String,
        name: String,
        args: Value,
    },
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    ToolExecutionEnd {
        id: String,
        name: String,
        result: String,
        #[serde(default)]
        is_error: bool,
    },

    // ── Compaction ───────────────────────────────────────────────
    CompactionStart {
        reason: String,
    },
    CompactionEnd {
        result: Option<String>,
        #[serde(default)]
        aborted: bool,
        /// `manual` | `threshold` | `overflow` (empty when legacy emitters omit).
        #[serde(default)]
        reason: String,
        #[serde(default)]
        will_retry: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error_message: Option<String>,
    },

    // ── Model and settings ───────────────────────────────────────
    ModelSelect {
        provider: String,
        model_id: String,
    },
    ThinkingLevelChanged {
        level: String,
    },

    // ── Queue ────────────────────────────────────────────────────
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },

    // ── Auto-retry ───────────────────────────────────────────────
    AutoRetryStart {
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
    },
    AutoRetryEnd {
        success: bool,
        attempt: u32,
    },

    // ── Error ────────────────────────────────────────────────────
    Error(String),

    // ── Session info ─────────────────────────────────────────────
    SessionInfoChanged {
        key: String,
        value: Value,
    },
}

impl XyEvent {
    /// A short human-readable description of the event.
    pub fn description(&self) -> &'static str {
        match self {
            XyEvent::AgentStart { .. } => "agent_start",
            XyEvent::AgentEnd { .. } => "agent_end",
            XyEvent::TurnStart { .. } => "turn_start",
            XyEvent::TurnEnd { .. } => "turn_end",
            XyEvent::MessageStart { .. } => "message_start",
            XyEvent::MessageUpdate { .. } => "message_update",
            XyEvent::MessageEnd { .. } => "message_end",
            XyEvent::TextDelta(_) => "text_delta",
            XyEvent::ThinkingDelta(_) => "thinking_delta",
            XyEvent::ToolExecutionStart { .. } => "tool_execution_start",
            XyEvent::ToolExecutionUpdate { .. } => "tool_execution_update",
            XyEvent::ToolExecutionEnd { .. } => "tool_execution_end",
            XyEvent::CompactionStart { .. } => "compaction_start",
            XyEvent::CompactionEnd { .. } => "compaction_end",
            XyEvent::ModelSelect { .. } => "model_select",
            XyEvent::ThinkingLevelChanged { .. } => "thinking_level_changed",
            XyEvent::QueueUpdate { .. } => "queue_update",
            XyEvent::AutoRetryStart { .. } => "auto_retry_start",
            XyEvent::AutoRetryEnd { .. } => "auto_retry_end",
            XyEvent::SessionInfoChanged { .. } => "session_info_changed",
            XyEvent::Error(_) => "error",
        }
    }
}
