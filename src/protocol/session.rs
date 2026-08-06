//! Session entry vocabulary — the shared data types describing a session's
//! persisted entries.
//!
//! Pure vocabulary (serde types only): both `agent` (compaction, export) and
//! `infra` (session manager, persistence) reference these. The storage-backend
//! enum (`SessionBackend`) is an infra implementation detail and stays in
//! `infra::session`; this module holds only the entry data shapes.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::message::{AgentMessage, EnvMessage};

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

/// Kind of session tree exposed via [`crate::app::core::driver::XyDriver`].
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

/// Parse session JSONL content: skip unparseable / non-SSOT lines with warn≤3 then `...`.
///
/// Requires a header with [`SESSION_VERSION`]; does not migrate older versions.
pub fn parse_session_jsonl(content: &str) -> Result<Vec<SessionEntry>, String> {
    let (entries, _) = parse_session_jsonl_lines(content);
    enforce_session_version(&entries)?;
    Ok(entries)
}

/// Line parse with skip/warn, without header-version enforcement (flush merge).
pub fn parse_session_jsonl_lines(content: &str) -> (Vec<SessionEntry>, usize) {
    let mut entries = Vec::new();
    let mut warn_count = 0usize;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => emit_session_load_warn(
                &mut warn_count,
                &format!("skip session line: parse entry: {e}"),
            ),
        }
    }
    (entries, warn_count)
}

fn emit_session_load_warn(warn_count: &mut usize, msg: &str) {
    *warn_count += 1;
    if *warn_count <= 3 {
        log::warn!(target: "xylitol::session", "{msg}");
    } else if *warn_count == 4 {
        log::warn!(target: "xylitol::session", "...");
    }
}

/// Reject sessions whose header is missing or not the current [`SESSION_VERSION`].
pub fn enforce_session_version(entries: &[SessionEntry]) -> Result<(), String> {
    let version = entries.iter().find_map(|e| match e {
        SessionEntry::Header(h) => Some(h.version),
        _ => None,
    });
    match version {
        Some(v) if v == SESSION_VERSION => Ok(()),
        Some(v) => Err(unsupported_session_version_msg(v)),
        None => Err("session has no header entry".into()),
    }
}

fn unsupported_session_version_msg(v: u32) -> String {
    format!(
        "session header version {v} is not supported (require {SESSION_VERSION}); refusing legacy migrate"
    )
}

/// First JSONL object's session-header version (listing fast-path; no full parse).
pub fn peek_session_header_version(content: &str) -> Option<u32> {
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        return match serde_json::from_str::<SessionEntry>(line) {
            Ok(SessionEntry::Header(h)) => Some(h.version),
            _ => None,
        };
    }
    None
}

/// Build a nested bash `SessionEntry::Message` for new bang writes (c1210 / be4).
pub fn bash_execution_message_entry(
    command: impl Into<String>,
    output: impl Into<String>,
    exit_code: Option<i32>,
    cancelled: bool,
    truncated: bool,
    full_output_path: Option<String>,
    exclude_from_context: bool,
) -> SessionEntry {
    let message = AgentMessage::Env(EnvMessage::BashExecutionMessage {
        command: command.into(),
        output: output.into(),
        exit_code,
        cancelled,
        truncated,
        full_output_path,
        exclude_from_context,
    });
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        message: serde_json::to_value(&message).unwrap_or(Value::Null),
    })
}

// ── Message entry helpers ───────────────────────────────────────────

/// AgentMessage-shaped JSON for session fixtures (c646 tagged content).
pub fn fixture_message_json(role: &str, text: &str) -> Value {
    serde_json::json!({
        "role": role,
        "content": [{ "type": "text", "text": text }],
        "timestamp": 0u64,
    })
}

/// Extract the `role` field from a serialized agent message JSON value.
pub fn message_role(msg: &Value) -> Option<&str> {
    msg.get("role").and_then(Value::as_str)
}

