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

use crate::infra::session::types::*;

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

// ── Token calculation (c50) ───────────────────────────────────────

/// Provider usage information from an assistant message.
#[derive(Debug, Clone, Copy)]
pub struct XyUsage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

/// Calculate total context tokens from a Usage struct.
/// Priority: total_tokens > input+output+cache_read+cache_write sum.
pub fn calculate_context_tokens(usage: &XyUsage) -> u64 {
    if usage.total_tokens > 0 {
        return usage.total_tokens;
    }
    usage.input_tokens + usage.output_tokens + usage.cache_read_tokens + usage.cache_write_tokens
}

/// Structure returned by [`estimate_context_tokens`].
#[derive(Debug, Clone)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Estimate context tokens from messages.
/// Uses chars/4 heuristic per message.
/// When a `last_usage` is provided, uses real usage tokens and
/// estimates only trailing messages.
pub fn estimate_context_tokens(
    messages: &[crate::core::message::AgentMessage],
    last_usage: Option<&XyUsage>,
) -> ContextUsageEstimate {
    if let Some(usage) = last_usage {
        let usage_tokens = calculate_context_tokens(usage);
        let mut trailing_tokens = 0u64;
        for msg in messages {
            trailing_tokens += estimate_tokens_agent(msg);
        }
        ContextUsageEstimate {
            tokens: usage_tokens + trailing_tokens,
            usage_tokens,
            trailing_tokens,
            last_usage_index: Some(0),
        }
    } else {
        let mut total = 0u64;
        for msg in messages {
            total += estimate_tokens_agent(msg);
        }
        ContextUsageEstimate {
            tokens: total,
            usage_tokens: 0,
            trailing_tokens: total,
            last_usage_index: None,
        }
    }
}

/// Estimate tokens for an AgentMessage using chars/4.
fn estimate_tokens_agent(msg: &crate::core::message::AgentMessage) -> u64 {
    let s = serde_json::to_string(msg).unwrap_or_default();
    (s.len() as u64).div_ceil(4)
}

/// Check if compaction should trigger based on context token usage.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool {
    if !settings.enabled {
        return false;
    }
    let threshold = context_window.saturating_sub(settings.reserve_tokens);
    context_tokens > threshold
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
/// Check if a token estimate indicates context overflow relative to the
/// model's context window.
pub fn is_context_overflow(token_estimate: u64, context_window: u64, reserve_tokens: u64) -> bool {
    if context_window == 0 {
        return false;
    }
    token_estimate + reserve_tokens > context_window
}

