//! Cut-point detection — where to compact session entries.
//!
//! Walk session entries backwards from newest, accumulate token estimates,
//! and find the nearest valid boundary (user / assistant / bash / custom / branch).

use crate::protocol::message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};
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

/// pi `ESTIMATED_IMAGE_CHARS` — counted as chars before `/4`.
const ESTIMATED_IMAGE_CHARS: u64 = 4800;

/// Estimate tokens for cut-point walking (pi `estimateTokens` on AgentMessage).
///
/// Returns 0 when the entry has no context-visible message (skipped in accumulation).
/// Message rows that fail typed deserialize still use a lax content walk (string or
/// parts) so cut math stays usable for legacy / fixture wire shapes.
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
                    estimate_tokens_message_for_cut(&agent_msg)
                }
                Err(_) => estimate_lax_message_json_chars(&msg.message).div_ceil(4),
            }
        }
        _ => entry
            .as_agent_message()
            .map(|m| estimate_tokens_message_for_cut(&m))
            .unwrap_or(0),
    }
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

/// pi-aligned chars/4 estimate for a single transcript message.
pub fn estimate_tokens_message_for_cut(msg: &AgentMessage) -> u64 {
    let chars = match msg {
        // pi user / toolResult / custom: text + image only
        AgentMessage::Llm(LlmMessage::UserMessage { content, .. })
        | AgentMessage::Llm(LlmMessage::ToolResultMessage { content, .. }) => {
            estimate_text_and_image_chars(content)
        }
        // pi assistant: text + thinking + toolCall (not image)
        AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. }) => {
            estimate_assistant_chars(content)
        }
        AgentMessage::Env(EnvMessage::BashExecutionMessage {
            command, output, ..
        }) => (command.len() + output.len()) as u64,
        AgentMessage::Env(EnvMessage::CompactionSummaryMessage { summary, .. })
        | AgentMessage::Env(EnvMessage::BranchSummaryMessage { summary, .. }) => {
            summary.len() as u64
        }
        AgentMessage::Env(EnvMessage::CustomMessage { content, .. }) => {
            estimate_custom_content_chars(content)
        }
    };
    chars.div_ceil(4)
}

fn estimate_text_and_image_chars(parts: &[AgentPart]) -> u64 {
    let mut chars = 0u64;
    for part in parts {
        match part {
            AgentPart::Text { text } => chars += text.len() as u64,
            AgentPart::Image(_) => chars += ESTIMATED_IMAGE_CHARS,
            AgentPart::Thinking { .. } | AgentPart::ToolCall { .. } => {}
        }
    }
    chars
}

fn estimate_assistant_chars(parts: &[AgentPart]) -> u64 {
    let mut chars = 0u64;
    for part in parts {
        match part {
            AgentPart::Text { text } => chars += text.len() as u64,
            AgentPart::Thinking { thinking, .. } => chars += thinking.len() as u64,
            AgentPart::ToolCall {
                name, arguments, ..
            } => {
                chars += name.len() as u64 + arguments.to_string().len() as u64;
            }
            AgentPart::Image(_) => {}
        }
    }
    chars
}

fn estimate_custom_content_chars(content: &serde_json::Value) -> u64 {
    if let Some(s) = content.as_str() {
        return s.len() as u64;
    }
    if let Some(parts) = content.as_array() {
        let mut chars = 0u64;
        for part in parts {
            let typ = part.get("type").and_then(|t| t.as_str());
            if typ == Some("image") {
                chars += ESTIMATED_IMAGE_CHARS;
            } else if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                chars += text.len() as u64;
            } else if let Some(s) = part.as_str() {
                chars += s.len() as u64;
            }
        }
        return chars;
    }
    content.to_string().len() as u64
}

