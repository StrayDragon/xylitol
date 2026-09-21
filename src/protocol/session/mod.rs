//! Session entry vocabulary — shared persisted shapes (not runtime capabilities).
//!
//! Pure vocabulary (serde types only): both `agent` (compaction, export) and
//! `infra` (session manager, persistence) reference these. Storage backends
//! stay in `infra::session`.

mod entries;
mod helpers;
mod manifest;
mod parse;
mod todo;
mod tree;

pub use entries::{
    BranchSummaryEntry, CompactionEntry, CompactionPolicySnapshot, CustomEntry, CustomMessageEntry,
    EntryBase, ForkPosition, LabelEntry, MessageEntry, ModelChangeEntry, SESSION_VERSION,
    SessionContext, SessionEntry, SessionHeader, SessionInfoEntry, ThinkingLevelChangeEntry,
    build_context_entries, done_bash_ids, fold_interrupted_bash_rows,
};
pub use helpers::{
    bash_execution_message_entry, count_tool_calls, fixture_message_json, is_assistant_message,
    is_env_custom_message, is_tool_call_part, is_user_message, message_custom_type, message_parts,
    message_role, message_text, session_fork_edge, tool_call_name, tool_file_paths,
    transcript_ancestry_ids, transcript_leaf_anchor,
};
pub use manifest::{SealedIndex, SessionManifest, SessionSegment};
pub use parse::{
    enforce_legacy_session_version, enforce_session_version, parse_session_jsonl,
    parse_session_jsonl_lines,
};
#[cfg(test)]
pub use todo::TodoItem;
pub use todo::{
    CUSTOM_TYPE_AGENT_TODO, TodoItemDraft, TodoItemPatch, TodoList, TodoStatus, apply_todo_patches,
    latest_agent_todo, normalize_rewrite_items,
};
pub use tree::{
    SessionTreeKind, SessionTreeNode, SessionTreeTravel, build_session_tree,
    plan_message_history_travel,
};

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
                timestamp: entry_ms(id),
            },
            message: fixture_message_json(role, text),
        })
    }

    /// Deterministic unix-ms from a string id (test fixture).
    fn entry_ms(id: &str) -> u64 {
        1_781_827_200_000u64
            + id.as_bytes()
                .iter()
                .fold(0u64, |acc, b| acc * 31 + *b as u64)
                % 1_000_000
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
            timestamp: 0,
            cwd: "/tmp".into(),
            parent_session: Some("p".into()),
            fork_at_entry_id: None,
        });
        let v = serde_json::to_value(&header).unwrap();
        assert_eq!(v["version"], 7);
        assert_eq!(v["parentSession"], "p");
        assert!(v.get("forkAtEntryId").is_none());

        let with_cut = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: "s1".into(),
            timestamp: 0,
            cwd: "/tmp".into(),
            parent_session: Some("p".into()),
            fork_at_entry_id: Some("u6".into()),
        });
        let v = serde_json::to_value(&with_cut).unwrap();
        assert_eq!(v["forkAtEntryId"], "u6");

        let old = r#"{"type":"session","version":7,"id":"s1","timestamp":0,"cwd":"/tmp","parentSession":"p"}"#;
        let parsed: SessionEntry = serde_json::from_str(old).unwrap();
        let SessionEntry::Header(h) = parsed else {
            panic!("header");
        };
        assert_eq!(h.parent_session.as_deref(), Some("p"));
        assert_eq!(h.fork_at_entry_id, None);

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
    fn message_text_ignores_legacy_parts_only_shape() {
        let msg = json!({
            "role": "user",
            "parts": [{ "type": "text", "text": "legacy" }],
        });
        assert_eq!(
            message_text(&msg),
            "",
            "canonical shape is `content`; parts-only MUST NOT be read"
        );
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
    fn count_tool_calls_ignores_untagged_part_shapes() {
        let msg = json!({
            "role": "assistant",
            "content": [{
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
                "timestamp": 0,
                "cwd": "/tmp"
            }),
            serde_json::json!({
                "type": "message",
                "id": "m1",
                "timestamp": 0,
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
        let content = r#"{"type":"session","version":4,"id":"s1","timestamp":0,"cwd":"/tmp"}
"#;
        let err = parse_session_jsonl(content).unwrap_err();
        assert!(err.to_string().contains("not supported"), "{err}");
    }

    #[test]
    fn parse_session_jsonl_rejects_v5_with_require_7_message() {
        // v5 disk is refused (only v6 receives the one-time migration); the
        // version message must report the current format.
        let content = r#"{"type":"session","version":5,"id":"s1","timestamp":0,"cwd":"/tmp"}
"#;
        let err = parse_session_jsonl(content).unwrap_err();
        assert!(
            err.to_string().contains("require 7")
                || err.to_string().contains("require {SESSION_VERSION}"),
            "v5 refusal must carry current version: {err}"
        );
        assert!(
            err.to_string().contains("5"),
            "must name the offending version: {err}"
        );
    }

    #[test]
    fn current_entry_round_trip_keeps_unix_ms_timestamps() {
        // s22: header + entry shell timestamps are u64 unix-ms on the wire.
        let header = SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: "s1".into(),
            timestamp: 1_781_827_200_000,
            cwd: "/tmp".into(),
            parent_session: Some("p".into()),
            fork_at_entry_id: None,
        });
        let raw_header = serde_json::to_string(&header).unwrap();
        assert!(
            raw_header.contains("\"timestamp\":1781827200000"),
            "header must serialize timestamp as u64 ms: {raw_header}"
        );
        let parsed: Vec<SessionEntry> =
            parse_session_jsonl(&format!("{raw_header}\n")).expect("current-format round-trip");
        let SessionEntry::Header(h) = &parsed[0] else {
            panic!("expected header");
        };
        assert_eq!(h.timestamp, 1_781_827_200_000);

        let msg = msg_entry("e1", Some("p1"), "user", "hi");
        let raw_msg = serde_json::to_string(&msg).unwrap();
        let back: SessionEntry = serde_json::from_str(&raw_msg).unwrap();
        assert_eq!(
            back.base().unwrap().timestamp,
            msg.base().unwrap().timestamp
        );
    }

    fn compaction_entry(id: &str, first_kept: &str, summary: &str) -> SessionEntry {
        SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: id.into(),
                parent_id: None,
                timestamp: 0,
            },
            summary: summary.into(),
            first_kept_entry_id: first_kept.into(),
            tokens_before: 1000,
            details: None,
            from_hook: None,
            policy: None,
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
                timestamp: 0,
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
            "excludeFromContext": true,
        }));
        assert!(e.as_agent_message().is_none());
    }

    #[test]
    fn snake_alias_no_longer_sets_excluded_flag() {
        // c2260: serde aliases removed. A snake `exclude_from_context` key is an
        // unknown field → ignored; the flag stays default (false), so the message
        // is NOT excluded (must not resurrect pre-fix semantics; s18).
        let e = entry(json!({
            "role": "bashExecution",
            "command": "secret",
            "output": "x",
            "cancelled": false,
            "truncated": false,
            "exclude_from_context": true,
        }));
        let msg = e.as_agent_message().expect("snake alias must be ignored");
        assert_eq!(msg.role_name(), "bashExecution");
    }

    #[test]
    fn compaction_projects_to_env() {
        let e = SessionEntry::Compaction(CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: 0,
            },
            summary: "sum".into(),
            first_kept_entry_id: "m1".into(),
            tokens_before: 100,
            details: None,
            from_hook: None,
            policy: None,
        });
        let msg = e.as_agent_message().expect("compaction");
        assert_eq!(msg.role_name(), "compactionSummary");
        assert!(msg.text().contains("sum"));
    }
}