/// Extract **visible text** for editor prefill / tree summary (dm2).
///
/// Only aggregates `type=text` parts. Thinking / toolCall / image are skipped.
/// Bare-string content and untyped objects are ignored (c646: no legacy read).
pub fn message_text(msg: &Value) -> String {
    if let Some(parts) = msg
        .get("content")
        .or_else(|| msg.get("parts"))
        .and_then(Value::as_array)
    {
        return extract_text_from_parts(parts);
    }
    String::new()
}

fn extract_text_from_parts(parts: &[Value]) -> String {
    let mut out = String::new();
    for p in parts {
        let Some(obj) = p.as_object() else {
            continue;
        };
        if obj.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        if let Some(t) = obj.get("text").and_then(Value::as_str) {
            out.push_str(t);
        }
    }
    out
}

/// Iterate `content` or legacy `parts` arrays on a serialized message.
pub fn message_parts(msg: &Value) -> Option<&Vec<Value>> {
    msg.get("content")
        .or_else(|| msg.get("parts"))
        .and_then(Value::as_array)
}

/// Whether a content/part value is a tool-call (requires `type: toolCall`).
pub fn is_tool_call_part(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("toolCall") && part.get("name").is_some()
}

/// Tool name from a tool-call part, if any.
pub fn tool_call_name(part: &Value) -> Option<&str> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("name").and_then(Value::as_str)
}

/// Tool arguments object (`arguments` or legacy `args`).
pub fn tool_call_arguments(part: &Value) -> Option<&Value> {
    if !is_tool_call_part(part) {
        return None;
    }
    part.get("arguments").or_else(|| part.get("args"))
}

/// Count tool-call parts in a serialized message.
pub fn count_tool_calls(msg: &Value) -> usize {
    message_parts(msg)
        .map(|parts| parts.iter().filter(|p| is_tool_call_part(p)).count())
        .unwrap_or(0)
}

/// Collect unique `path` args from read/write/edit tool calls in a message.
pub fn tool_file_paths(msg: &Value) -> Vec<String> {
    let Some(parts) = message_parts(msg) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for part in parts {
        let Some(name) = tool_call_name(part) else {
            continue;
        };
        if !matches!(name, "read" | "write" | "edit") {
            continue;
        }
        let Some(path) = tool_call_arguments(part)
            .and_then(|a| a.get("path"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if !files.iter().any(|f| f == path) {
            files.push(path.to_string());
        }
    }
    files
}

/// Whether `entry` is a persisted user message.
pub fn is_user_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("user")
    )
}

