use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::message::{AgentMessage, BashExecutionStatus, EnvMessage};

pub const SESSION_VERSION: u32 = 7;
pub const LEGACY_SESSION_VERSION: u32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkPosition {
    #[default]
    At,
    Before,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    #[serde(skip, default)]
    pub entry_type: String,
    #[serde(default = "default_version")]
    pub version: u32,
    pub id: String,
    pub timestamp: u64,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_at_entry_id: Option<String>,
}

fn default_version() -> u32 {
    SESSION_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryBase {
    #[serde(skip, default)]
    pub entry_type: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub message: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionPolicySnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimator_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl CompactionPolicySnapshot {
    pub fn current(
        context_window: u64,
        reserve_tokens: u64,
        keep_recent_tokens: u64,
        estimator_version: impl Into<String>,
    ) -> Self {
        Self {
            context_window: Some(context_window),
            reserve_tokens: Some(reserve_tokens),
            keep_recent_tokens: Some(keep_recent_tokens),
            estimator_version: Some(estimator_version.into()),
            status: None,
        }
    }

    pub fn legacy_unknown() -> Self {
        Self {
            context_window: None,
            reserve_tokens: None,
            keep_recent_tokens: None,
            estimator_version: None,
            status: Some("legacy/unknown".into()),
        }
    }

    pub fn is_complete_current(&self) -> bool {
        self.context_window.is_some()
            && self.reserve_tokens.is_some()
            && self.keep_recent_tokens.is_some()
            && self
                .estimator_version
                .as_deref()
                .is_some_and(|version| !version.trim().is_empty())
            && self.status.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_hook: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<CompactionPolicySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchSummaryEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub from_id: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_hook: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub provider: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub thinking_level: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomMessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    pub content: Value,
    pub display: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub target_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub messages: Vec<serde_json::Value>,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    #[serde(rename = "session")]
    Header(SessionHeader),
    #[serde(rename = "message")]
    Message(MessageEntry),
    #[serde(rename = "compaction")]
    Compaction(CompactionEntry),
    #[serde(rename = "branchSummary")]
    BranchSummary(BranchSummaryEntry),
    #[serde(rename = "modelChange")]
    ModelChange(ModelChangeEntry),
    #[serde(rename = "thinkingLevelChange")]
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    #[serde(rename = "custom")]
    Custom(CustomEntry),
    #[serde(rename = "customMessage")]
    CustomMessage(CustomMessageEntry),
    #[serde(rename = "label")]
    Label(LabelEntry),
    #[serde(rename = "sessionInfo")]
    SessionInfo(SessionInfoEntry),
}

impl SessionEntry {
    pub fn base(&self) -> Option<&EntryBase> {
        match self {
            SessionEntry::Header(_) => None,
            SessionEntry::Message(e) => Some(&e.base),
            SessionEntry::Compaction(e) => Some(&e.base),
            SessionEntry::BranchSummary(e) => Some(&e.base),
            SessionEntry::ModelChange(e) => Some(&e.base),
            SessionEntry::ThinkingLevelChange(e) => Some(&e.base),
            SessionEntry::Custom(e) => Some(&e.base),
            SessionEntry::CustomMessage(e) => Some(&e.base),
            SessionEntry::Label(e) => Some(&e.base),
            SessionEntry::SessionInfo(e) => Some(&e.base),
        }
    }

    pub fn entry_type(&self) -> &str {
        match self {
            SessionEntry::Header(_) => "session",
            SessionEntry::Message(_) => "message",
            SessionEntry::Compaction(_) => "compaction",
            SessionEntry::BranchSummary(_) => "branchSummary",
            SessionEntry::ModelChange(_) => "modelChange",
            SessionEntry::ThinkingLevelChange(_) => "thinkingLevelChange",
            SessionEntry::Custom(_) => "custom",
            SessionEntry::CustomMessage(_) => "customMessage",
            SessionEntry::Label(_) => "label",
            SessionEntry::SessionInfo(_) => "sessionInfo",
        }
    }

    pub fn entry_id(&self) -> Option<&str> {
        self.base().map(|b| b.id.as_str())
    }

    pub fn parent_id(&self) -> Option<&str> {
        self.base().and_then(|b| b.parent_id.as_deref())
    }

    pub fn anchors_transcript(&self) -> bool {
        !matches!(
            self,
            SessionEntry::Header(_)
                | SessionEntry::ModelChange(_)
                | SessionEntry::ThinkingLevelChange(_)
        )
    }

    pub fn as_agent_message(&self) -> Option<AgentMessage> {
        match self {
            SessionEntry::Message(msg) => {
                match serde_json::from_value::<AgentMessage>(msg.message.clone()) {
                    Ok(agent_msg) => {
                        if agent_msg_excluded_from_context(&agent_msg) {
                            return None;
                        }
                        Some(agent_msg)
                    }
                    Err(e) => {
                        log::warn!(target: "xylitol::session", "skip message entry: AgentMessage deserialize failed (c646 tagged wire only) error={}", e);
                        None
                    }
                }
            }
            SessionEntry::Compaction(c) => {
                Some(AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                    summary: c.summary.clone(),
                    tokens_before: c.tokens_before,
                    tokens_after: 0,
                    read_files: None,
                    modified_files: None,
                }))
            }
            SessionEntry::BranchSummary(b) => {
                Some(AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                    summary: b.summary.clone(),
                    from_id: b.from_id.clone(),
                }))
            }
            SessionEntry::CustomMessage(cm) => {
                if !cm.display {
                    return None;
                }
                Some(AgentMessage::Env(EnvMessage::CustomMessage {
                    custom_type: cm.custom_type.clone(),
                    content: cm.content.clone(),
                    display: Value::Bool(cm.display),
                    details: cm.details.clone().unwrap_or(Value::Null),
                }))
            }
            _ => None,
        }
    }
}

