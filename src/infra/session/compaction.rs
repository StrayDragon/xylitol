//! Context compaction — LLM-based structured summarization + cut-point detection.
//!
//! Aligns with pi's core/compaction/ module.
//!
//! Data flow:
//! 1. `should_compact()` — token threshold check (in crate::agent::session)
//! 2. `find_cut_point()` — find where to cut session entries to keep N tokens
//! 3. `generate_summary()` — LLM call producing structured summary
//! 4. `compact_session()` — orchestrate everything, write CompactionEntry

use anyhow::Result;
use serde_json::json;

use super::types::*;

// ── Compaction Settings ────────────────────────────────────────────

/// Settings controlling compaction behavior.
#[derive(Debug, Clone)]
pub struct CompactionSettings {
    /// Master toggle.
    pub enabled: bool,
    /// Tokens reserved for the summarization LLM call itself.
    pub reserve_tokens: u64,
    /// Target number of tokens to keep in the recent context window.
    pub keep_recent_tokens: u64,
}

impl Default for CompactionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        }
    }
}

// ── Cut-point detection ────────────────────────────────────────────

/// Result from [`find_cut_point`].
#[derive(Debug, Clone)]
pub struct CutPointResult {
    /// Index of first entry to keep (inclusive).
    pub first_kept_entry_index: usize,
    /// If cutting mid-turn, the user message that started that turn (-1 = no split).
    pub turn_start_index: isize,
    /// Whether the cut splits a turn (cut point is not a user message).
    pub is_split_turn: bool,
}

/// Estimate tokens for a single `SessionEntry` using chars/4 heuristic.
pub fn estimate_tokens_entry(entry: &SessionEntry) -> u64 {
    match entry {
        SessionEntry::Message(msg) => {
            // Estimate from the JSON-serialized message content
            let s = msg.message.to_string();
            (s.len() as u64).div_ceil(4)
        }
        SessionEntry::Header(_) => 0,
        SessionEntry::Compaction(c) => (c.summary.len() as u64).div_ceil(4),
        SessionEntry::BranchSummary(b) => (b.summary.len() as u64).div_ceil(4),
        SessionEntry::ModelChange(_) => 0,
        SessionEntry::ThinkingLevelChange(_) => 0,
        SessionEntry::Custom(c) => {
            let s = c.data.to_string();
            (s.len() as u64).div_ceil(4)
        }
    }
}

/// Check whether `entry_type` is a valid cut point.
///
/// Valid: message (user, assistant), branch_summary, custom_message.
/// Invalid: tool results (they must stay with their tool call), compaction boundaries.
fn is_valid_cut_point(entry: &SessionEntry) -> bool {
    match entry {
        SessionEntry::Message(msg) => {
            // Check role from the message Value
            msg.message
                .get("role")
                .and_then(|r| r.as_str())
                .map(|role| role == "user" || role == "assistant")
                .unwrap_or(false)
        }
        SessionEntry::BranchSummary(_) => true,
        // custom messages that are user-initiated can be cut points
        SessionEntry::Custom(c) => c.custom_type == "custom_message",
        _ => false,
    }
}

/// Check whether `entry` is a compaction boundary (don't cut across it).
fn is_compaction_boundary(entry: &SessionEntry) -> bool {
    matches!(entry, SessionEntry::Compaction(_))
}

/// Scan backwards to include non-message entries (model changes, settings, etc.)
/// that precede the cut point but aren't compaction boundaries.
fn include_preceding_non_messages(
    entries: &[SessionEntry],
    cut_index: usize,
    start_index: usize,
) -> usize {
    let mut idx = cut_index;
    while idx > start_index {
        let prev = &entries[idx - 1];
        if is_compaction_boundary(prev) {
            break;
        }
        match prev {
            SessionEntry::Message(_) => break,
            _ => idx -= 1,
        }
    }
    idx
}

