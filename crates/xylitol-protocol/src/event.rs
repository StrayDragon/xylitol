//! Event vocabulary — core → client messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An event from the core: a streamed occurrence during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        /// Stable classification aligned with Host lifecycle kinds.
        kind: String,
        message: String,
    },
    TextDelta {
        text: String,
    },
    /// Streaming thinking / reasoning text (e.g. Anthropic extended thinking).
    ThinkingDelta {
        text: String,
    },
    ToolStart {
        id: String,
        name: String,
        args: Value,
    },
    ToolEnd {
        id: String,
        name: String,
        result: String,
        /// Terminal failure marker (c2440); required on the wire.
        is_error: bool,
    },
    AgentEnd,
    ModelSelect {
        provider: String,
        model_id: String,
    },
    CompactionStart {
        reason: String,
    },
    /// Turn started.
    TurnStart {
        turn_index: u32,
    },
    /// Turn ended.
    TurnEnd {
        turn_index: u32,
    },
    /// Message started.
    MessageStart {
        role: String,
        message: Option<Value>,
    },
    /// Message ended.
    MessageEnd {
        role: String,
        message: Option<Value>,
    },
    /// Streaming message update (replaces previous text/thinking for this message).
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        message: Option<Value>,
    },
    /// Streaming tool execution output.
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    /// Compaction completed — payload so attach clients render real token
    /// counts / expandable summary (pa-wire3).
    CompactionEnd {
        result: Option<String>,
        #[serde(default)]
        aborted: bool,
        #[serde(default)]
        reason: String,
        #[serde(default)]
        will_retry: bool,
        error_message: Option<String>,
        summary: Option<String>,
        tokens_before: Option<u64>,
        #[serde(default)]
        tokens_after: Option<u64>,
        #[serde(default)]
        notice: Option<String>,
    },
    /// Shared context-token settlement (c1860) for footer / cross-client fixed zone.
    ContextTokenSettlement {
        tokens: u64,
        provenance: String,
        usage_tokens: u64,
        trailing_tokens: u64,
        last_usage_index: Option<usize>,
        reason: String,
        generation: u64,
    },
    /// Pending steer / follow-up queue depths (cross-client badge).
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },
    /// Agent Todo checklist snapshot changed (atd13 / pa-todo1): attach clients
    /// refresh the checklist projection from this payload, never from parsing
    /// tool-result text.
    TodoUpdated {
        list: crate::TodoList,
    },
}
