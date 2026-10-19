//! Cut-point detection — where to compact session entries.
//!
//! Walk session entries backwards from newest, accumulate token estimates,
//! and find the nearest valid boundary (user / assistant / bash / custom / branch).

use xylitol_plugin_sdk::accounting::heuristic_token_count;
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, LlmMessage};
use xylitol_plugin_sdk::session::SessionEntry;

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

/// pi `ESTIMATED_IMAGE_CHARS` — counted as chars before `/3`.
const ESTIMATED_IMAGE_CHARS: u64 = 4800;

/// Estimate tokens for cut-point walking (pi `estimateTokens` on AgentMessage).
///
/// Returns 0 when the entry has no context-visible message (skipped in accumulation).
/// Message rows that fail typed deserialize still use a lax content walk (string or
/// parts) so cut math stays usable for legacy / fixture wire shapes.
/// cut 逐条度量与 unified 估算链（`accounting::heuristic_tokens`）同源：
/// 决策 SSOT = 对**投影后单条**消息按序列化字节 /3 计数（同一算法族、同一投影源）；
/// 独立的内容字符启发式不参与决策。不可行的反序列化走 lax 兜底。
///
/// 注意：unified 的启发式是逐条 `serde_json::to_string /3` 之和，本函数即它的
/// 逐条切片——切点位置因此与决策/展示落在同一度量空间（中文会话不再 5x 失真）。
pub fn estimate_tokens_entry_for_cut(entry: &SessionEntry) -> u64 {
    match entry {
        SessionEntry::Message(msg) => {
            match serde_json::from_value::<AgentMessage>(msg.message.clone()) {
                Ok(agent_msg) => {
                    if let AgentMessage::Env(env) = &agent_msg
                        && env.exclude_from_context()
                    {
                        return 0;
                    }
                    unified_projected_entry_token_estimate(&agent_msg)
                }
                // lax 兜底：非 SSOT，仅在消息反序列化失败时使用。
                Err(_) => heuristic_token_count(estimate_lax_message_json_chars(&msg.message)),
            }
        }
        _ => entry
            .as_agent_message()
            .map(|m| unified_projected_entry_token_estimate(&m))
            .unwrap_or(0),
    }
}

/// c2848: 与 `accounting::heuristic_tokens` 同算法的单条投影计数（逐条切片）。
/// 投影丢弃 error 助手与无投影的 Env —— 与 unified 估算完全同源。
///
/// pi 视觉成本单独叠加：图片（user/toolResult）按 `ESTIMATED_IMAGE_CHARS`（≈1600
/// token）计——投影后的序列化只含图片元数据、不承载真实视觉载荷，必须显式建模。
fn unified_projected_entry_token_estimate(agent_msg: &AgentMessage) -> u64 {
    let image_tokens = match agent_msg {
        AgentMessage::Llm(LlmMessage::UserMessage { content, .. })
        | AgentMessage::Llm(LlmMessage::ToolResultMessage { content, .. }) => {
            content
                .iter()
                .filter(|p| matches!(p, AgentPart::Image(_)))
                .count() as u64
                * heuristic_token_count(ESTIMATED_IMAGE_CHARS)
        }
        _ => 0,
    };
    let projected = xylitol_plugin_sdk::project_for_llm(std::slice::from_ref(agent_msg));
    let serialized: u64 = projected
        .iter()
        .map(|m| {
            heuristic_token_count(
                serde_json::to_string(m)
                    .map(|s| s.len() as u64)
                    .unwrap_or(0),
            )
        })
        .sum();
    serialized.saturating_add(image_tokens)
}

