//! Typed lifecycle events for the agent session.
//!
//! Pure vocabulary: every phase of the agent lifecycle emits a typed event
//! with a meaningful payload. Sinks are wired via the [`crate::XyEventSink`]
//! port; `infra::event` holds an in-process sink implementation. This module
//! holds only the event enum and its
//! handler type alias so both `agent` and `infra` can reference them without
//! a cross-layer reach. Zero crate-internal deps beyond `protocol::message`
//! and `protocol::session` (Todo SSOT vocabulary for state-projection events).
//!
//! **Closed set:** `XyEvent` is the agent lifecycle vocabulary (Agent/Turn/Message/
//! Tool/Compaction/Queue…), not a dump of provider SSE names. Provider streams map
//! to [`crate::protocol::model::XyChunk`] in adapters, then the ReAct loop emits
//! standard `XyEvent`s. See
//! `llmanspec/changes/archive/2026-07-11-c520-update-xy-event-extensibility/design.md`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::IntoStaticStr;

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
    pub kind: String,
    pub message: String,
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

    pub fn is_aborted(&self) -> bool {
        self.kind == "Aborted" || self.message == "aborted"
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
///
/// `IntoStaticStr` (snake_case) feeds [`Self::description`] — stable log/obs/
/// topic keys, **deliberately distinct** from the camelCase wire tag.
#[derive(Debug, Clone, Serialize, Deserialize, IntoStaticStr)]
#[serde(tag = "type", rename_all = "camelCase")]
#[strum(serialize_all = "snake_case")]
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
        /// AfterCompaction settlement tokens (c26 same-source value) when compact
        /// succeeded — powers the `Compacted from N → M tokens` word form and the
        /// c28 floor diagnostic. Live-only; never persisted on CompactionEntry.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tokens_after: Option<u64>,
        /// One-shot actionable diagnostic (c28): post-compact projection still ≥
        /// window. At most once per session; manual compacts never carry it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notice: Option<String>,
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

    // ── Domain state projections ─────────────────────────────────
    /// Agent Todo SSOT changed (atd13): full latest-wins snapshot pushed from
    /// the mutation point. Clients project the checklist from this event —
    /// they MUST NOT re-parse tool-result strings. Empty list = cleared.
    /// Not a cold-replay tape: resume rebuilds from `agent_todo` snapshots.
    TodoUpdated {
        list: crate::protocol::session::TodoList,
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
        Self::Error(XyEventError::new("Message", message))
    }

    /// A short human-readable description of the event.
    pub fn description(&self) -> &'static str {
        self.into()
    }
}
