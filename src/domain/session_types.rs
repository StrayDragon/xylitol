//! Session entry vocabulary — the shared data types describing a session's
//! persisted entries. Aligns with pi's SessionEntry interfaces.
//!
//! Pure vocabulary (serde types only): both `agent` (compaction, export) and
//! `infra` (session manager, persistence) reference these. The storage-backend
//! enum (`SessionBackend`) is an infra implementation detail and stays in
//! `infra::session`; this module holds only the entry data shapes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current session format version.
/// v3: legacy (no id/parentId tree)
/// v4: tree-aware with id/parentId
pub const SESSION_VERSION: u32 = 4;

// ── Header ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ── Bash execution entry ─────────────────────────────────────────────

/// Records a user-initiated bash execution (`!cmd` / `!!cmd`).
///
/// When `exclude_from_context` is true (`!!` prefix), the entry is stored
/// on disk but omitted from the LLM context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BashExecutionEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub command: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_output_path: Option<String>,
    pub exclude_from_context: bool,
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

/// Tree node for getTree() - defensive copy of session structure.
#[derive(Debug, Clone)]
pub struct SessionTreeNode {
    /// The session entry at this node.
    pub entry: SessionEntry,
    /// Child nodes in timestamp order.
    pub children: Vec<SessionTreeNode>,
    /// Resolved label for this entry, if any.
    pub label: Option<String>,
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
    #[serde(rename = "branch_summary")]
    BranchSummary(BranchSummaryEntry),
    #[serde(rename = "model_change")]
    ModelChange(ModelChangeEntry),
    #[serde(rename = "thinking_level_change")]
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    #[serde(rename = "custom")]
    Custom(CustomEntry),
    #[serde(rename = "custom_message")]
    CustomMessage(CustomMessageEntry),
    #[serde(rename = "label")]
    Label(LabelEntry),
    #[serde(rename = "session_info")]
    SessionInfo(SessionInfoEntry),
    #[serde(rename = "bash_execution")]
    BashExecution(BashExecutionEntry),
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
            SessionEntry::BashExecution(e) => Some(&e.base),
        }
    }

    pub fn entry_type(&self) -> &str {
        match self {
            SessionEntry::Header(_) => "session",
            SessionEntry::Message(_) => "message",
            SessionEntry::Compaction(_) => "compaction",
            SessionEntry::BranchSummary(_) => "branch_summary",
            SessionEntry::ModelChange(_) => "model_change",
            SessionEntry::ThinkingLevelChange(_) => "thinking_level_change",
            SessionEntry::Custom(_) => "custom",
            SessionEntry::CustomMessage(_) => "custom_message",
            SessionEntry::Label(_) => "label",
            SessionEntry::SessionInfo(_) => "session_info",
            SessionEntry::BashExecution(_) => "bash_execution",
        }
    }

    pub fn entry_id(&self) -> Option<&str> {
        self.base().map(|b| b.id.as_str())
    }

    pub fn parent_id(&self) -> Option<&str> {
        self.base().and_then(|b| b.parent_id.as_deref())
    }
}