/// Lax wire: string `content` or part array (text / image / thinking / toolCall).
fn estimate_lax_message_json_chars(message: &serde_json::Value) -> u64 {
    if let Some(s) = message.get("content").and_then(|c| c.as_str()) {
        return s.len() as u64;
    }
    let Some(parts) = message
        .get("content")
        .or_else(|| message.get("parts"))
        .and_then(|p| p.as_array())
    else {
        return 0;
    };
    let role = message.get("role").and_then(|r| r.as_str()).unwrap_or("");
    let mut chars = 0u64;
    for part in parts {
        let typ = part.get("type").and_then(|t| t.as_str());
        match typ {
            Some("image") => chars += ESTIMATED_IMAGE_CHARS,
            Some("thinking") if role == "assistant" => {
                if let Some(t) = part.get("thinking").and_then(|t| t.as_str()) {
                    chars += t.len() as u64;
                }
            }
            Some("toolCall") | Some("tool_call") if role == "assistant" => {
                chars += part
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.len() as u64)
                    .unwrap_or(0);
                if let Some(args) = part.get("arguments") {
                    chars += args.to_string().len() as u64;
                }
            }
            Some("text") | None => {
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    chars += t.len() as u64;
                } else if let Some(s) = part.as_str() {
                    chars += s.len() as u64;
                }
            }
            _ => {}
        }
    }
    chars
}

/// Estimate tokens for a single `SessionEntry` using the `/3` byte heuristic.
///
/// Prefer [`estimate_tokens_entry_for_cut`] for cut-point walking (pi-aligned).
pub fn estimate_tokens_entry(entry: &SessionEntry) -> u64 {
    match entry {
        SessionEntry::Message(msg) => estimate_tokens_message_json(&msg.message),
        SessionEntry::Header(_) => 0,
        SessionEntry::Compaction(c) => heuristic_token_count(c.summary.len() as u64),
        SessionEntry::BranchSummary(b) => heuristic_token_count(b.summary.len() as u64),
        SessionEntry::ModelChange(_) => 0,
        SessionEntry::ThinkingLevelChange(_) => 0,
        SessionEntry::Custom(c) => {
            let s = c.data.to_string();
            heuristic_token_count(s.len() as u64)
        }
        SessionEntry::CustomMessage(cm) => {
            let s = cm.content.to_string();
            heuristic_token_count(s.len() as u64)
        }
        SessionEntry::Label(_) => 0,
        SessionEntry::SessionInfo(_) => 0,
    }
}

fn estimate_tokens_message_json(message: &serde_json::Value) -> u64 {
    if let Some(s) = message.get("content").and_then(|c| c.as_str()) {
        return heuristic_token_count(s.len() as u64);
    }
    let Some(parts) = message
        .get("content")
        .or_else(|| message.get("parts"))
        .and_then(|p| p.as_array())
    else {
        return heuristic_token_count(message.to_string().len() as u64);
    };

    let mut tokens: u64 = 0;
    for part in parts {
        if let Some(t) = part.as_str() {
            tokens += heuristic_token_count(t.len() as u64);
            continue;
        }
        let typ = part.get("type").and_then(|t| t.as_str());
        if typ == Some("image")
            || ((part.get("url").is_some() || part.get("data").is_some())
                && part.get("text").is_none()
                && part.get("name").is_none())
        {
            tokens += heuristic_token_count(4800);
            continue;
        }
        match typ {
            Some("text") | Some("thinking") | None => {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    tokens += heuristic_token_count(text.len() as u64);
                } else {
                    tokens += heuristic_token_count(part.to_string().len() as u64);
                }
            }
            _ => {
                tokens += heuristic_token_count(part.to_string().len() as u64);
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
        SessionEntry::BranchSummary(_) => true,
        SessionEntry::CustomMessage(_) => true,
        SessionEntry::Custom(c) => c.custom_type == "custom_message",
        _ => false,
    }
}

/// Whether this entry opens a new user turn.
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
        // pi: stop when previous entry contributes context messages.
        if estimate_tokens_entry_for_cut(prev) > 0 {
            break;
        }
        match prev {
            SessionEntry::Message(_) => break,
            SessionEntry::BranchSummary(_) | SessionEntry::CustomMessage(_) => break,
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
        let message_tokens = estimate_tokens_entry_for_cut(entry);
        if message_tokens == 0 {
            continue;
        }
        accumulated += message_tokens;

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
