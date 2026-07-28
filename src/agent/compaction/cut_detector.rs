//! Cut-point detection — where to compact session entries.
//!
//! Walk session entries backwards from newest, accumulate token estimates,
//! and find the nearest valid boundary (user / assistant / bash / custom / branch).

use crate::protocol::session::SessionEntry;

/// Result from [`find_cut_point`].
#[derive(Debug, Clone)]
pub struct CutPointResult {
    /// Index of first entry to keep (inclusive).
    pub first_kept_entry_index: usize,
    /// If cutting mid-turn, the turn-start entry index (-1 = no split).
    pub turn_start_index: isize,
    /// Whether the cut splits a turn (cut point is not a turn-start).
    pub is_split_turn: bool,
}

/// Check if a token estimate indicates context overflow relative to the
/// model's context window.
pub fn is_context_overflow(token_estimate: u64, context_window: u64, reserve_tokens: u64) -> bool {
    if context_window == 0 {
        return false;
    }
    token_estimate + reserve_tokens > context_window
}

/// Estimate tokens for a single `SessionEntry` using chars/4 heuristic.
pub fn estimate_tokens_entry(entry: &SessionEntry) -> u64 {
    match entry {
        SessionEntry::Message(msg) => estimate_tokens_message_json(&msg.message),
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

fn estimate_tokens_message_json(message: &serde_json::Value) -> u64 {
    if let Some(s) = message.get("content").and_then(|c| c.as_str()) {
        return (s.len() as u64).div_ceil(4);
    }
    let Some(parts) = message
        .get("content")
        .or_else(|| message.get("parts"))
        .and_then(|p| p.as_array())
    else {
        return (message.to_string().len() as u64).div_ceil(4);
    };

    let mut tokens: u64 = 0;
    for part in parts {
        if let Some(t) = part.as_str() {
            tokens += (t.len() as u64).div_ceil(4);
            continue;
        }
        let typ = part.get("type").and_then(|t| t.as_str());
        if typ == Some("image")
            || ((part.get("url").is_some() || part.get("data").is_some())
                && part.get("text").is_none()
                && part.get("name").is_none())
        {
            tokens += 4800;
            continue;
        }
        match typ {
            Some("text") | Some("thinking") | None => {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    tokens += (text.len() as u64).div_ceil(4);
                } else {
                    tokens += (part.to_string().len() as u64).div_ceil(4);
                }
            }
            _ => {
                tokens += (part.to_string().len() as u64).div_ceil(4);
            }
        }
    }
    tokens
}

fn message_role(entry: &SessionEntry) -> Option<&str> {
    match entry {
        SessionEntry::Message(msg) => msg.message.get("role").and_then(|r| r.as_str()),
        _ => None,
    }
}

/// Valid cut points align with pi `isCutPointMessage` + entry types.
/// Never cut at `toolResult` (must follow its tool call).
fn is_valid_cut_point(entry: &SessionEntry) -> bool {
    match entry {
        SessionEntry::Message(_) => match message_role(entry) {
            Some("user")
            | Some("assistant")
            | Some("bashExecution")
            | Some("custom")
            | Some("branchSummary")
            | Some("compactionSummary") => true,
            Some("toolResult") => false,
            _ => false,
        },
        SessionEntry::BashExecution(_) => true,
        SessionEntry::BranchSummary(_) => true,
        SessionEntry::CustomMessage(_) => true,
        SessionEntry::Custom(c) => c.custom_type == "custom_message",
        _ => false,
    }
}

/// Turn-start aligns with pi `isTurnStartMessage` / `isTurnStartEntry`.
/// Assistant is never a turn start. Compaction entry type is never a turn start.
fn is_turn_start_entry(entry: &SessionEntry) -> bool {
    if matches!(entry, SessionEntry::Compaction(_)) {
        return false;
    }
    match entry {
        SessionEntry::Message(_) => matches!(
            message_role(entry),
            Some("user")
                | Some("bashExecution")
                | Some("custom")
                | Some("branchSummary")
                | Some("compactionSummary")
        ),
        SessionEntry::BashExecution(_) => true,
        SessionEntry::BranchSummary(_) => true,
        SessionEntry::CustomMessage(_) => true,
        SessionEntry::Custom(c) => c.custom_type == "custom_message",
        _ => false,
    }
}

fn is_compaction_boundary(entry: &SessionEntry) -> bool {
    matches!(entry, SessionEntry::Compaction(_))
}

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
            SessionEntry::BashExecution(_)
            | SessionEntry::BranchSummary(_)
            | SessionEntry::CustomMessage(_) => break,
            SessionEntry::Custom(c) if c.custom_type == "custom_message" => break,
            _ => idx -= 1,
        }
    }
    idx
}

fn find_turn_start_index(
    entries: &[SessionEntry],
    entry_index: usize,
    start_index: usize,
) -> isize {
    for i in (start_index..=entry_index).rev() {
        if is_turn_start_entry(&entries[i]) {
            return i as isize;
        }
    }
    -1
}

/// Find the cut point in session entries that preserves approximately `keep_tokens`
/// of recent context.
///
/// Walks backwards from newest, accumulating estimated message sizes.
/// Stops when accumulated >= keep_tokens, then finds the closest valid cut point.
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

    let mut accumulated = 0u64;
    let mut cut_index = cut_points[0];

    for i in (start_index..end_index).rev() {
        let entry = &entries[i];
        accumulated += estimate_tokens_entry(entry);

        if accumulated >= keep_tokens {
            for &cp in &cut_points {
                if cp >= i {
                    cut_index = cp;
                    break;
                }
            }
            break;
        }
    }

    // Remember the original cut point before including preceding non-messages.
    // If the original cut was at a turn-start, preserve non-split semantics even
    // when `include_preceding_non_messages` moves the index backwards.
    let original_cut_index = cut_index;
    cut_index = include_preceding_non_messages(entries, cut_index, start_index);

    let starts_turn = is_turn_start_entry(&entries[original_cut_index])
        || (cut_index..=original_cut_index).any(|i| is_turn_start_entry(&entries[i]));

    if starts_turn {
        CutPointResult {
            first_kept_entry_index: cut_index,
            turn_start_index: -1,
            is_split_turn: false,
        }
    } else {
        let turn_start = find_turn_start_index(entries, original_cut_index, start_index);
        CutPointResult {
            first_kept_entry_index: cut_index,
            turn_start_index: turn_start,
            is_split_turn: turn_start >= 0,
        }
    }
}
