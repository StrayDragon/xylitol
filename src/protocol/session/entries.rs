//! Session entry vocabulary — persisted entry shapes and SessionEntry.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::message::{AgentMessage, BashExecutionStatus, EnvMessage};

/// Current session format version.
/// v3: legacy (no id/parentId tree)
/// v4: tree-aware with id/parentId (snake_case / untagged AgentPart era)
/// v5: camelCase entry shell + tagged AgentPart content (c646 / pi-aligned)
/// v6: v5 semantics + shell/header timestamps are u64 unix-ms; no serde aliases
/// Current storage: manifest + active/sealed JSONL segments
pub const SESSION_VERSION: u32 = 7;
pub const LEGACY_SESSION_VERSION: u32 = 6;

/// Where a session fork cuts the parent tree.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    #[serde(skip, default)]
    pub entry_type: String, // "session" — provided by enum tag
    #[serde(default = "default_version")]
    pub version: u32,
    pub id: String,
    pub timestamp: u64,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    /// Fork cut: the `at_entry_id` passed to `fork` (s23). Absent on non-fork / old files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fork_at_entry_id: Option<String>,
}

fn default_version() -> u32 {
    SESSION_VERSION
}

// ── Entry base ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryBase {
    #[serde(skip, default)]
    pub entry_type: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub timestamp: u64,
}

// ── Message entry ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub message: Value, // AgentMessage equivalent — serialized to JSON
}

// ── Compaction entry ───────────────────────────────────────────────

/// The policy snapshot captured when a compaction summary was produced.
///
/// Current entries carry all four values. Migrated v6 entries use
/// `status = "legacy/unknown"` and leave the values absent so diagnostics
/// cannot mistake them for the current runtime policy.
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

// ── Branch summary entry ───────────────────────────────────────────

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

// ── Model change entry ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub provider: String,
    pub model_id: String,
}

// ── Thinking level change entry ────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingLevelChangeEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub thinking_level: String,
}

// ── Custom entry ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub custom_type: String,
    pub data: Value,
}

// ── Custom message entry (participates in LLM context) ─────────────

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

// ── Label entry ──────────────────────────────────────────────────────

/// Label entry for user-defined bookmarks/markers on entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelEntry {
    #[serde(flatten)]
    pub base: EntryBase,
    pub target_id: String,
    pub label: Option<String>,
}

// ── Session info entry ──────────────────────────────────────────────

/// Session metadata entry (e.g., user-defined display name).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// True when this entry kind participates in the message chain and may
    /// anchor transcript projection / tree travel (leaf).
    ///
    /// Bookkeeping rows (`session` header / `modelChange` /
    /// `thinkingLevelChange`) never anchor: they project no transcript rows and
    /// cold materialize may append them before any leaf is known (parent-less
    /// tail), which would collapse an ancestry walk to that row alone.
    pub fn anchors_transcript(&self) -> bool {
        !matches!(
            self,
            SessionEntry::Header(_)
                | SessionEntry::ModelChange(_)
                | SessionEntry::ThinkingLevelChange(_)
        )
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

/// Session-scoped set of `bash_id`s that have a done row (c2770 / as-bang1).
///
/// Pairing is session-wide on purpose: a done row off the resumed leaf path
/// (fork / travel to an earlier leaf) still proves the command finished, so it
/// MUST suppress the interrupted notice.
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

/// Fold orphan running bash rows into one stable interrupted notice (c2770).
///
/// Orphan = running row whose `bash_id` has no done row in `done_bash_ids`
/// (session-scoped — see [`done_bash_ids`]). The orphan becomes
/// [`EnvMessage::interrupted_bash`]; excluded (`!!`) rows and running rows
/// paired with a done keep the c2760 no-projection behavior. Call this once on
/// the post-cut message list, in every SessionEntry → AgentMessage assembly
/// that feeds the model (as48 single path).
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
    match msg {
        AgentMessage::Env(env) => env.exclude_from_context(),
        AgentMessage::Llm(_) => false,
    }
}

#[cfg(test)]
mod interrupted_bash_tests {
    use super::*;
    use crate::protocol::message::{BashExecutionStatus, LlmMessage};
    use crate::protocol::session::bash_execution_message_entry;

    fn bash_row(
        bash_id: &str,
        command: &str,
        status: BashExecutionStatus,
        exclude: bool,
    ) -> SessionEntry {
        bash_execution_message_entry(
            bash_id,
            command,
            "",
            Some(0),
            false,
            false,
            None,
            exclude,
            status,
        )
    }