/// Whether `entry` is a persisted assistant message.
pub fn is_assistant_message(entry: &SessionEntry) -> bool {
    matches!(
        entry,
        SessionEntry::Message(m) if message_role(&m.message) == Some("assistant")
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
            message: fixture_message_json(role, text),
        })
    }

    #[test]
    fn message_text_ignores_bare_string_content() {
        let msg = json!({
            "role": "user",
            "content": ["你好"],
            "timestamp": 1u64,
        });
        assert_eq!(message_text(&msg), "");
    }

    #[test]
    fn message_text_extracts_typed_text_parts() {
        let msg = json!({
            "role": "user",
            "content": [{ "type": "text", "text": "hello" }],
        });
        assert_eq!(message_text(&msg), "hello");
    }

    #[test]
    fn message_text_skips_thinking_parts() {
        let msg = json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "nope" },
                { "type": "text", "text": "yes" }
            ],
        });
        assert_eq!(message_text(&msg), "yes");
    }

    #[test]
    fn entry_shell_serializes_parent_id_camel_and_version_5() {
        let header = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: "s1".into(),
            timestamp: "t".into(),
            cwd: "/tmp".into(),
            parent_session: Some("p".into()),
        });
        let v = serde_json::to_value(&header).unwrap();
        assert_eq!(v["version"], 5);
        assert_eq!(v["parentSession"], "p");

        let msg = msg_entry("e1", Some("p1"), "user", "hi");
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["parentId"], "p1");
        assert_eq!(v["type"], "message");
        assert_eq!(v["message"]["content"][0]["type"], "text");
    }

    #[test]
    fn message_text_skips_tool_call_parts() {
        let msg = json!({
            "role": "assistant",
            "content": [
                { "type": "text", "text": "前置文字" },
                { "type": "toolCall", "id": "1", "name": "bash", "arguments": {"command": "ls"} }
            ],
        });
        assert_eq!(message_text(&msg), "前置文字");
    }

    #[test]
    fn message_text_legacy_parts_still_works() {
        let msg = json!({
            "role": "user",
            "parts": [{ "type": "text", "text": "legacy" }],
        });
        assert_eq!(message_text(&msg), "legacy");
    }

    #[test]
    fn count_tool_calls_reads_agent_message_shape() {
        let msg = json!({
            "role": "assistant",
            "content": [
                { "type": "text", "text": "ok" },
                { "type": "toolCall", "id": "1", "name": "read", "arguments": { "path": "a.rs" } },
                { "type": "toolCall", "id": "2", "name": "bash", "arguments": { "command": "ls" } }
            ],
        });
        assert_eq!(count_tool_calls(&msg), 2);
        assert_eq!(tool_file_paths(&msg), vec!["a.rs".to_string()]);
    }

    #[test]
    fn count_tool_calls_ignores_untagged_legacy_shape() {
        let msg = json!({
            "role": "assistant",
            "parts": [{
                "type": "FunctionCall",
                "id": "c1",
                "name": "write",
                "args": { "path": "b.rs" }
            }]
        });
        assert_eq!(count_tool_calls(&msg), 0);
        assert!(tool_file_paths(&msg).is_empty());
    }

    #[test]
    fn plan_travel_user_prefills_plain_text_not_json() {
        let entries = vec![msg_entry("u1", None, "user", "你能做什么")];
        let travel = plan_message_history_travel(&entries, "u1").expect("travel");
        assert_eq!(travel.editor_text.as_deref(), Some("你能做什么"));
        assert!(!travel.editor_text.as_deref().unwrap_or("").contains('{'));
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

    #[test]
    fn parse_session_jsonl_skips_legacy_bash_and_unknown_type() {
        let content = format!(
            "{}\n{}\n{}\n{}\n",
            serde_json::json!({
                "type": "session",
                "version": SESSION_VERSION,
                "id": "s1",
                "timestamp": "t",
                "cwd": "/tmp"
            }),
            serde_json::json!({
                "type": "message",
                "id": "m1",
                "timestamp": "t",
                "message": {
                    "role": "user",
                    "content": [{ "type": "text", "text": "ok" }]
                }
            }),
            r#"{"type":"bash_execution","id":"b1","timestamp":"t","command":"ls","output":"","cancelled":false,"truncated":false,"excludeFromContext":false}"#,
            r#"{"type":"unknownLegacy","id":"x1","timestamp":"t"}"#,
        );
        let entries = parse_session_jsonl(&content).expect("load");
        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0], SessionEntry::Header(_)));
        assert!(matches!(entries[1], SessionEntry::Message(_)));
    }

    #[test]
    fn parse_session_jsonl_rejects_non_current_header_version() {
        let content = r#"{"type":"session","version":4,"id":"s1","timestamp":"t","cwd":"/tmp"}
"#;
        let err = parse_session_jsonl(content).unwrap_err();
        assert!(err.contains("not supported"), "{err}");
    }

    fn compaction_entry(id: &str, first_kept: &str, summary: &str) -> SessionEntry {
        SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: id.into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            summary: summary.into(),
            first_kept_entry_id: first_kept.into(),
            tokens_before: 1000,
            details: None,
            from_hook: None,
        })
    }

    #[test]
    fn build_context_entries_without_compaction_returns_path() {
        let path = vec![
            msg_entry("u1", None, "user", "early"),
            msg_entry("a1", Some("u1"), "assistant", "reply"),
        ];
        let ctx = build_context_entries(&path);
        let ids: Vec<_> = ctx.iter().filter_map(|e| e.entry_id()).collect();
        assert_eq!(ids, vec!["u1", "a1"]);
    }

    #[test]
    fn build_context_entries_cuts_before_first_kept() {
        let path = vec![
            msg_entry("u_old", None, "user", "summarized away"),
            msg_entry("a_old", Some("u_old"), "assistant", "old reply"),
            msg_entry("u_keep", Some("a_old"), "user", "kept"),
            msg_entry("a_keep", Some("u_keep"), "assistant", "kept reply"),
            compaction_entry("c1", "u_keep", "## Goal\nkeep me"),
            msg_entry("u_new", Some("c1"), "user", "after compact"),
        ];
        let ctx = build_context_entries(&path);
        let ids: Vec<_> = ctx.iter().filter_map(|e| e.entry_id()).collect();
        assert_eq!(ids, vec!["c1", "u_keep", "a_keep", "u_new"]);
        assert!(!ids.contains(&"u_old"));
        assert!(!ids.contains(&"a_old"));
        match &ctx[0] {
            SessionEntry::Compaction(c) => assert!(c.summary.contains("keep me")),
            other => panic!("expected compaction first, got {other:?}"),
        }
    }

    #[test]
    fn build_context_entries_uses_latest_compaction() {
        let path = vec![
            msg_entry("u0", None, "user", "very old"),
            compaction_entry("c0", "u0", "old summary"),
            msg_entry("u1", Some("c0"), "user", "mid"),
            msg_entry("u2", Some("u1"), "user", "keep from here"),
            compaction_entry("c1", "u2", "new summary"),
        ];
        let ctx = build_context_entries(&path);
        let ids: Vec<_> = ctx.iter().filter_map(|e| e.entry_id()).collect();
        assert_eq!(ids, vec!["c1", "u2"]);
        assert!(!ids.contains(&"c0"));
        assert!(!ids.contains(&"u0"));
        assert!(!ids.contains(&"u1"));
    }
}

