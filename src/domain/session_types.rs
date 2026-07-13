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

/// Kind of session tree exposed via [`crate::app::core::driver::Driver`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTreeKind {
    MessageHistory,
    /// Reserved; callers MUST receive an explicit error until implemented.
    FileBrowser,
}

/// Result of travelling a session tree — leaf position plus optional editor prefill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTreeTravel {
    pub kind: SessionTreeKind,
    pub selected_id: String,
    pub leaf_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_text: Option<String>,
}

/// Tree node for getTree() - defensive copy of session structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// ── Message entry helpers ───────────────────────────────────────────

/// Extract the `role` field from a serialized agent message JSON value.
pub fn message_role(msg: &Value) -> Option<&str> {
    msg.get("role").and_then(Value::as_str)
}

/// Extract human-readable text from a serialized agent message JSON value.
pub fn message_text(msg: &Value) -> String {
    if let Some(parts) = msg.get("parts").and_then(Value::as_array) {
        let mut out = String::new();
        for p in parts {
            if let Some(t) = p.get("text").and_then(Value::as_str) {
                out.push_str(t);
            } else {
                out.push_str(&p.to_string());
            }
        }
        return out;
    }
    msg.to_string()
}

/// Whether `entry` is a persisted user message.
pub fn is_user_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("user")
    )
}

// ── Session tree ────────────────────────────────────────────────────

/// Build a parent/child tree from flat session entries (labels resolved).
pub fn build_session_tree(entries: &[SessionEntry]) -> Vec<SessionTreeNode> {
    use std::collections::HashMap;

    let mut labels: HashMap<String, String> = HashMap::new();
    for entry in entries {
        if let SessionEntry::Label(l) = entry {
            if let Some(ref label) = l.label {
                labels.insert(l.target_id.clone(), label.clone());
            } else {
                labels.remove(&l.target_id);
            }
        }
    }

    let mut node_map: HashMap<String, SessionTreeNode> = HashMap::new();
    for entry in entries {
        if entry.entry_type() == "label" || entry.entry_type() == "session" {
            continue;
        }
        if let Some(id) = entry.entry_id() {
            let label = labels.get(id).cloned();
            node_map.insert(
                id.to_string(),
                SessionTreeNode {
                    entry: entry.clone(),
                    children: Vec::new(),
                    label,
                },
            );
        }
    }

    let mut roots: Vec<SessionTreeNode> = Vec::new();

    // Link children to parents (reverse order so parents stay in node_map).
    for entry in entries.iter().rev() {
        if entry.entry_type() == "label" || entry.entry_type() == "session" {
            continue;
        }
        let Some(id) = entry.entry_id() else {
            continue;
        };
        let Some(node) = node_map.remove(id) else {
            continue;
        };

        if let Some(parent_id) = entry.parent_id() {
            if let Some(parent) = node_map.get_mut(parent_id) {
                parent.children.push(node);
            } else {
                roots.push(node);
            }
        } else {
            roots.push(node);
        }
    }

    fn sort_children(nodes: &mut [SessionTreeNode]) {
        for node in nodes.iter_mut() {
            node.children.sort_by(|a, b| {
                let ta = a.entry.base().map(|b| b.timestamp.clone());
                let tb = b.entry.base().map(|b| b.timestamp.clone());
                ta.cmp(&tb)
            });
            sort_children(&mut node.children);
        }
    }
    sort_children(&mut roots);

    roots
}

/// Pure planner for MessageHistory travel (pi / c600 semantics).
pub fn plan_message_history_travel(
    entries: &[SessionEntry],
    selected_id: &str,
) -> Result<SessionTreeTravel, String> {
    let selected = entries
        .iter()
        .find(|e| e.entry_id() == Some(selected_id))
        .ok_or_else(|| format!("entry not found: {selected_id}"))?;

    if is_user_message(selected) {
        let SessionEntry::Message(m) = selected else {
            return Err(format!("entry not found: {selected_id}"));
        };
        Ok(SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: selected_id.to_string(),
            leaf_id: selected.parent_id().map(str::to_string),
            editor_text: Some(message_text(&m.message)),
        })
    } else {
        Ok(SessionTreeTravel {
            kind: SessionTreeKind::MessageHistory,
            selected_id: selected_id.to_string(),
            leaf_id: Some(selected_id.to_string()),
            editor_text: None,
        })
    }
}

#[cfg(test)]
mod session_tree_tests {
    use super::*;
    use serde_json::json;

    fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: parent.map(str::to_string),
                timestamp: format!("2026-01-01T00:00:{id}Z"),
            },
            message: json!({
                "role": role,
                "parts": [{ "type": "text", "text": text }],
            }),
        })
    }

    #[test]
    fn build_session_tree_links_parent_child() {
        let entries = vec![
            msg_entry("u1", None, "user", "hello"),
            msg_entry("a1", Some("u1"), "assistant", "hi"),
        ];
        let tree = build_session_tree(&entries);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].entry.entry_id(), Some("u1"));
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].entry.entry_id(), Some("a1"));
    }

    #[test]
    fn plan_travel_user_sets_parent_leaf_and_editor_text() {
        let entries = vec![
            msg_entry("u1", None, "user", "edit me"),
            msg_entry("a1", Some("u1"), "assistant", "reply"),
        ];
        let travel = plan_message_history_travel(&entries, "u1").expect("travel");
        assert_eq!(travel.kind, SessionTreeKind::MessageHistory);
        assert_eq!(travel.selected_id, "u1");
        assert_eq!(travel.leaf_id, None);
        assert_eq!(travel.editor_text.as_deref(), Some("edit me"));
    }

    #[test]
    fn plan_travel_non_user_sets_leaf_to_selected() {
        let entries = vec![
            msg_entry("u1", None, "user", "hello"),
            msg_entry("a1", Some("u1"), "assistant", "reply"),
        ];
        let travel = plan_message_history_travel(&entries, "a1").expect("travel");
        assert_eq!(travel.leaf_id.as_deref(), Some("a1"));
        assert!(travel.editor_text.is_none());
    }

    #[test]
    fn plan_travel_nested_user_uses_parent_leaf() {
        let entries = vec![
            msg_entry("u1", None, "user", "root"),
            msg_entry("u2", Some("u1"), "user", "child"),
        ];
        let travel = plan_message_history_travel(&entries, "u2").expect("travel");
        assert_eq!(travel.leaf_id.as_deref(), Some("u1"));
        assert_eq!(travel.editor_text.as_deref(), Some("child"));
    }
}