pub fn done_bash_ids(entries: &[SessionEntry]) -> std::collections::HashSet<String> {
    entries
        .iter()
        .filter_map(SessionEntry::as_agent_message)
        .filter_map(|msg| match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                bash_id,
                status: BashExecutionStatus::Done,
                ..
            }) if !bash_id.is_empty() => Some(bash_id),
            _ => None,
        })
        .collect()
}

pub fn fold_interrupted_bash_rows(
    messages: Vec<AgentMessage>,
    done_bash_ids: &std::collections::HashSet<String>,
) -> Vec<AgentMessage> {
    messages
        .into_iter()
        .map(|msg| match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                bash_id,
                command,
                exclude_from_context,
                status: BashExecutionStatus::Running,
                ..
            }) if !exclude_from_context && !done_bash_ids.contains(&bash_id) => {
                AgentMessage::Env(EnvMessage::interrupted_bash(command))
            }
            other => other,
        })
        .collect()
}

pub fn build_context_entries(path: &[SessionEntry]) -> Vec<SessionEntry> {
    let mut latest: Option<(usize, &CompactionEntry)> = None;
    for (i, entry) in path.iter().enumerate() {
        if let SessionEntry::Compaction(c) = entry {
            latest = Some((i, c));
        }
    }
    let Some((compaction_idx, compaction)) = latest else {
        return path.to_vec();
    };

    let mut out = Vec::with_capacity(path.len().saturating_sub(compaction_idx) + 1);
    out.push(path[compaction_idx].clone());

    let mut found_first_kept = false;
    for entry in &path[..compaction_idx] {
        if entry.entry_id() == Some(compaction.first_kept_entry_id.as_str()) {
            found_first_kept = true;
        }
        if found_first_kept {
            out.push(entry.clone());
        }
    }
    out.extend(path[compaction_idx + 1..].iter().cloned());
    out
}

fn agent_msg_excluded_from_context(msg: &AgentMessage) -> bool {
    match msg {
        AgentMessage::Env(env) => env.exclude_from_context(),
        AgentMessage::Llm(_) => false,
    }
}
