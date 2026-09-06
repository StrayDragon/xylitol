//! Command vocabulary — client → core messages.

use serde::{Deserialize, Serialize};

use crate::protocol::session::SessionTreeKind;

/// A command from the client (c2530 后继：传输级关联唯一走信封 rpcId，
/// `id` 字段作为无消费者的逻辑死码已移除)。
#[derive(Debug, Clone, Serialize, Deserialize, strum::VariantNames, strum::IntoStaticStr)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Prompt {
        message: String,
    },
    Abort {},
    GetState {},
    SetModel {
        /// Hand parse accepted a missing `provider` as empty string; keep that
        /// wire shape accept-able under serde parsing (c2530).
        #[serde(default)]
        provider: String,
        model_id: String,
    },
    CycleModel {},
    GetAvailableModels {},
    SetThinkingLevel {
        level: String,
    },
    Bash {
        command: String,
        #[serde(default)]
        exclude_from_context: bool,
    },
    Compact {
        /// Optional focus text for summarization (c1670; pi `customInstructions`).
        #[serde(default)]
        instructions: Option<String>,
    },
    GetSessionStats {},
    ExportHtml {
        #[serde(default, alias = "path")]
        output_path: Option<String>,
    },
    ExportJsonl {
        #[serde(default, alias = "path")]
        output_path: Option<String>,
    },
    ImportJsonl {
        #[serde(alias = "path")]
        input_path: String,
    },
    SwitchSession {
        #[serde(alias = "session_id")]
        session_path: String,
    },
    Fork {
        entry_id: String,
        /// `"before"` | `"at"` (default `"at"`).
        #[serde(default)]
        position: Option<String>,
    },
    GetMessages {},
    GetCommands {},
    SessionTree {
        /// Hand parse defaulted a missing `kind` to message history (c2530).
        #[serde(default = "default_session_tree_kind")]
        kind: SessionTreeKind,
    },
    TravelSessionTree {
        kind: SessionTreeKind,
        entry_id: String,
    },
    AppendEntryLabel {
        target_id: String,
        #[serde(default)]
        label: Option<String>,
    },
    ListSessions {},
    LoadSessionEntries {
        session_id: String,
    },
    NewSession {},
    GetSessionName {},
    SetSessionName {
        name: String,
    },
    SetSessionNameFor {
        session_id: String,
        name: String,
    },
    DeleteSession {
        session_id: String,
    },
    Reload {},
    LoadedResources {},
    /// Wire method is `queue_stats`; the alias keeps serde tag parsing
    /// accept-able for it (c2530).
    #[serde(alias = "queue_stats")]
    GetQueueStats {},
    Steer {
        message: String,
    },
    FollowUp {
        message: String,
    },
    ClearQueue {
        #[serde(default = "default_true")]
        clear_steer: bool,
        #[serde(default = "default_true")]
        clear_follow_up: bool,
    },
    /// Subscribe to a session's event stream (WebSocket).
    Subscribe {
        session_id: String,
        last_seq: u64,
    },
    /// Approve a tool execution (reverse RPC response).
    ApproveTool {
        call_id: String,
        approved: bool,
    },
    /// Answer a user question (reverse RPC response).
    AnswerQuestion {
        call_id: String,
        answer: String,
    },
    Quit {},
}

fn default_true() -> bool {
    true
}

fn default_session_tree_kind() -> SessionTreeKind {
    SessionTreeKind::MessageHistory
}
