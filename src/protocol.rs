//! Protocol — the single source of truth for client↔core interaction.
//!
//! Defines the command (client → core) and event (core → client) vocabularies.
//! Transport-agnostic: the same `Command`/`Event` types are spoken over the
//! stdio RPC transport today and over WebSocket/REST once the server lands.
//!
//! Wire format is stable: serde `tag = "type"` + `snake_case` variants. Adding
//! a command/event = adding a variant; unknown variants are tolerated by serde
//! defaults on the receiver side.
//!
//! NOTE: envelope/error-code typing and a REST `{code,msg,data,request_id}`
//! envelope are deferred until the server (P5 server phase) needs them; the
//! stdio transport currently serializes `Event` lines directly.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Command (client → core) ────────────────────────────────────────

/// A command from the client. Each carries an optional `id` for correlation.
#[derive(Debug, Deserialize)]
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
            | Command::Quit { id } => id.as_deref(),
        }
    }
}

// ── Event (core → client) ──────────────────────────────────────────

/// An event from the core: either a response to a command or a streamed
/// occurrence during a turn.
#[derive(Debug, Serialize)]
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
    BashResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        output: String,
        exit_code: Option<i32>,
        cancelled: bool,
        truncated: bool,
    },
}
