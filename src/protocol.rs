//! Protocol — the single source of truth for client↔core interaction.
//!
//! Defines the command (client → core) and event (core → client) vocabularies.
//! Transport-agnostic: the same `Command`/`Event` types are spoken over the
//! stdio RPC transport, WebSocket, and REST.
//!
//! Wire format is stable: serde `tag = "type"` + `snake_case` variants. Adding
//! a command/event = adding a variant; unknown variants are tolerated by serde
//! defaults on the receiver side.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Command (client → core) ────────────────────────────────────────

/// A command from the client. Each carries an optional `id` for correlation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Prompt {
        #[serde(default)]
        id: Option<String>,
        message: String,
    },
    Abort {
        #[serde(default)]
        id: Option<String>,
    },
    GetState {
        #[serde(default)]
        id: Option<String>,
    },
    SetModel {
        #[serde(default)]
        id: Option<String>,
        provider: String,
        model_id: String,
    },
    CycleModel {
        #[serde(default)]
        id: Option<String>,
    },
    GetAvailableModels {
        #[serde(default)]
        id: Option<String>,
    },
    SetThinkingLevel {
        #[serde(default)]
        id: Option<String>,
        level: String,
    },
    Bash {
        #[serde(default)]
        id: Option<String>,
        command: String,
        #[serde(default)]
        exclude_from_context: bool,
    },
    Compact {
        #[serde(default)]
        id: Option<String>,
    },
    GetSessionStats {
        #[serde(default)]
        id: Option<String>,
    },
    ExportHtml {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        output_path: Option<String>,
    },
    SwitchSession {
        #[serde(default)]
        id: Option<String>,
        session_path: String,
    },
    Fork {
        #[serde(default)]
        id: Option<String>,
        entry_id: String,
    },
    GetMessages {
        #[serde(default)]
        id: Option<String>,
    },
    GetCommands {
        #[serde(default)]
        id: Option<String>,
    },
    /// Subscribe to a session's event stream (WebSocket).
    Subscribe {
        #[serde(default)]
        id: Option<String>,
        session_id: String,
        last_seq: u64,
    },
    /// Approve a tool execution (reverse RPC response).
    ApproveTool {
        #[serde(default)]
        id: Option<String>,
        call_id: String,
        approved: bool,
    },
    /// Answer a user question (reverse RPC response).
    AnswerQuestion {
        #[serde(default)]
        id: Option<String>,
        call_id: String,
        answer: String,
    },
    Quit {
        #[serde(default)]
        id: Option<String>,
    },
}

impl Command {
    /// The correlation id, if any.
    pub fn id(&self) -> Option<&str> {
        match self {
            Command::Prompt { id, .. }
            | Command::Abort { id }
            | Command::GetState { id }
            | Command::SetModel { id, .. }
            | Command::CycleModel { id }
            | Command::GetAvailableModels { id }
            | Command::SetThinkingLevel { id, .. }
            | Command::Bash { id, .. }
            | Command::Compact { id }
            | Command::GetSessionStats { id }
            | Command::ExportHtml { id, .. }
            | Command::SwitchSession { id, .. }
            | Command::Fork { id, .. }
            | Command::GetMessages { id }
            | Command::GetCommands { id }
            | Command::Subscribe { id, .. }
            | Command::ApproveTool { id, .. }
            | Command::AnswerQuestion { id, .. }
            | Command::Quit { id } => id.as_deref(),
        }
    }
}

// ── Event (core → client) ──────────────────────────────────────────

/// An event from the core: either a response to a command or a streamed
/// occurrence during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        message: String,
    },
    Response {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    TextDelta {
        text: String,
    },
    ToolStart {
        id: String,
        name: String,
    },
    ToolEnd {
        id: String,
        name: String,
        result: String,
    },
    AgentEnd,
    ModelSelect {
        provider: String,
        model_id: String,
    },
    CompactionStart {
        reason: String,
    },
    /// Acknowledgment of a Subscribe command.
    Subscribed {
        session_id: String,
        seq: u64,
    },
    BashResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        output: String,
        exit_code: Option<i32>,
        cancelled: bool,
        truncated: bool,
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
    },
    /// Message ended.
    MessageEnd {
        role: String,
    },
    /// Streaming message update (replaces previous text/thinking for this message).
    MessageUpdate {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
    },
    /// Streaming tool execution output.
    ToolExecutionUpdate {
        id: String,
        output: String,
    },
    /// Compaction completed.
    CompactionEnd,
}

// ── REST envelope types ────────────────────────────────────────────

/// Error codes for REST envelope responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ErrorCode {
    #[default]
    Ok,
    BadRequest,
    NotFound,
    ServerLocked,
    SessionNotFound,
    InternalError,
    Timeout,
}

/// Uniform REST response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T: Serialize> {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl<T: Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: ErrorCode::Ok,
            msg: None,
            data: Some(data),
            request_id: None,
        }
    }

    pub fn error(code: ErrorCode, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: Some(msg.into()),
            data: None,
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, id: Option<String>) -> Self {
        self.request_id = id;
        self
    }
}
