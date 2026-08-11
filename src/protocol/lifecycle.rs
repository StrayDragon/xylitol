//! Typed lifecycle events for the agent session.
//!
//! Pure vocabulary: every phase of the agent lifecycle emits a typed event
//! with a meaningful payload. The `EventBus` runtime (dispatch/subscription)
//! lives in `infra::event`; this module holds only the event enum and its
//! handler type alias so both `agent` and `infra` can reference them without
//! a cross-layer reach. Zero crate-internal deps beyond `protocol::message`.
//!
//! **Closed set:** `XyEvent` is the agent lifecycle vocabulary (Agent/Turn/Message/
//! Tool/Compaction/Queue…), not a dump of provider SSE names. Provider streams map
//! to [`crate::protocol::model::XyChunk`] in adapters, then the ReAct loop emits
//! standard `XyEvent`s. See
//! `llmanspec/changes/archive/2026-07-11-c520-update-xy-event-extensibility/design.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::error::XyError;
use crate::protocol::message::AgentMessage;
use crate::protocol::model::ContextTokenEstimate;

// ── XyEvent ─────────────────────────────────────────────

/// Structured agent-stream error (surfaces can branch on [`Self::kind`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XyEventError {
    /// Stable kind aligned with [`XyError::kind`] when sourced from hot-path errors
    /// (`Aborted`, `Provider`, `Session`, `Config`, …). Opaque strings use `Message`.
    #[serde(default = "default_event_error_kind")]
    pub kind: String,
    pub message: String,
}

fn default_event_error_kind() -> String {
    "Message".into()
}

impl XyEventError {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }

    pub fn aborted() -> Self {
        Self::new("Aborted", "aborted")
    }

    pub fn from_xy(err: &XyError) -> Self {
        Self::new(err.kind(), err.to_string())
    }

    /// Classify a bare message (legacy emitters / remote strings).
    pub fn message_only(message: impl Into<String>) -> Self {
        let message = message.into();
        if message == "aborted" {
            return Self::aborted();
        }
        Self::new("Message", message)
    }

    pub fn is_aborted(&self) -> bool {
        self.kind == "Aborted" || self.message == "aborted"
    }
}

impl From<String> for XyEventError {
    fn from(message: String) -> Self {
        Self::message_only(message)
    }
}

impl From<&str> for XyEventError {
    fn from(message: &str) -> Self {
        Self::message_only(message)
    }
}

impl From<&XyError> for XyEventError {
    fn from(err: &XyError) -> Self {
        Self::from_xy(err)
    }
}

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
        /// CompactionEntry.summary when compact succeeded (TUI block).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        /// CompactionEntry.tokens_before when compact succeeded.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_before: Option<u64>,
    },

    /// Shared context-token settlement (c1860) — compact + footer consume one snapshot.
    ///
    /// `reason` is a stable snake_case string (`turn_settled`, `after_compaction`, …).
    ContextTokenSettlement {
        estimate: ContextTokenEstimate,
        reason: String,
        generation: u64,
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
    Error(XyEventError),

    // ── Session info ─────────────────────────────────────────────
    SessionInfoChanged {
        key: String,
        value: Value,
    },
}

impl XyEvent {
    pub fn aborted() -> Self {
        Self::Error(XyEventError::aborted())
    }

    pub fn error_msg(message: impl Into<String>) -> Self {
        Self::Error(XyEventError::message_only(message))
    }

    pub fn error_xy(err: &XyError) -> Self {
        Self::Error(XyEventError::from_xy(err))
    }

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
            XyEvent::ContextTokenSettlement { .. } => "context_token_settlement",
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
