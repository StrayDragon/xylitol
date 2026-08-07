//! Session entry vocabulary — persisted entry shapes and SessionEntry.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::message::{AgentMessage, EnvMessage};

/// Current session format version.
/// v3: legacy (no id/parentId tree)
/// v4: tree-aware with id/parentId (snake_case / untagged AgentPart era)
/// v5: camelCase entry shell + tagged AgentPart content (c646 / pi-aligned)
pub const SESSION_VERSION: u32 = 5;

/// Where a session fork cuts the parent tree (aligns with pi `fork` position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkPosition {
    /// Include `at_entry_id` in the child path (pi `position: "at"` / clone).
    #[default]
    At,
    /// User-message only: path ends at the user's **parent**; the user entry is
    /// **not** copied (pi `/fork` default `position: "before"`).
    Before,
}

// ── Header ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    #[serde(skip, default)]
    pub entry_type: String, // "session" — provided by enum tag
    #[serde(default = "default_version")]
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
}

fn default_version() -> u32 {
    SESSION_VERSION
}

// ── Entry base ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryBase {
    #[serde(skip, default)]
    pub entry_type: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: String,
}

// ── Message entry ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub message: Value, // AgentMessage equivalent — serialized to JSON
}

// ── Compaction entry ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

// ── Branch summary entry ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ── Model change entry ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub provider: String,
    pub model_id: String,
}

// ── Thinking level change entry ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub thinking_level: String,
}

// ── Custom entry ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    pub data: Value,
}

// ── Custom message entry (participates in LLM context) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ── Label entry ──────────────────────────────────────────────────────

/// Label entry for user-defined bookmarks/markers on entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub target_id: String,
    pub label: Option<String>,
}

// ── Session info entry ──────────────────────────────────────────────

/// Session metadata entry (e.g., user-defined display name).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub name: Option<String>,
}

// ── Session context (reconstructed LLM messages) ───────────────────

/// Reconstructed session context from stored entries.
/// Messages use Value to avoid circular agent dependency in types.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub messages: Vec<serde_json::Value>, // Vec<AgentMessage> in JSON form
    pub thinking_level: String,
    pub model: Option<(String, String)>, // (provider, model_id)
}

// ── Unified entry enum ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Convert a persisted entry into an [`AgentMessage`] for context / ReAct seed.
    ///
    /// Includes: Message (all roles including nested `bashExecution`), compaction /
    /// branch summaries as Env. Honors `exclude_from_context` (returns `None`).
    /// Non-context entry kinds → `None`. Top-level bash is not a legal SSOT type.
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

/// Build LLM context entries from a leaf branch path (pi `buildContextEntries`).
///
/// Takes the **latest** compaction on `path`. Returns that compaction entry first,
/// then entries from `firstKeptEntryId` up to (but not including) the compaction,
/// then entries after the compaction. With no compaction, returns `path` unchanged.
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
    matches!(
        msg,
        AgentMessage::Env(EnvMessage::BashExecutionMessage {
            exclude_from_context: true,
            ..
        })
    )
}
