//! Typed lifecycle events for the agent — aligns with pi's AgentSessionEvent.
//!
//! Pure vocabulary: every phase of the agent lifecycle emits a typed event
//! with a meaningful payload. The `EventBus` runtime (dispatch/subscription)
//! lives in `infra::event`; this module holds only the event enum and its
//! handler type alias so both `agent` and `infra` can reference them without
//! a cross-layer reach. Zero crate-internal deps beyond `domain::message`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── AgentLifecycleEvent ─────────────────────────────────────────────

/// All possible lifecycle events emitted during agent execution.
///
/// Each variant carries a typed payload — no generic `Value` here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentLifecycleEvent {
    // ── Agent lifecycle ──────────────────────────────────────────
    AgentStart {
        session_id: String,
        model: String,
    },
    AgentEnd {
        session_id: String,
        reason: String,
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
        message: Option<crate::domain::message::AgentMessage>,
    },
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        /// Partial agent message with current streaming state.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<crate::domain::message::AgentMessage>,
    },
    MessageEnd {
        role: String,
        /// Complete agent message after streaming finishes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<crate::domain::message::AgentMessage>,
    },

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

    // ── Session info ─────────────────────────────────────────────
    SessionInfoChanged {
        key: String,
        value: Value,
    },
}

impl AgentLifecycleEvent {
    /// A short human-readable description of the event.
    pub fn description(&self) -> &'static str {
        match self {
            Self::AgentStart { .. } => "agent_start",
            Self::AgentEnd { .. } => "agent_end",
            Self::TurnStart { .. } => "turn_start",
            Self::TurnEnd { .. } => "turn_end",
            Self::MessageStart { .. } => "message_start",
            Self::MessageUpdate { .. } => "message_update",
            Self::MessageEnd { .. } => "message_end",
            Self::ToolExecutionStart { .. } => "tool_execution_start",
            Self::ToolExecutionUpdate { .. } => "tool_execution_update",
            Self::ToolExecutionEnd { .. } => "tool_execution_end",
            Self::CompactionStart { .. } => "compaction_start",
            Self::CompactionEnd { .. } => "compaction_end",
            Self::ModelSelect { .. } => "model_select",
            Self::ThinkingLevelChanged { .. } => "thinking_level_changed",
            Self::QueueUpdate { .. } => "queue_update",
            Self::AutoRetryStart { .. } => "auto_retry_start",
            Self::AutoRetryEnd { .. } => "auto_retry_end",
            Self::SessionInfoChanged { .. } => "session_info_changed",
        }
    }
}
