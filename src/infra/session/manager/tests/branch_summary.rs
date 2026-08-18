use super::super::*;
use crate::protocol::session::{EntryBase, MessageEntry, fixture_message_json};
use serde_json::json;

fn msg(id: &str, message: serde_json::Value) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: None,
            timestamp: 0,
        },
        message,
    })
}

#[test]
fn branch_summary_counts_content_tool_calls_and_plain_user_text() {
    let mgr = SessionManager::in_memory();
    let entries = vec![
        msg("u1", fixture_message_json("user", "你能做什么")),
        msg(
            "a1",
            json!({
                "role": "assistant",
                "content": [
                    { "type": "text", "text": "查文件" },
                    { "type": "toolCall", "id": "1", "name": "read", "arguments": { "path": "src/lib.rs" } }
                ],
                "timestamp": 0u64,
            }),
        ),
    ];
    let summary = mgr.generate_branch_summary(&entries);
    assert!(summary.contains("1 次工具调用"), "{summary}");
    assert!(summary.contains("src/lib.rs"), "{summary}");
    assert!(summary.contains("你能做什么"), "{summary}");
    assert!(!summary.contains("{\"content\""), "{summary}");
}
