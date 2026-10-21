use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        kind: String,
        message: String,
    },
    TextDelta {
        text: String,
    },
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
    TurnStart {
        turn_index: u32,
    },
    TurnEnd {
        turn_index: u32,
    },
    MessageStart {
        role: String,
        message: Option<Value>,
    },
    MessageEnd {
        role: String,
        message: Option<Value>,
    },
    MessageUpdate {
        text: String,
        thinking: Option<String>,
        message: Option<Value>,
    },
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
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
    ContextTokenSettlement {
        tokens: u64,
        provenance: String,
        usage_tokens: u64,
        trailing_tokens: u64,
        last_usage_index: Option<usize>,
        reason: String,
        generation: u64,
    },
    QueueUpdate {
        steer_count: usize,
        follow_up_count: usize,
    },
    TodoUpdated {
        list: crate::TodoList,
    },
}