#[cfg(test)]
mod as_agent_message_tests {
    use super::*;
    use crate::protocol::message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
    use serde_json::{Value, json};

    fn entry(message: Value) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "e1".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            message,
        })
    }

    #[test]
    fn converts_agent_message_content_shape() {
        let e = entry(fixture_message_json("user", "hello"));
        let msg = e.as_agent_message().expect("user");
        assert_eq!(msg.role_name(), "user");
        assert_eq!(msg.text(), "hello");
    }

    #[test]
    fn converts_assistant_thinking_and_text() {
        let e = entry(json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "reason" },
                { "type": "text", "text": "answer" }
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }));
        let msg = e.as_agent_message().expect("assistant");
        match msg {
            AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. }) => {
                assert!(matches!(
                    content.as_slice(),
                    [
                        AgentPart::Thinking { thinking, .. },
                        AgentPart::Text { text }
                    ] if thinking == "reason" && text == "answer"
                ));
            }
            _ => panic!("expected assistant"),
        }
    }

    #[test]
    fn rejects_untagged_thinking_content() {
        let e = entry(json!({
            "role": "assistant",
            "content": [
                { "redacted": false, "text": "old thinking" },
                "answer"
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }));
        assert!(e.as_agent_message().is_none());
    }

    #[test]
    fn nested_bash_message_projects_to_env() {
        let e = entry(json!({
            "role": "bashExecution",
            "command": "ls",
            "output": "a",
            "cancelled": false,
            "truncated": false,
            "exclude_from_context": false,
        }));
        let msg = e.as_agent_message().expect("nested bash");
        match msg {
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                ..
            }) => {
                assert_eq!(command, "ls");
                assert_eq!(output, "a");
                assert!(!exclude_from_context);
            }
            _ => panic!("expected Env bash"),
        }
    }

    #[test]
    fn skips_excluded_nested_bash_message() {
        let e = entry(json!({
            "role": "bashExecution",
            "command": "secret",
            "output": "x",
            "cancelled": false,
            "truncated": false,
            "exclude_from_context": true,
        }));
        assert!(e.as_agent_message().is_none());
    }

    #[test]
    fn compaction_projects_to_env() {
        let e = SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: "t".into(),
            },
            summary: "sum".into(),
            first_kept_entry_id: "m1".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
        });
        let msg = e.as_agent_message().expect("compaction");
        assert_eq!(msg.role_name(), "compactionSummary");
        assert!(msg.text().contains("sum"));
    }
}
