use xylitol_plugin_sdk::accounting::heuristic_token_count;
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::session::SessionEntry;

#[derive(Debug, Clone)]
pub struct CutPointResult {
    pub first_kept_entry_index: usize,
    pub turn_start_index: isize,
    pub is_split_turn: bool,
}

const ESTIMATED_IMAGE_CHARS: u64 = 4800;

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
                Err(_) => heuristic_token_count(estimate_lax_message_json_chars(&msg.message)),
            }
        }
        _ => entry
            .as_agent_message()
            .map(|m| unified_projected_entry_token_estimate(&m))
            .unwrap_or(0),
    }
}

fn unified_projected_entry_token_estimate(agent_msg: &AgentMessage) -> u64 {
    let image_tokens = match agent_msg {
        AgentMessage::Llm(XyLlmMessage::UserMessage { content, .. })
        | AgentMessage::Llm(XyLlmMessage::ToolResultMessage { content, .. }) => {
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