pub fn estimate_tokens_entry(entry: &SessionEntry) -> u64 {
    match entry {
        SessionEntry::Message(msg) => {
            // Estimate from the JSON-serialized message content.
            // Check for image parts (4800 tokens per image as a heuristic).
            let mut tokens: u64 = 0;
            if let Some(parts) = msg.message.get("parts").and_then(|p| p.as_array()) {
                for part in parts {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("image") => tokens += 4800,
                        Some("text") | Some("thinking") => {
                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                tokens += (text.len() as u64).div_ceil(4);
                            }
                        }
                        _ => {
                            tokens += (part.to_string().len() as u64).div_ceil(4);
                        }
                    }
                }
            } else {
                // Fallback: estimate from the whole JSON
                tokens = (msg.message.to_string().len() as u64).div_ceil(4);
            }
            tokens
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
        SessionEntry::CustomMessage(cm) => {
            let s = cm.content.to_string();
            (s.len() as u64).div_ceil(4)
        }
        SessionEntry::Label(_) => 0,
        SessionEntry::SessionInfo(_) => 0,
        SessionEntry::BashExecution(b) => {
            (b.command.len() as u64 + b.output.len() as u64).div_ceil(4)
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

/// Extract file operations from `AgentMessage` messages.
pub fn extract_file_ops_from_messages(
    messages: &[crate::core::message::AgentMessage],
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

    // Extract from tool calls in AgentMessage
    for msg in messages {
        for part in msg.content() {
            if let crate::core::message::AgentPart::ToolCall {
                name, arguments, ..
            } = part
            {
                let path = arguments.get("path").and_then(|v| v.as_str());
                // Also check "file_path" alias used by some tools
                let path = path.or_else(|| arguments.get("file_path").and_then(|v| v.as_str()));
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

use crate::core::traits::XyModel;
use crate::core::types::XyChunk;
use futures::StreamExt;

/// Call the LLM non-streaming, collecting all text chunks.
async fn generate_complete(
    model: &dyn XyModel,
    messages: Vec<crate::core::message::AgentMessage>,
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

/// Serialize `AgentMessage` messages to text for the summarization prompt.
pub fn serialize_conversation(messages: &[crate::core::message::AgentMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        let role = msg.role_name();
        let content = msg.content();
        match role {
            "user" | "toolResult" | "bashExecution" => {
                for part in content {
                    if let Some(t) = part.as_text() {
                        parts.push(format!("[{role}]: {t}"));
                    }
                }
            }
            "assistant" => {
                let mut text_parts = Vec::new();
                let mut thinking_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for part in content {
                    match part {
                        crate::core::message::AgentPart::Text(t) => text_parts.push(t.as_str()),
                        crate::core::message::AgentPart::Thinking { text, .. } => {
                            thinking_parts.push(text.as_str())
                        }
                        crate::core::message::AgentPart::ToolCall {
                            name, arguments, ..
                        } => {
                            let args_str = if let Some(obj) = arguments.as_object() {
                                obj.iter()
                                    .map(|(k, v)| format!("{k}={v}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            } else {
                                arguments.to_string()
                            };
                            tool_calls.push(format!("{name}({args_str})"));
                        }
                        _ => {}
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
            _ => {
                parts.push(format!("[{role}]: {}", msg.text()));
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
    messages: &[crate::core::message::AgentMessage],
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

    let summarization_messages = vec![crate::core::message::AgentMessage::user(
        prompt_text.clone(),
    )];
    // System prompt will be prepended by the conversion layer.

    // Use 0.8 * reserve as max tokens for the response
    let max_tokens = ((_reserve_tokens as f64) * 0.8) as u32;
    generate_complete(model, summarization_messages, max_tokens.max(256)).await
}

// ── Main compaction function ───────────────────────────────────────

use crate::infra::session::manager::SessionManager;

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

    let messages_to_summarize: Vec<crate::core::message::AgentMessage> = entries
        [boundary_start..history_end]
        .iter()
        .filter_map(|entry| entry.as_agent_message())
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

// ── AgentMessage conversion helper ────────────────────────────────────

// ── LLM Branch Summary ────────────────────────────────────────────

/// Prompt for generating a branch summary (aligned with pi's BRANCH_SUMMARY_PROMPT).
const BRANCH_SUMMARY_PROMPT: &str = "Create a structured summary of this conversation branch for context when returning later.\n\nUse this EXACT format:\n\n## Goal\n[What was the user trying to accomplish in this branch?]\n\n## Constraints & Preferences\n- [Any constraints, preferences, or requirements mentioned]\n- [Or \"(none)\" if none were mentioned]\n\n## Progress\n### Done\n- [x] [Completed tasks/changes]\n\n### In Progress\n- [ ] [Work that was started but not finished]\n\n### Blocked\n- [Issues preventing progress, if any]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale]\n\n## Next Steps\n1. [What should happen next to continue this work]\n\nKeep each section concise. Preserve exact file paths, function names, and error messages.";

/// Preamble prepended to branch summaries for LLM context.
const BRANCH_SUMMARY_PREAMBLE: &str = "The user explored a different conversation branch before returning here.\nSummary of that exploration:\n\n";

/// Prepare entries for branch summarization with a token budget.
///
/// Walks entries from NEWEST to OLDEST, adding messages until the token
/// budget is exhausted. Also collects file operations from tool calls.
pub fn prepare_branch_entries(entries: &[SessionEntry], token_budget: u64) -> BranchPreparation {
    let mut messages: Vec<crate::core::message::AgentMessage> = Vec::new();
    let mut file_ops = FileOps::default();
    let mut total_tokens: u64 = 0;

    // Walk from newest to oldest
    for entry in entries.iter().rev() {
        let msg = match entry.as_agent_message() {
            Some(m) => m,
            None => continue,
        };

        // Skip tool results for budget (context is in assistant's tool call)
        if msg.role_name() == "toolResult" {
            continue;
        }

        // Extract file ops from all messages
        extract_single_message_file_ops(&msg, &mut file_ops);

        let tokens = (serde_json::to_string(&msg).unwrap_or_default().len() as u64).div_ceil(4);

        // Check budget
        if token_budget > 0 && total_tokens + tokens > token_budget {
            // Allow compaction/branch_summary entries to fit
            if matches!(
                entry,
                SessionEntry::Compaction(_) | SessionEntry::BranchSummary(_)
            ) && total_tokens < token_budget * 9 / 10
            {
                messages.insert(0, msg);
                total_tokens += tokens;
            }
            break;
        }

        messages.insert(0, msg);
        total_tokens += tokens;
    }

    BranchPreparation {
        messages,
        file_ops,
        total_tokens,
    }
}

/// Result from preparing branch entries.
#[derive(Debug)]
pub struct BranchPreparation {
    pub messages: Vec<crate::core::message::AgentMessage>,
    pub file_ops: FileOps,
    pub total_tokens: u64,
}

/// Extract file operations from a single AgentMessage.
fn extract_single_message_file_ops(msg: &crate::core::message::AgentMessage, ops: &mut FileOps) {
    for part in msg.content() {
        let (name, path) = match part {
            crate::core::message::AgentPart::ToolCall {
                name, arguments, ..
            } => match arguments.get("path").and_then(|v| v.as_str()) {
                Some(p) => (name.as_str(), p.to_string()),
                None => continue,
            },
            _ => continue,
        };
        match name {
            "read" if !ops.read.contains(&path) => ops.read.push(path),
            "write" if !ops.written.contains(&path) => ops.written.push(path),
            "edit" if !ops.edited.contains(&path) => ops.edited.push(path),
            _ => {}
        }
    }
}

/// Generate an LLM-powered branch summary.
///
/// Falls back to plain-text stats when no model is available (returns None).
/// Aligns with pi's generateBranchSummary().
pub async fn generate_branch_summary_llm(
    entries: &[SessionEntry],
    model: &dyn XyModel,
    reserve_tokens: u64,
) -> Option<BranchSummaryResult> {
    if entries.is_empty() {
        return Some(BranchSummaryResult {
            summary: "No content to summarize".to_string(),
            read_files: vec![],
            modified_files: vec![],
        });
    }

    // Token budget = default context window minus reserved
    let token_budget = 128_000u64.saturating_sub(reserve_tokens);
    let preparation = prepare_branch_entries(entries, token_budget);

    if preparation.messages.is_empty() {
        return Some(BranchSummaryResult {
            summary: "No content to summarize".to_string(),
            read_files: vec![],
            modified_files: vec![],
        });
    }

    // Serialize to text and build prompt
    let conversation_text = serialize_conversation(&preparation.messages);
    let prompt_text =
        format!("<conversation>\n{conversation_text}\n</conversation>\n\n{BRANCH_SUMMARY_PROMPT}");

    let messages = vec![crate::core::message::AgentMessage::user(
        prompt_text.clone(),
    )];

    // Call LLM (non-streaming, max 2048 tokens)
    let result = generate_complete(model, messages, 2048).await;

    match result {
        Ok(mut summary) => {
            summary = format!("{BRANCH_SUMMARY_PREAMBLE}{summary}");

            let (read_files, modified_files) = compute_file_lists(&preparation.file_ops);
            summary.push_str(&format_file_ops_xml(&read_files, &modified_files));

            Some(BranchSummaryResult {
                summary: if summary.is_empty() {
                    "No summary generated".to_string()
                } else {
                    summary
                },
                read_files,
                modified_files,
            })
        }
        Err(_) => None, // LLM call failed
    }
}

/// Result from branch summarization.
#[derive(Debug, Clone)]
pub struct BranchSummaryResult {
    pub summary: String,
    pub read_files: Vec<String>,
    pub modified_files: Vec<String>,
}

// ── ────────────────────────────────────────────────────────────────

impl SessionEntry {
    /// Convert a SessionEntry to `AgentMessage` if it contains conversation content.
    pub fn as_agent_message(&self) -> Option<crate::core::message::AgentMessage> {
        match self {
            SessionEntry::Message(msg) => {
                // Try direct deserialization first (preferred path)
                if let Ok(agent_msg) = serde_json::from_value::<crate::core::message::AgentMessage>(
                    msg.message.clone(),
                ) {
                    return Some(agent_msg);
                }
                // Fallback: try legacy JSON format
                let role = msg.message.get("role")?.as_str()?;
                let parts: Vec<crate::core::message::AgentPart> = msg
                    .message
                    .get("parts")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| {
                                let typ = p.get("type")?.as_str()?;
                                match typ {
                                    "Text" => Some(crate::core::message::AgentPart::Text(
                                        p.get("text")?.as_str()?.to_string(),
                                    )),
                                    "Thinking" => Some(crate::core::message::AgentPart::Thinking {
                                        text: p.get("thinking")?.as_str()?.to_string(),
                                        redacted: false,
                                        signature: None,
                                    }),
                                    "FunctionCall" => {
                                        Some(crate::core::message::AgentPart::ToolCall {
                                            name: p.get("name")?.as_str()?.to_string(),
                                            arguments: p.get("args")?.clone(),
                                            id: p.get("id")?.as_str()?.to_string(),
                                        })
                                    }
                                    "FunctionResponse" => {
                                        Some(crate::core::message::AgentPart::ToolResult {
                                            tool_use_id: p.get("id")?.as_str()?.to_string(),
                                            content: vec![crate::core::message::AgentPart::Text(
                                                p.get("result")?.as_str()?.to_string(),
                                            )],
                                            is_error: false,
                                        })
                                    }
                                    _ => None,
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                match role {
                    "user" => Some(crate::core::message::AgentMessage::UserMessage {
                        content: parts,
                        timestamp: crate::core::message::now_ms(),
                    }),
                    "assistant" => Some(crate::core::message::AgentMessage::AssistantMessage {
                        content: parts,
                        stop_reason: None,
                        usage: None,
                        api: String::new(),
                        provider: String::new(),
                        model: String::new(),
                        response_id: None,
                        error_message: None,
                        timestamp: crate::core::message::now_ms(),
                        diagnostics: Vec::new(),
                    }),
                    "system" => Some(crate::core::message::AgentMessage::UserMessage {
                        content: parts,
                        timestamp: crate::core::message::now_ms(),
                    }),
                    "tool" => Some(crate::core::message::AgentMessage::ToolResultMessage {
                        tool_use_id: String::new(),
                        tool_name: String::new(),
                        content: parts,
                        details: None,
                        is_error: false,
                        timestamp: crate::core::message::now_ms(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

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
        // User message: u1, then assistant/tool cycles, then u2
        let entries = vec![
            make_user_entry("u1", &long),
            make_assistant_entry("a1", &long),
            make_tool_result_entry("t1", &long),
            make_assistant_entry("a2", &long),
            make_tool_result_entry("t2", &long),
            make_user_entry("u2", "short"),
        ];

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
        if let SessionEntry::Message(msg) = cut {
            let role = msg.message["role"].as_str().unwrap();
            assert_ne!(role, "tool"); // Never cut at tool results
        }
    }

    // ── serialize_conversation ─────────────────────────────────────

    #[test]
    fn test_serialize_conversation_basic() {
        let msgs = vec![
            crate::core::message::AgentMessage::user("Hello"),
            crate::core::message::AgentMessage::AssistantMessage {
                content: vec![crate::core::message::AgentPart::Text("Hi there!".into())],
                stop_reason: Some(crate::core::message::StopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
        ];
        let text = serialize_conversation(&msgs);
        assert!(text.contains("[user]: Hello"));
        assert!(text.contains("[Assistant]: Hi there!"));
    }

    #[test]
    fn test_serialize_conversation_with_tool_calls() {
        let msgs = vec![
            crate::core::message::AgentMessage::AssistantMessage {
                content: vec![
                    crate::core::message::AgentPart::Text("Let me read that.".into()),
                    crate::core::message::AgentPart::ToolCall {
                        name: "read".into(),
                        arguments: json!({"path": "/tmp/test.txt"
                        }),
                        id: "call-1".into(),
                    },
                ],
                stop_reason: Some(crate::core::message::StopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
            crate::core::message::AgentMessage::UserMessage {
                content: vec![crate::core::message::AgentPart::ToolResult {
                    tool_use_id: "call-1".into(),
                    content: vec![crate::core::message::AgentPart::Text(
                        "file content here".into(),
                    )],
                    is_error: false,
                }],
                timestamp: 0,
            },
        ];
        let text = serialize_conversation(&msgs);
        assert!(text.contains("Let me read that"));
        assert!(text.contains("read(path"));
        assert!(text.contains("/tmp/test.txt"));
    }

    // ── file_ops ───────────────────────────────────────────────────

    #[test]
    fn test_extract_file_ops_basic() {
        let msgs = vec![
            crate::core::message::AgentMessage::AssistantMessage {
                content: vec![crate::core::message::AgentPart::ToolCall {
                    name: "read".into(),
                    arguments: json!({"path": "src/main.rs"
                    }),
                    id: "c1".into(),
                }],
                stop_reason: Some(crate::core::message::StopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
            crate::core::message::AgentMessage::AssistantMessage {
                content: vec![
                    crate::core::message::AgentPart::ToolCall {
                        name: "write".into(),
                        arguments: json!({"path": "src/new.rs", "content": "x"
                        }),
                        id: "c2".into(),
                    },
                    crate::core::message::AgentPart::ToolCall {
                        name: "edit".into(),
                        arguments: json!({"path": "src/old.rs", "oldText": "a", "newText": "b"
                        }),
                        id: "c3".into(),
                    },
                ],
                stop_reason: Some(crate::core::message::StopReason::Stop),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            },
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

    // ── c50: Token calculation tests ──────────────────────────

    #[test]
    fn test_calculate_context_tokens_total() {
        let usage = XyUsage {
            total_tokens: 50000,
            input_tokens: 30000,
            output_tokens: 15000,
            cache_read_tokens: 3000,
            cache_write_tokens: 2000,
        };
        // total_tokens has priority
        assert_eq!(calculate_context_tokens(&usage), 50000);
    }

    #[test]
    fn test_calculate_context_tokens_sum() {
        let usage = XyUsage {
            total_tokens: 0,
            input_tokens: 30000,
            output_tokens: 15000,
            cache_read_tokens: 3000,
            cache_write_tokens: 2000,
        };
        // Falls back to sum
        assert_eq!(calculate_context_tokens(&usage), 50000);
    }

    #[test]
    fn test_should_compact_triggers() {
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };
        // 170000 tokens, 200000 window → 170000 > 200000-16384=183616? false
        let false_case = should_compact(170000, 200000, &settings);
        assert!(!false_case);
        // 190000 tokens > 183616 → true
        let true_case = should_compact(190000, 200000, &settings);
        assert!(true_case);
    }

    #[test]
    fn test_should_compact_disabled() {
        let settings = CompactionSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_compact(999999, 100000, &settings));
    }

    /// Collect entries suitable for branch summarization from a session.
    ///
    /// Returns entries from the fork point (target_entry_id's parent) to the
    /// end of the session, limited by a token budget.
    pub async fn collect_entries_for_branch_summary(
        mgr: &SessionManager,
        session_id: &str,
        fork_entry_id: &str,
        _token_budget: u64,
    ) -> Result<Vec<SessionEntry>> {
        let entries = mgr
            .load(session_id)
            .await
            .map_err(|e| anyhow::anyhow!("load: {e}"))?;

        // Find the fork point
        let fork_index = entries
            .iter()
            .position(|e| e.entry_id() == Some(fork_entry_id))
            .ok_or_else(|| anyhow::anyhow!("entry not found: {fork_entry_id}"))?;

        // Collect entries AFTER the fork point
        let entries_to_collect: Vec<SessionEntry> = entries[fork_index + 1..]
            .iter()
            .take_while(|e| !matches!(e, SessionEntry::BranchSummary(_)))
            .cloned()
            .collect();

        Ok(entries_to_collect)
    }

    /// Generate a branch summary entry (LLM or fallback) and persist it.
    pub async fn create_branch_summary_entry(
        mgr: &SessionManager,
        session_id: &str,
        fork_entry_id: &str,
        model: Option<&dyn XyModel>,
        token_budget: u64,
    ) -> Result<()> {
        let entries =
            collect_entries_for_branch_summary(mgr, session_id, fork_entry_id, token_budget)
                .await?;

        if entries.is_empty() {
            return Ok(());
        }

        let summary = if let Some(m) = model {
            generate_branch_summary_llm(&entries, m, token_budget)
                .await
                .map(|r| r.summary)
                .unwrap_or_else(|| String::new())
        } else {
            String::new()
        };

        let summary = if summary.is_empty() {
            // Fallback summary when no LLM is available
            let count = entries.len();
            let user_count = entries.iter().filter(|e| {
            matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("user"))
        }).count();
            let assistant_count = entries.iter().filter(|e| {
            matches!(e, SessionEntry::Message(m) if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant"))
        }).count();
            format!(
                "Branch summary: {count} entries ({user_count} user, {assistant_count} assistant messages)"
            )
        } else {
            summary
        };

        use chrono::Utc;
        use uuid::Uuid;
        let branch_entry = SessionEntry::BranchSummary(BranchSummaryEntry {
            base: EntryBase {
                entry_type: "branch_summary".into(),
                id: Uuid::new_v4().to_string(),
                parent_id: Some(fork_entry_id.to_string()),
                timestamp: Utc::now().to_rfc3339(),
            },
            from_id: fork_entry_id.to_string(),
            summary,
            details: None,
            from_hook: Some(false),
        });

        mgr.append_with_id(session_id, &branch_entry)
            .await
            .map_err(|e| anyhow::anyhow!("append branch summary: {e}"))?;

        Ok(())
    }

    #[test]
    fn test_estimate_context_tokens() {
        let msgs = vec![
            crate::core::message::AgentMessage::user("hello"),
            crate::core::message::AgentMessage::user("world"),
        ];
        let estimate = estimate_context_tokens(&msgs, None);
        assert!(estimate.tokens > 0);
        assert_eq!(estimate.usage_tokens, 0);
    }
}