/// Find the user message (or branch_summary/custom_message) that starts the turn
/// containing the given entry index.
fn find_turn_start_index(
    entries: &[SessionEntry],
    entry_index: usize,
    start_index: usize,
) -> isize {
    for i in (start_index..=entry_index).rev() {
        let entry = &entries[i];
        match entry {
            SessionEntry::BranchSummary(_) => return i as isize,
            SessionEntry::Custom(c) if c.custom_type == "custom_message" => return i as isize,
            SessionEntry::Message(msg) => {
                if let Some("user") = msg.message.get("role").and_then(|r| r.as_str()) {
                    return i as isize;
                }
            }
            _ => {}
        }
    }
    -1
}

/// Find the cut point in session entries that preserves approximately `keep_tokens` of recent context.
///
/// Algorithm: Walk backwards from newest, accumulating estimated message sizes.
/// Stop when accumulated >= keep_tokens. Find the closest valid cut point.
///
/// Only considers entries between `start_index` (inclusive) and `end_index` (exclusive).
pub fn find_cut_point(
    entries: &[SessionEntry],
    start_index: usize,
    end_index: usize,
    keep_tokens: u64,
) -> CutPointResult {
    if entries.is_empty() || start_index >= end_index {
        return CutPointResult {
            first_kept_entry_index: start_index,
            turn_start_index: -1,
            is_split_turn: false,
        };
    }

    // Collect all valid cut points in this range
    let mut cut_points: Vec<usize> = Vec::new();
    for (i, _entry) in entries.iter().enumerate().take(end_index).skip(start_index) {
        if is_valid_cut_point(&entries[i]) {
            cut_points.push(i);
        }
    }

    if cut_points.is_empty() {
        return CutPointResult {
            first_kept_entry_index: start_index,
            turn_start_index: -1,
            is_split_turn: false,
        };
    }

    // Walk backwards from newest, accumulate token estimates
    let mut accumulated = 0u64;
    let mut cut_index = cut_points[0]; // Default: keep from first cut point

    for i in (start_index..end_index).rev() {
        let entry = &entries[i];
        accumulated += estimate_tokens_entry(entry);

        if accumulated >= keep_tokens {
            // Find the closest valid cut point at or after this entry
            for &cp in &cut_points {
                if cp >= i {
                    cut_index = cp;
                    break;
                }
            }
            break;
        }
    }

    // Include preceding non-message entries
    cut_index = include_preceding_non_messages(entries, cut_index, start_index);

    // Check split-turn: if cut point is not a user/branch_summary, it's a split
    let cut_entry = &entries[cut_index];
    let is_user_turn_start = match cut_entry {
        SessionEntry::BranchSummary(_) => true,
        SessionEntry::Custom(c) => c.custom_type == "custom_message",
        SessionEntry::Message(msg) => msg
            .message
            .get("role")
            .and_then(|r| r.as_str())
            .map(|r| r == "user")
            .unwrap_or(false),
        _ => false,
    };

    if is_user_turn_start {
        CutPointResult {
            first_kept_entry_index: cut_index,
            turn_start_index: -1,
            is_split_turn: false,
        }
    } else {
        let turn_start = find_turn_start_index(entries, cut_index, start_index);
        CutPointResult {
            first_kept_entry_index: cut_index,
            turn_start_index: turn_start,
            is_split_turn: turn_start >= 0,
        }
    }
}

// ── File operation tracking ────────────────────────────────────────

/// Tracked file operations from tool calls.
#[derive(Debug, Clone, Default)]
pub struct FileOps {
    pub read: Vec<String>,
    pub written: Vec<String>,
    pub edited: Vec<String>,
}

impl FileOps {
    fn read_set(&self) -> std::collections::BTreeSet<&str> {
        self.read.iter().map(|s| s.as_str()).collect()
    }
    fn written_set(&self) -> std::collections::BTreeSet<&str> {
        self.written.iter().map(|s| s.as_str()).collect()
    }
    fn edited_set(&self) -> std::collections::BTreeSet<&str> {
        self.edited.iter().map(|s| s.as_str()).collect()
    }
}