    fn mapped(entries: &[SessionEntry]) -> Vec<AgentMessage> {
        entries
            .iter()
            .filter_map(|e| e.as_agent_message())
            .collect()
    }

    #[test]
    fn orphan_running_projects_interrupted_notice() {
        let entries = vec![bash_row("b1", "serve", BashExecutionStatus::Running, false)];
        let done = done_bash_ids(&entries);
        let folded = fold_interrupted_bash_rows(mapped(&entries), &done);
        let rows = crate::agent::llm_project::project_for_llm(&folded);
        let texts: Vec<_> = rows.iter().filter_map(user_text_of).collect();
        assert_eq!(
            texts,
            vec!["[interrupted] $ serve".to_string()],
            "orphan running MUST fold into the pinned interrupted line: {texts:?}"
        );
    }

    #[test]
    fn paired_running_stays_hidden_and_done_folds() {
        let entries = vec![
            bash_row("b1", "serve", BashExecutionStatus::Running, false),
            bash_row("b1", "serve", BashExecutionStatus::Done, false),
        ];
        let done = done_bash_ids(&entries);
        let folded = fold_interrupted_bash_rows(mapped(&entries), &done);
        let rows = crate::agent::llm_project::project_for_llm(&folded);
        let texts: Vec<_> = rows.iter().filter_map(user_text_of).collect();
        assert!(
            texts.iter().all(|t| !t.contains("[interrupted]")),
            "paired running MUST NOT project interrupted: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("$ serve")),
            "done row keeps the bash fold: {texts:?}"
        );
    }

    #[test]
    fn excluded_orphan_never_projects() {
        let entries = vec![bash_row("b1", "clean", BashExecutionStatus::Running, true)];
        assert!(
            mapped(&entries).is_empty(),
            "excluded rows are dropped before the fold (as_agent_message)"
        );
        let folded = fold_interrupted_bash_rows(mapped(&entries), &done_bash_ids(&entries));
        assert!(
            crate::agent::llm_project::project_for_llm(&folded).is_empty(),
            "`!!` orphan MUST NOT project, interrupted included"
        );
    }

    #[test]
    fn repeated_folds_are_byte_stable() {
        let entries = vec![
            bash_row("b1", "serve", BashExecutionStatus::Running, false),
            bash_row("b1", "serve", BashExecutionStatus::Done, false),
            bash_row("b2", "serve", BashExecutionStatus::Running, false),
        ];
        let done = done_bash_ids(&entries);
        let texts = |rows: &Vec<crate::protocol::message::LlmMessage>| -> Vec<String> {
            rows.iter().filter_map(|m| user_text_of(m)).collect()
        };
        let first = texts(&crate::agent::llm_project::project_for_llm(
            &fold_interrupted_bash_rows(mapped(&entries), &done),
        ));
        let second = texts(&crate::agent::llm_project::project_for_llm(
            &fold_interrupted_bash_rows(mapped(&entries), &done),
        ));
        // Synthesized rows carry per-build wall-clock timestamps (pre-existing
        // user_text behavior for every env fold); the folded TEXT is the
        // as48/as-bang1 byte-stability contract.
        assert_eq!(first, second, "repeated context builds MUST be byte-stable");
    }

    #[test]
    fn done_off_leaf_path_still_suppresses() {
        // Session-scope pairing: the caller passes ALL session entries, so a
        // done row outside the resumed window still suppresses the notice.
        let on_path = vec![bash_row("b1", "serve", BashExecutionStatus::Running, false)];
        let whole_session = vec![
            bash_row("b1", "serve", BashExecutionStatus::Running, false),
            bash_row("b1", "serve", BashExecutionStatus::Done, false),
        ];
        let done = done_bash_ids(&whole_session);
        let folded = fold_interrupted_bash_rows(mapped(&on_path), &done);
        let rows = crate::agent::llm_project::project_for_llm(&folded);
        assert!(
            rows.is_empty(),
            "done off the leaf path MUST suppress the interrupted notice: {rows:?}"
        );
    }

    fn user_text_of(msg: &LlmMessage) -> Option<String> {
        match msg {
            LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                crate::protocol::message::AgentPart::Text { text } => Some(text.clone()),
                _ => None,
            }),
            _ => None,
        }
    }
}
