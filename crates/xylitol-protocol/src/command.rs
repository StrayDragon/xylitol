use serde::{Deserialize, Serialize};

use crate::SessionTreeKind;

#[derive(Debug, Clone, Serialize, Deserialize, strum::VariantNames, strum::IntoStaticStr)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Prompt {
        message: String,
    },
    Abort {},
    GetState {},
    SetModel {
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
        #[serde(default)]
        instructions: Option<String>,
    },
    GetSessionStats {},
    ExportHtml {
        #[serde(default)]
        output_path: Option<String>,
    },
    ExportJsonl {
        #[serde(default)]
        output_path: Option<String>,
    },
    ImportJsonl {
        input_path: String,
    },
    SwitchSession {
        session_path: String,
    },
    Fork {
        entry_id: String,
        #[serde(default)]
        position: Option<String>,
    },
    GetMessages {},
    #[serde(rename = "estimate_context")]
    EstimateContext {},
    GetCommands {},
    SessionTree {
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
    #[serde(rename = "queue_stats")]
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
    Subscribe {
        session_id: String,
        last_seq: u64,
    },
    ApproveTool {
        call_id: String,
        approved: bool,
    },
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