/// Estimate tokens for a single `SessionEntry` using chars/4 heuristic.
///
/// Prefer [`estimate_tokens_entry_for_cut`] for cut-point walking (pi-aligned).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::{
        BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry, EntryBase,
        LabelEntry, MessageEntry, SESSION_VERSION, SessionEntry, SessionHeader,
    };
    use serde_json::json;

    fn msg_entry(id: &str, role: &str, content: &str) -> SessionEntry {
        SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            message: json!({"role": role, "content": content}),
        })
    }
    fn compaction_entry(summary: &str) -> SessionEntry {
        SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c".into(),
                parent_id: None,
                timestamp: 0,
            },
            summary: summary.into(),
            first_kept_entry_id: String::new(),
            tokens_before: 0,
            details: None,
            from_hook: None,
        })
    }
    fn label_entry() -> SessionEntry {
        SessionEntry::Label(LabelEntry {
            base: EntryBase {
                entry_type: "label".into(),
                id: "l".into(),
                parent_id: None,
                timestamp: 0,
            },
            target_id: "m1".into(),
            label: None,
        })
    }
    fn header_entry() -> SessionEntry {
        SessionEntry::Header(SessionHeader {
            entry_type: String::new(),
            version: SESSION_VERSION,
            id: "s".into(),
            timestamp: 0,
            cwd: String::new(),
            parent_session: None,
            fork_at_entry_id: None,
        })
    }
    fn custom_entry_of(custom_type: &str) -> SessionEntry {
        SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: "x".into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: custom_type.into(),
            data: json!({"k": 1}),
        })
    }
    fn branch_summary_entry() -> SessionEntry {
        SessionEntry::BranchSummary(BranchSummaryEntry {
            base: EntryBase {
                entry_type: "branchSummary".into(),
                id: "b".into(),
                parent_id: None,
                timestamp: 0,
            },
            from_id: "root".into(),
            summary: "s".into(),
            details: None,
            from_hook: None,
        })
    }
    fn custom_message_entry() -> SessionEntry {
        SessionEntry::CustomMessage(CustomMessageEntry {
            base: EntryBase {
                entry_type: "customMessage".into(),
                id: "cm".into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: "note".into(),
            content: json!("hi"),
            display: false,
            details: None,
        })
    }

    // ── estimate_lax_message_json_chars（chars 口径）────────────────

    #[test]
    fn lax_string_content_is_byte_len() {
        // .len() 是字节口径：CJK 按字节计（4 个汉字 = 12 字节），这是现状契约。
        assert_eq!(
            estimate_lax_message_json_chars(&json!({"content": "hello"})),
            5
        );
        assert_eq!(
            estimate_lax_message_json_chars(&json!({"content": "你好世界"})),
            12
        );
        assert_eq!(estimate_lax_message_json_chars(&json!({"content": ""})), 0);
    }

    #[test]
    fn lax_part_array_table() {
        let cases = [
            // (描述, message, 期望 chars)
            (
                "text part",
                json!({"role":"user","content":[{"type":"text","text":"abc"}]}),
                3,
            ),
            (
                "parts 键回退",
                json!({"parts":[{"type":"text","text":"ab"}]}),
                2,
            ),
            ("无 content/parts → 0", json!({"role":"user"}), 0),
            (
                "image 固定 4800",
                json!({"role":"user","content":[{"type":"image","url":"x"}]}),
                4800,
            ),
            (
                "assistant thinking 计入",
                json!({"role":"assistant","content":[{"type":"thinking","thinking":"abcd"}]}),
                4,
            ),
            (
                "非 assistant thinking 不计入",
                json!({"role":"user","content":[{"type":"thinking","thinking":"abcd"}]}),
                0,
            ),
            (
                "assistant toolCall = name + arguments 序列化",
                json!({"role":"assistant","content":[
                    {"type":"toolCall","name":"read","arguments":{"path":"a"}}
                ]}),
                4 + r#""path":"a""#.len() as u64 + 2, // {"path":"a"} = 12 字节
            ),
            (
                "toolCall 在非 assistant 角色不计入",
                json!({"role":"toolResult","content":[
                    {"type":"toolCall","name":"read","arguments":{}}
                ]}),
                0,
            ),
            ("裸字符串 part", json!({"role":"user","content":["abc"]}), 3),
            (
                "未知 type 忽略",
                json!({"role":"user","content":[{"type":"other","text":"zzzz"}]}),
                0,
            ),
            ("空 parts", json!({"role":"assistant","content":[]}), 0),
        ];
        for (desc, msg, expected) in cases {
            assert_eq!(
                estimate_lax_message_json_chars(&msg),
                expected,
                "case: {desc}"
            );
        }
    }

    // ── estimate_tokens_message_json（tokens = chars/4 上取整）──────

    #[test]
    fn tokens_string_content_div_ceil() {
        assert_eq!(
            estimate_tokens_message_json(&json!({"content": "hello"})),
            2
        ); // 5→2
        assert_eq!(estimate_tokens_message_json(&json!({"content": "abcd"})), 1); // 4→1
        assert_eq!(estimate_tokens_message_json(&json!({"content": ""})), 0);
    }

    #[test]
    fn tokens_part_array_table() {
        let cases = [
            ("字符串 part", json!({"content":["abcdef"]}), 2u64), // 6→2
            (
                // 注意：tokens 变体把 image 直接记为 4800「token」，而 lax
                // 变体记 4800「char」（÷4 后=1200 token）——两口径相差 4 倍。
                // 此处按现状钉住；是否统一属显式行为决策，勿顺手改。
                "image 形态 → 直接 4800 token",
                json!({"content":[{"type":"image"}]}),
                4800,
            ),
            (
                "有 url 无 text/name 同样直接 4800 token",
                json!({"content":[{"url":"http://x"}]}),
                4800,
            ),
            (
                "url 带 text 则按 text 计",
                json!({"content":[{"url":"http://x","text":"abcdefgh"}]}),
                2,
            ),
            (
                "thinking 与 lax 不同：不区分角色",
                json!({"role":"user","content":[{"type":"thinking","text":"abcdefgh"}]}),
                2,
            ),
            (
                "未知 type 用整个 part 序列化长度",
                json!({"content":[{"type":"zzz","x":"yy"}]}),
                {
                    let part = json!({"type":"zzz","x":"yy"});
                    (part.to_string().len() as u64).div_ceil(4)
                },
            ),
            (
                "缺 content → 整个消息 JSON 长度",
                json!({"foo":1}),
                (r#"{"foo":1}"#.len() as u64).div_ceil(4),
            ),
        ];
        for (desc, msg, expected) in cases {
            assert_eq!(estimate_tokens_message_json(&msg), expected, "case: {desc}");
        }
    }

    #[test]
    fn entry_level_estimates_table() {
        use crate::protocol::session::{CompactionEntry, EntryBase, MessageEntry, SessionEntry};
        let base = |id: &str| EntryBase {
            entry_type: id.into(),
            id: id.into(),
            parent_id: None,
            timestamp: 0,
        };
        let header = SessionEntry::Header(crate::protocol::session::SessionHeader {
            entry_type: String::new(),
            version: SESSION_VERSION,
            id: "s".into(),
            timestamp: 0,
            cwd: String::new(),
            parent_session: None,
            fork_at_entry_id: None,
        });
        assert_eq!(estimate_tokens_entry(&header), 0);
        let compaction = SessionEntry::Compaction(CompactionEntry {
            base: base("c1"),
            summary: "abcdefgh".into(), // 8 → 2
            first_kept_entry_id: String::new(),
            tokens_before: 0,
            details: None,
            from_hook: None,
        });
        assert_eq!(estimate_tokens_entry(&compaction), 2);
        let message = SessionEntry::Message(MessageEntry {
            base: base("m1"),
            message: json!({"role": "user", "content": "hello"}),
        });
        assert_eq!(estimate_tokens_entry(&message), 2);
    }

    // ── estimate_custom_content_chars ───────────────────────────────

    #[test]
    fn custom_content_chars_table() {
        let cases = [
            ("字符串直取", json!("abcd"), 4u64),
            ("数组内 image", json!([{"type":"image"}]), 4800),
            ("数组内 text part", json!([{"text":"ab"}]), 2),
            ("数组内裸字符串", json!(["xyz"]), 3),
            ("空数组", json!([]), 0),
            (
                "非数组非字符串 → 整体序列化",
                json!({"k":1}),
                r#"{"k":1}"#.len() as u64,
            ),
        ];
        for (desc, content, expected) in cases {
            assert_eq!(
                estimate_custom_content_chars(&content),
                expected,
                "case: {desc}"
            );
        }
    }

    // ── is_valid_cut_point / is_turn_start_entry ────────────────────

    #[test]
    fn valid_cut_point_table() {
        let user = msg_entry("u", "user", "hi");
        let assistant = msg_entry("a", "assistant", "yo");
        let tool_result = msg_entry("t", "toolResult", "out");
        let unknown = msg_entry("?", "mystery", "?");
        let cases = [
            (&user, true),
            (&assistant, true),
            (&msg_entry("b", "bashExecution", "!ls"), true),
            (&msg_entry("c", "custom", "x"), true),
            (&msg_entry("bs", "branchSummary", "s"), true),
            (&msg_entry("cs", "compactionSummary", "s"), true),
            (&tool_result, false), // 永不在 toolResult 后切开
            (&unknown, false),
            (&branch_summary_entry(), true),
            (&custom_message_entry(), true),
            (&custom_entry_of("custom_message"), true),
            (&custom_entry_of("other_kind"), false),
            (&header_entry(), false),
            (&label_entry(), false),
            (&compaction_entry("s"), false),
        ];
        for (entry, expected) in cases {
            assert_eq!(is_valid_cut_point(entry), expected, "entry role/type");
        }
    }

    #[test]
    fn turn_start_table() {
        let cases = [
            (msg_entry("u", "user", "hi"), true),
            (msg_entry("b", "bashExecution", "!ls"), true),
            (msg_entry("a", "assistant", "yo"), false),
            (msg_entry("t", "toolResult", "out"), false),
            (compaction_entry("s"), false), // 显式早退：压缩条目永不开新 turn
            (branch_summary_entry(), true),
            (custom_message_entry(), true),
            (custom_entry_of("custom_message"), true),
            (custom_entry_of("other"), false),
            (header_entry(), false),
        ];
        for (entry, expected) in cases {
            assert_eq!(is_turn_start_entry(&entry), expected);
        }
    }

    // ── include_preceding_non_messages ──────────────────────────────

    #[test]
    fn preceding_zero_token_entries_are_included_until_boundary() {
        // [user_a, header, label, user_b] cut=3：零 token 的 header/label
        // 被并入保留窗，遇上有 token 的 user_a 截停 → 保留从 1 开始。
        let entries = vec![
            msg_entry("u1", "user", "first"),
            header_entry(),
            label_entry(),
            msg_entry("u2", "user", "hi"),
        ];
        assert_eq!(include_preceding_non_messages(&entries, 3, 0), 1);
    }

    #[test]
    fn preceding_walk_stops_at_compaction_and_at_token_entries() {
        // 压缩边界挡住回溯（中间的零 token 条目被并入）。
        let entries = vec![
            compaction_entry("s"),
            header_entry(),
            label_entry(),
            msg_entry("u", "user", "hi"),
        ];
        assert_eq!(include_preceding_non_messages(&entries, 3, 0), 1);
        // 有 token 的前驱本身就是保留起点。
        let entries = vec![
            msg_entry("u1", "user", "first"),
            msg_entry("u2", "user", "second"),
        ];
        assert_eq!(include_preceding_non_messages(&entries, 1, 0), 1);
        // 尊重 start_index 下限。
        let entries = vec![header_entry(), label_entry(), msg_entry("u", "user", "hi")];
        assert_eq!(include_preceding_non_messages(&entries, 2, 2), 2);
    }

    // ── find_cut_point 行为场景 ─────────────────────────────────────

    #[test]
    fn find_cut_point_empty_and_degenerate() {
        let empty: Vec<SessionEntry> = Vec::new();
        let r = find_cut_point(&empty, 0, 0, 100);
        assert_eq!(
            (
                r.first_kept_entry_index,
                r.turn_start_index,
                r.is_split_turn
            ),
            (0, -1, false)
        );

        let entries = vec![
            msg_entry("u", "user", "hi"),
            msg_entry("a", "assistant", "yo"),
        ];
        let r = find_cut_point(&entries, 2, 2, 10);
        assert_eq!(
            (
                r.first_kept_entry_index,
                r.turn_start_index,
                r.is_split_turn
            ),
            (2, -1, false)
        );
    }

    #[test]
    fn find_cut_point_no_valid_cut_points_falls_back_to_start() {
        // 只有 toolResult：永不成为切点。
        let entries = vec![
            msg_entry("t1", "toolResult", "out"),
            msg_entry("t2", "toolResult", "out2"),
        ];
        let r = find_cut_point(&entries, 0, 2, 1);
        assert_eq!(
            (
                r.first_kept_entry_index,
                r.turn_start_index,
                r.is_split_turn
            ),
            (0, -1, false)
        );
    }

    #[test]
    fn find_cut_point_splits_mid_turn_with_turn_start_hint() {
        // [user(4tok), assistant(4tok), user(4tok)]，keep=6 → 累计在 assistant 处
        // 达标 → 切点落到 mid-turn → 返回 turn_start 提示恢复完整 turn。
        let entries = vec![
            msg_entry("u1", "user", "abcdefghijklmnop"), // 16 chars → 4 tok
            msg_entry("a1", "assistant", "abcdefghijklmnop"), // 4 tok
            msg_entry("u2", "user", "abcdefghijklmnop"), // 4 tok
        ];
        let r = find_cut_point(&entries, 0, 3, 6);
        assert_eq!(r.first_kept_entry_index, 1);
        assert_eq!(r.turn_start_index, 0);
        assert!(r.is_split_turn);
    }

    #[test]
    fn find_cut_point_on_turn_start_preserves_non_split_semantics() {
        // keep=4 → 累计恰好在最后一个 user 达标 → 切点即 turn-start：
        // 即使 include_preceding 回移，也不得标记为 split。
        let entries = vec![
            msg_entry("u1", "user", "abcdefghijklmnop"),
            msg_entry("a1", "assistant", "abcdefghijklmnop"),
            msg_entry("u2", "user", "abcdefghijklmnop"),
        ];
        let r = find_cut_point(&entries, 0, 3, 4);
        assert_eq!(r.first_kept_entry_index, 2);
        assert_eq!(r.turn_start_index, -1);
        assert!(!r.is_split_turn);
    }

    #[test]
    fn find_cut_point_respects_compaction_boundary_when_including() {
        // [user, compaction, user]，小 keep → 切点=2；向前并入被压缩边界挡住。
        let entries = vec![
            msg_entry("u1", "user", "first"),
            compaction_entry("summary"),
            msg_entry("u2", "user", "abcdefghijklmnop"),
        ];
        let r = find_cut_point(&entries, 0, 3, 4);
        assert_eq!(r.first_kept_entry_index, 2);
        assert!(!r.is_split_turn);
    }
}
