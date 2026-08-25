//! Command vocabulary — client → core messages.

use serde::{Deserialize, Serialize};

use crate::protocol::session::SessionTreeKind;

/// A command from the client. Each carries an optional `id` for correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        /// Optional focus text for summarization (c1670; pi `customInstructions`).
        #[serde(default)]
        instructions: Option<String>,
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
    ExportJsonl {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        output_path: Option<String>,
    },
    ImportJsonl {
        #[serde(default)]
        id: Option<String>,
        input_path: String,
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
        /// `"before"` | `"at"` (default `"at"`).
        #[serde(default)]
        position: Option<String>,
    },
    GetMessages {
        #[serde(default)]
        id: Option<String>,
    },
    GetCommands {
        #[serde(default)]
        id: Option<String>,
    },
    SessionTree {
        #[serde(default)]
        id: Option<String>,
        kind: SessionTreeKind,
    },
    TravelSessionTree {
        #[serde(default)]
        id: Option<String>,
        kind: SessionTreeKind,
        entry_id: String,
    },
    AppendEntryLabel {
        #[serde(default)]
        id: Option<String>,
        target_id: String,
        #[serde(default)]
        label: Option<String>,
    },
    ListSessions {
        #[serde(default)]
        id: Option<String>,
    },
    LoadSessionEntries {
        #[serde(default)]
        id: Option<String>,
        session_id: String,
    },
    NewSession {
        #[serde(default)]
        id: Option<String>,
    },
    GetSessionName {
        #[serde(default)]
        id: Option<String>,
    },
    SetSessionName {
        #[serde(default)]
        id: Option<String>,
        name: String,
    },
    SetSessionNameFor {
        #[serde(default)]
        id: Option<String>,
        session_id: String,
        name: String,
    },
    DeleteSession {
        #[serde(default)]
        id: Option<String>,
        session_id: String,
    },
    Reload {
        #[serde(default)]
        id: Option<String>,
    },
    LoadedResources {
        #[serde(default)]
        id: Option<String>,
    },
    GetQueueStats {
        #[serde(default)]
        id: Option<String>,
    },
    Steer {
        #[serde(default)]
        id: Option<String>,
        message: String,
    },
    FollowUp {
        #[serde(default)]
        id: Option<String>,
        message: String,
    },
    ClearQueue {
        #[serde(default)]
        id: Option<String>,
        #[serde(default = "default_true")]
        clear_steer: bool,
        #[serde(default = "default_true")]
        clear_follow_up: bool,
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

fn default_true() -> bool {
    true
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
            | Command::Compact { id, .. }
            | Command::GetSessionStats { id }
            | Command::ExportHtml { id, .. }
            | Command::ExportJsonl { id, .. }
            | Command::ImportJsonl { id, .. }
            | Command::SwitchSession { id, .. }
            | Command::Fork { id, .. }
            | Command::GetMessages { id }
            | Command::GetCommands { id }
            | Command::SessionTree { id, .. }
            | Command::TravelSessionTree { id, .. }
            | Command::AppendEntryLabel { id, .. }
            | Command::ListSessions { id }
            | Command::LoadSessionEntries { id, .. }
            | Command::NewSession { id }
            | Command::GetSessionName { id }
            | Command::SetSessionName { id, .. }
            | Command::SetSessionNameFor { id, .. }
            | Command::DeleteSession { id, .. }
            | Command::Reload { id }
            | Command::LoadedResources { id }
            | Command::GetQueueStats { id }
            | Command::Steer { id, .. }
            | Command::FollowUp { id, .. }
            | Command::ClearQueue { id, .. }
            | Command::Subscribe { id, .. }
            | Command::ApproveTool { id, .. }
            | Command::AnswerQuestion { id, .. }
            | Command::Quit { id } => id.as_deref(),
        }
    }
}