/// Extract file operations from `XyContent` messages.
pub fn extract_file_ops_from_messages(
    messages: &[crate::agent::types::XyContent],
    prev_compaction: Option<&CompactionEntry>,
) -> FileOps {
    let mut ops = FileOps::default();

    // Merge from previous compaction's details
    if let Some(comp) = prev_compaction
        && let Some(ref details) = comp.details
    {
        if let Some(read_files) = details.get("readFiles").and_then(|v| v.as_array()) {
            for f in read_files {
                if let Some(s) = f.as_str() {
                    ops.read.push(s.to_string());
                }
            }
        }
        if let Some(modified) = details.get("modifiedFiles").and_then(|v| v.as_array()) {
            for f in modified {
                if let Some(s) = f.as_str() {
                    ops.edited.push(s.to_string());
                }
            }
        }
    }

    // Extract from tool calls in XyContent
    use crate::agent::types::XyPart;
    for msg in messages {
        for part in &msg.parts {
            if let XyPart::FunctionCall { name, args, .. } = part {
                let path = args.get("path").and_then(|v| v.as_str());
                // Also check "file_path" alias used by some tools
                let path = path.or_else(|| args.get("file_path").and_then(|v| v.as_str()));
                if let Some(p) = path {
                    match name.as_str() {
                        "read" => ops.read.push(p.to_string()),
                        "write" => {
                            ops.written.push(p.to_string());
                        }
                        "edit" => {
                            ops.edited.push(p.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Deduplicate
    ops.read.sort();
    ops.read.dedup();
    ops.written.sort();
    ops.written.dedup();
    ops.edited.sort();
    ops.edited.dedup();

    ops
}

/// Compute final file lists: read_only = read \ (edited ∪ written), modified = edited ∪ written.
pub fn compute_file_lists(ops: &FileOps) -> (Vec<String>, Vec<String>) {
    let modified_set: std::collections::BTreeSet<&str> = ops
        .edited_set()
        .union(&ops.written_set())
        .copied()
        .collect();
    let read_only: Vec<String> = ops
        .read_set()
        .difference(&modified_set)
        .map(|s| s.to_string())
        .collect();
    let modified: Vec<String> = modified_set.iter().map(|s| s.to_string()).collect();
    (read_only, modified)
}

/// Format file operations as XML tags for the summary suffix.
pub fn format_file_ops_xml(read_files: &[String], modified_files: &[String]) -> String {
    let mut parts = Vec::new();
    if !read_files.is_empty() {
        parts.push(format!(
            "<read-files>\n{}\n</read-files>",
            read_files.join("\n")
        ));
    }
    if !modified_files.is_empty() {
        parts.push(format!(
            "<modified-files>\n{}\n</modified-files>",
            modified_files.join("\n")
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", parts.join("\n\n"))
    }
}

// ── Summarization prompts ──────────────────────────────────────────

pub const SUMMARIZATION_SYSTEM_PROMPT: &str = "You are a context summarization assistant. Your task is to read a conversation between a user and an AI assistant, then produce a structured summary following the exact format specified.\n\nDo NOT continue the conversation. Do NOT respond to any questions in the conversation. ONLY output the structured summary.";

pub const SUMMARIZATION_PROMPT: &str = r#"The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or "(none)" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or "(none)" if not applicable]

Keep each section concise. Preserve exact file paths, function names, and error messages."#;

pub const UPDATE_SUMMARIZATION_PROMPT: &str = r#"The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.

Update the existing structured summary with new information. RULES:
- PRESERVE all existing information from the previous summary
- ADD new progress, decisions, and context from the new messages
- UPDATE the Progress section: move items from "In Progress" to "Done" when completed
- UPDATE "Next Steps" based on what was accomplished
- PRESERVE exact file paths, function names, and error messages
- If something is no longer relevant, you may remove it

Use this EXACT format:

## Goal
[Preserve existing goals, add new ones if the task expanded]

## Constraints & Preferences
- [Preserve existing, add new ones discovered]

## Progress
### Done
- [x] [Include previously done items AND newly completed items]

### In Progress
- [ ] [Current work - update based on progress]

### Blocked
- [Current blockers - remove if resolved]

## Key Decisions
- **[Decision]**: [Brief rationale] (preserve all previous, add new)

## Next Steps
1. [Update based on current state]

## Critical Context
- [Preserve important context, add new if needed]

Keep each section concise. Preserve exact file paths, function names, and error messages."#;

// ── LLM summarization ──────────────────────────────────────────────

use crate::agent::traits::XyModel;
use crate::agent::types::{XyChunk, XyContent, XyPart};
use futures::StreamExt;

/// Call the LLM non-streaming, collecting all text chunks.
async fn generate_complete(
    model: &dyn XyModel,
    messages: Vec<XyContent>,
    _max_tokens: u32,
) -> Result<String> {
    let mut stream = model
        .generate_stream(messages, &[], false)
        .await
        .map_err(|e| anyhow::anyhow!("summarization model error: {e}"))?;

    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk.map_err(|e| anyhow::anyhow!("summarization stream error: {e}"))? {
            XyChunk::TextDelta(delta) => text.push_str(&delta),
            XyChunk::ThinkingDelta(_) => {} // ignore thinking in summarization
            XyChunk::Done { .. } => break,
            XyChunk::FunctionCall { .. } => {} // shouldn't happen with empty tools
        }
    }

    if text.is_empty() {
        return Err(anyhow::anyhow!("summarization returned empty response"));
    }

    Ok(text)
}

/// Serialize `XyContent` messages to text for the summarization prompt.
pub fn serialize_conversation(messages: &[XyContent]) -> String {
    let mut parts = Vec::new();

    for msg in messages {
        match msg.role {
            crate::agent::types::XyRole::System => {
                for part in &msg.parts {
                    if let XyPart::Text(t) = part {
                        parts.push(format!("[System]: {t}"));
                    }
                }
            }
            crate::agent::types::XyRole::User => {
                for part in &msg.parts {
                    if let XyPart::Text(t) = part {
                        parts.push(format!("[User]: {t}"));
                    }
                }
            }
            crate::agent::types::XyRole::Assistant => {
                let mut text_parts = Vec::new();
                let mut thinking_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for part in &msg.parts {
                    match part {
                        XyPart::Text(t) => text_parts.push(t.as_str()),
                        XyPart::Thinking(t) => thinking_parts.push(t.as_str()),
                        XyPart::FunctionCall { name, args, .. } => {
                            let args_str = if let Some(obj) = args.as_object() {
                                obj.iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                args.to_string()
                            };
                            tool_calls.push(format!("{name}({args_str})"));
                        }
                        XyPart::FunctionResponse { .. } => {}
                    }
                }
                if !thinking_parts.is_empty() {
                    parts.push(format!(
                        "[Assistant thinking]: {}",
                        thinking_parts.join("\n")
                    ));
                }
                if !text_parts.is_empty() {
                    parts.push(format!("[Assistant]: {}", text_parts.join("\n")));
                }
                if !tool_calls.is_empty() {
                    parts.push(format!("[Assistant tool calls]: {}", tool_calls.join("; ")));
                }
            }
            crate::agent::types::XyRole::Tool => {
                for part in &msg.parts {
                    if let XyPart::FunctionResponse { name, result, .. } = part {
                        // Truncate very long tool results for summarization
                        let truncated = if result.len() > 2000 {
                            format!(
                                "{}... [{} more chars truncated]",
                                &result[..2000],
                                result.len() - 2000
                            )
                        } else {
                            result.clone()
                        };
                        parts.push(format!("[Tool result ({name})]: {truncated}"));
                    }
                }
            }
        }
    }

    parts.join("\n\n")
}

/// Generate a structured summary of messages using the LLM.
///
/// If `previous_summary` is provided, uses the UPDATE variant of the prompt
/// to merge new information into the existing summary.
pub async fn generate_summary(
    messages: &[XyContent],
    model: &dyn XyModel,
    _reserve_tokens: u64,
    previous_summary: Option<&str>,
) -> Result<String> {
    let conversation_text = serialize_conversation(messages);

    let base_prompt = if previous_summary.is_some() {
        UPDATE_SUMMARIZATION_PROMPT
    } else {
        SUMMARIZATION_PROMPT
    };

    let mut prompt_text = format!("<conversation>\n{conversation_text}\n</conversation>\n\n");
    if let Some(prev) = previous_summary {
        prompt_text.push_str(&format!(
            "<previous-summary>\n{prev}\n</previous-summary>\n\n"
        ));
    }
    prompt_text.push_str(base_prompt);

    let summarization_messages = vec![
        XyContent::system(SUMMARIZATION_SYSTEM_PROMPT),
        XyContent::user(&prompt_text),
    ];

    // Use 0.8 * reserve as max tokens for the response
    let max_tokens = ((_reserve_tokens as f64) * 0.8) as u32;
    generate_complete(model, summarization_messages, max_tokens.max(256)).await
}

// ── Main compaction function ───────────────────────────────────────

use super::manager::SessionManager;

/// Compact a session by summarizing old entries and writing a CompactionEntry.
///
/// Returns the created `CompactionEntry` on success.
pub async fn compact_session(
    mgr: &SessionManager,
    session_id: &str,
    model: &dyn XyModel,
    settings: &CompactionSettings,
) -> Result<CompactionEntry> {
    if !settings.enabled {
        anyhow::bail!("compaction disabled");
    }

    // 1. Load session entries
    let entries = mgr
        .load(session_id)
        .await
        .map_err(|e| anyhow::anyhow!("load session: {e}"))?;

    if entries.is_empty() {
        anyhow::bail!("empty session, nothing to compact");
    }

    // 2. Find previous compaction boundary and summary
    let mut prev_compaction: Option<&CompactionEntry> = None;
    let mut boundary_start = 0usize;
    for (i, entry) in entries.iter().enumerate().rev() {
        if let SessionEntry::Compaction(comp) = entry {
            prev_compaction = Some(comp);
            // Find first entry after this compaction
            let first_kept_id = comp.first_kept_entry_id.as_str();
            if let Some(idx) = entries
                .iter()
                .position(|e| e.entry_id() == Some(first_kept_id))
            {
                boundary_start = idx;
            } else {
                boundary_start = i + 1;
            }
            break;
        }
    }

    let boundary_end = entries.len();

    // 3. Estimate total tokens before compaction
    let tokens_before: u64 = entries[boundary_start..boundary_end]
        .iter()
        .map(estimate_tokens_entry)
        .sum();

    // 4. Find cut point
    let cut = find_cut_point(
        &entries,
        boundary_start,
        boundary_end,
        settings.keep_recent_tokens,
    );

    let first_kept_entry = &entries[cut.first_kept_entry_index];
    let first_kept_entry_id = first_kept_entry
        .entry_id()
        .ok_or_else(|| anyhow::anyhow!("first kept entry has no id"))?
        .to_string();

    // 5. Collect messages to summarize
    let history_end = if cut.is_split_turn {
        cut.turn_start_index.max(0) as usize
    } else {
        cut.first_kept_entry_index
    };

    let messages_to_summarize: Vec<XyContent> = entries[boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_xyt_content())
        .collect();

    // 6. Extract file operations
    let file_ops = extract_file_ops_from_messages(&messages_to_summarize, prev_compaction);

    // 7. Generate summary
    let previous_summary = prev_compaction.map(|c| c.summary.as_str());
    let summary = match generate_summary(
        &messages_to_summarize,
        model,
        settings.reserve_tokens,
        previous_summary,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            // Fallback: simple text summary
            tracing::warn!("LLM summarization failed, using fallback: {e}");
            format!(
                "[Compacted: {} entries, ~{tokens_before} tokens]",
                messages_to_summarize.len()
            )
        }
    };

    // 8. Append file operations XML to summary
    let (read_files, modified_files) = compute_file_lists(&file_ops);
    let summary_with_files = format!(
        "{}{}",
        summary,
        format_file_ops_xml(&read_files, &modified_files)
    );

    // 9. Build and write CompactionEntry
    let now = chrono::Utc::now().to_rfc3339();
    let entry = CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
            timestamp: now,
        },
        summary: summary_with_files,
        first_kept_entry_id,
        tokens_before,
        details: Some(json!({
            "readFiles": read_files,
            "modifiedFiles": modified_files,
        })),
        from_hook: Some(false),
    };

    mgr.append(session_id, &SessionEntry::Compaction(entry.clone()))
        .await
        .map_err(|e| anyhow::anyhow!("write compaction entry: {e}"))?;

    Ok(entry)
}

// ── XyContent conversion helper ─────────────────────────────────────

impl SessionEntry {
    /// Convert a SessionEntry to `XyContent` if it contains conversation content.
    pub fn as_xyt_content(&self) -> Option<XyContent> {
        match self {
            SessionEntry::Message(msg) => {
                let role = msg.message.get("role")?.as_str()?;
                let parts: Vec<XyPart> = msg
                    .message
                    .get("parts")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| {
                                let typ = p.get("type")?.as_str()?;
                                match typ {
                                    "Text" => {
                                        Some(XyPart::Text(p.get("text")?.as_str()?.to_string()))
                                    }
                                    "Thinking" => Some(XyPart::Thinking(
                                        p.get("thinking")?.as_str()?.to_string(),
                                    )),
                                    "FunctionCall" => Some(XyPart::FunctionCall {
                                        name: p.get("name")?.as_str()?.to_string(),
                                        args: p.get("args")?.clone(),
                                        id: p.get("id")?.as_str()?.to_string(),
                                    }),
                                    "FunctionResponse" => Some(XyPart::FunctionResponse {
                                        name: p.get("name")?.as_str()?.to_string(),
                                        result: p.get("result")?.as_str()?.to_string(),
                                        id: p.get("id")?.as_str()?.to_string(),
                                    }),
                                    _ => None,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                match role {
                    "user" => Some(XyContent {
                        role: crate::agent::types::XyRole::User,
                        parts,
                    }),
                    "assistant" => Some(XyContent {
                        role: crate::agent::types::XyRole::Assistant,
                        parts,
                    }),
                    "system" => Some(XyContent {
                        role: crate::agent::types::XyRole::System,
                        parts,
                    }),
                    "tool" => Some(XyContent {
                        role: crate::agent::types::XyRole::Tool,
                        parts,
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers for building test entries ──────────────────────────

    fn make_message_entry(id: &str, role: &str, content: &str) -> SessionEntry {
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.to_string(),
                parent_id: None,
                timestamp: now,
            },
            message: json!({
                "role": role,
                "parts": [{"type": "Text", "text": content}]
            }),
        })
    }

    fn make_user_entry(id: &str, content: &str) -> SessionEntry {
        make_message_entry(id, "user", content)
    }

    fn make_assistant_entry(id: &str, content: &str) -> SessionEntry {
        make_message_entry(id, "assistant", content)
    }

    fn make_tool_result_entry(id: &str, content: &str) -> SessionEntry {
        make_message_entry(id, "tool", content)
    }

    fn make_compaction_entry(id: &str, summary: &str, first_kept: &str) -> SessionEntry {
        let now = chrono::Utc::now().to_rfc3339();
        SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: id.to_string(),
                parent_id: None,
                timestamp: now,
            },
            summary: summary.to_string(),
            first_kept_entry_id: first_kept.to_string(),
            tokens_before: 5000,
            details: None,
            from_hook: Some(false),
        })
    }

    // ── T2: estimate_tokens_entry ─────────────────────────────────

    #[test]
    fn test_estimate_tokens_entry_basic() {
        let entry = make_user_entry("e1", "hello");
        let tokens = estimate_tokens_entry(&entry);
        // JSON-serialized size ≈ some dozens of chars
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_entry_header_zero() {
        let header = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: 3,
            id: "s1".into(),
            timestamp: "now".into(),
            cwd: ".".into(),
            parent_session: None,
        });
        assert_eq!(estimate_tokens_entry(&header), 0);
    }

    // ── T1: find_cut_point ───────────────────────────────────────

    #[test]
    fn test_find_cut_point_empty() {
        let entries: Vec<SessionEntry> = vec![];
        let result = find_cut_point(&entries, 0, 0, 1000);
        assert_eq!(result.first_kept_entry_index, 0);
        assert!(!result.is_split_turn);
    }

    #[test]
    fn test_find_cut_point_all_user_messages() {
        let long_content = "x".repeat(400); // ~100 tokens (400/4)
        let mut entries = Vec::new();
        for i in 0..20 {
            entries.push(make_user_entry(&format!("e{i}"), &long_content));
        }
        // 20 entries × ~100 tokens = ~2000 tokens
        let result = find_cut_point(&entries, 0, entries.len(), 500);
        // Should cut somewhere in the middle
        assert!(result.first_kept_entry_index > 0);
        assert!(result.first_kept_entry_index < entries.len());
    }

    #[test]
    fn test_find_cut_point_keep_all_when_below_threshold() {
        let mut entries = Vec::new();
        for i in 0..5 {
            entries.push(make_user_entry(&format!("e{i}"), "short"));
        }
        let result = find_cut_point(&entries, 0, entries.len(), 99999);
        // All entries are tiny, should keep from first entry
        assert_eq!(result.first_kept_entry_index, 0);
    }

    #[test]
    fn test_find_cut_point_respects_start_index() {
        let mut entries = Vec::new();
        for i in 0..10 {
            entries.push(make_user_entry(
                &format!("e{i}"),
                "standard entry content here",
            ));
        }
        // Only consider entries 5..10
        let result = find_cut_point(&entries, 5, 10, 1);
        assert!(result.first_kept_entry_index >= 5);
    }

    #[test]
    fn test_find_cut_point_split_turn_detected() {
        let long = "x".repeat(1000); // ~250 tokens
        let mut entries = Vec::new();
        // User message
        entries.push(make_user_entry("u1", &long));
        // Assistant message (cut point here = split turn)
        entries.push(make_assistant_entry("a1", &long));
        entries.push(make_tool_result_entry("t1", &long));
        entries.push(make_assistant_entry("a2", &long));
        entries.push(make_tool_result_entry("t2", &long));
        // Next user
        entries.push(make_user_entry("u2", "short"));

        let result = find_cut_point(&entries, 0, entries.len(), 200);
        // Should cut at the assistant after u2 or at u2
        // If it cuts at assistant "a2", it's a split turn from u1
        if result.is_split_turn {
            assert!(result.turn_start_index >= 0);
            assert_eq!(result.turn_start_index, 0); // u1 is the turn start
        }
    }

    // ── T3: Mixed entry types ────────────────────────────────────

    #[test]
    fn test_find_cut_point_mixed_types() {
        let long = "x".repeat(500);
        let entries = vec![
            make_user_entry("u1", &long),
            make_assistant_entry("a1", &long),
            make_tool_result_entry("t1", &long),
            make_user_entry("u2", &long),
            make_assistant_entry("a2", &long),
            make_tool_result_entry("t2", "short"),
        ];

        let result = find_cut_point(&entries, 0, entries.len(), 200);
        // Should never cut at tool_result
        let cut = &entries[result.first_kept_entry_index];
        match cut {
            SessionEntry::Message(msg) => {
                let role = msg.message["role"].as_str().unwrap();
                assert_ne!(role, "tool"); // Never cut at tool results
            }
            _ => {}
        }
    }

    // ── serialize_conversation ─────────────────────────────────────

    #[test]
    fn test_serialize_conversation_basic() {
        let msgs = vec![
            XyContent::user("Hello"),
            XyContent::assistant(vec![XyPart::Text("Hi there!".into())]),
        ];
        let text = serialize_conversation(&msgs);
        assert!(text.contains("[User]: Hello"));
        assert!(text.contains("[Assistant]: Hi there!"));
    }

    #[test]
    fn test_serialize_conversation_with_tool_calls() {
        let msgs = vec![
            XyContent::assistant(vec![
                XyPart::Text("Let me read that.".into()),
                XyPart::FunctionCall {
                    name: "read".into(),
                    args: json!({"path": "/tmp/test.txt"}),
                    id: "call-1".into(),
                },
            ]),
            XyContent {
                role: crate::agent::types::XyRole::Tool,
                parts: vec![XyPart::FunctionResponse {
                    name: "read".into(),
                    result: "file content here".into(),
                    id: "call-1".into(),
                }],
            },
        ];
        let text = serialize_conversation(&msgs);
        assert!(text.contains("Let me read that"));
        assert!(text.contains("read(path"));
        assert!(text.contains("/tmp/test.txt"));
        assert!(text.contains("Tool result (read)"));
    }

    // ── file_ops ───────────────────────────────────────────────────

    #[test]
    fn test_extract_file_ops_basic() {
        let msgs = vec![
            XyContent::assistant(vec![XyPart::FunctionCall {
                name: "read".into(),
                args: json!({"path": "src/main.rs"}),
                id: "c1".into(),
            }]),
            XyContent::assistant(vec![
                XyPart::FunctionCall {
                    name: "write".into(),
                    args: json!({"path": "src/new.rs", "content": "x"}),
                    id: "c2".into(),
                },
                XyPart::FunctionCall {
                    name: "edit".into(),
                    args: json!({"path": "src/old.rs", "oldText": "a", "newText": "b"}),
                    id: "c3".into(),
                },
            ]),
        ];

        let ops = extract_file_ops_from_messages(&msgs, None);
        assert_eq!(ops.read, vec!["src/main.rs"]);
        assert_eq!(ops.written, vec!["src/new.rs"]);
        assert_eq!(ops.edited, vec!["src/old.rs"]);
    }

    #[test]
    fn test_compute_file_lists() {
        let ops = FileOps {
            read: vec!["a.txt".into(), "b.txt".into()],
            written: vec!["b.txt".into(), "c.txt".into()],
            edited: vec!["c.txt".into()],
        };
        let (read_only, modified) = compute_file_lists(&ops);
        assert_eq!(read_only, vec!["a.txt"]); // Only a.txt is read-only
        assert_eq!(modified.len(), 2); // b.txt and c.txt are modified
    }

    #[test]
    fn test_format_file_ops_xml() {
        let xml = format_file_ops_xml(&["a.txt".into()], &["b.rs".into(), "c.py".into()]);
        assert!(xml.contains("<read-files>"));
        assert!(xml.contains("a.txt"));
        assert!(xml.contains("<modified-files>"));
        assert!(xml.contains("b.rs"));
        assert!(xml.contains("c.py"));
    }

    #[test]
    fn test_format_file_ops_xml_empty() {
        let xml = format_file_ops_xml(&[], &[]);
        assert!(xml.is_empty());
    }
}
