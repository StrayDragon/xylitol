//! Session → LLM projection: fold environment roles into [`xylitol_plugin_sdk::XyLlmMessage`] rows.
//!
//! [`xylitol_plugin_sdk::AgentMessage`] remains the session SSOT (`Llm` ∪ `Env`). Before any
//! provider call, history MUST pass through [`project_for_llm`].
//! Llm arm is passthrough (`XyLlmMessage`).
//!
//! Env fold **shapes** are part of the resume/import provider-prefix contract
//! (c1930 / as48): changing the bash or context-summary templates breaks
//! Responses `input` byte prefixes and prompt cache. Edit only via explicit change.
//!
//! Request-time (non-transcript) tails — AgentStatusBar readings — are
//! **not** this module. They append after the fold via
//! `crate::agent::prompt::status_bar`.

pub use xylitol_plugin_sdk::llm_project::{
    fold_bash_for_llm, fold_context_summary_for_llm, project_for_llm, user_text,
};

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, EnvMessage, XyLlmMessage, now_ms};

    /// Zero every timestamp so the snapshot pins shape, not wall-clock.
    fn normalize_timestamps(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if map.contains_key("timestamp") {
                    map.insert("timestamp".into(), serde_json::Value::from(0));
                }
                for child in map.values_mut() {
                    normalize_timestamps(child);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    normalize_timestamps(item);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn full_projection_shape_snapshot() {
        // One conversation touching every projection arm. The serialized shape
        // is a prompt-cache prefix contract (c1930/as48): treat snapshot diffs
        // like wire diffs and land them only via an explicit change.
        let history = vec![
            AgentMessage::user("plan the refactor"),
            AgentMessage::assistant("I will read the entry first"),
            AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "call_1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "src/main.rs"}),
                }],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 0,
                diagnostics: Vec::new(),
            }),
            AgentMessage::tool_result(
                "call_1",
                "read",
                vec![AgentPart::text("fn main() {}")],
                false,
            ),
            AgentMessage::tool_result("call_2", "read", vec![AgentPart::text("boom")], true),
            AgentMessage::bash("ls", "a.txt", Some(0)),
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                bash_id: String::new(),
                command: "secret".into(),
                output: "x".into(),
                exit_code: None,
                cancelled: false,
                truncated: false,
                full_output_path: None,
                exclude_from_context: true,
                status: xylitol_plugin_sdk::message::BashExecutionStatus::Done,
            }),
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "earlier work summarized".into(),
                tokens_before: 20_000,
                tokens_after: 4_000,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Env(EnvMessage::CustomMessage {
                custom_type: "note".into(),
                content: serde_json::Value::String("user pinned note".into()),
                display: serde_json::Value::Null,
                details: serde_json::Value::Null,
            }),
        ];
        let projected = project_for_llm(&history);
        let mut value = serde_json::to_value(&projected).unwrap();
        normalize_timestamps(&mut value);
        let pretty = serde_json::to_string_pretty(&value).unwrap();
        insta::assert_snapshot!(pretty);
    }

    #[test]
    fn agent_todo_custom_skipped_in_llm_prefix() {
        use xylitol_plugin_sdk::session::{
            CUSTOM_TYPE_AGENT_TODO, CustomEntry, EntryBase, SessionEntry, TodoItem, TodoList,
            TodoStatus,
        };
        let list = TodoList::new(vec![TodoItem {
            id: "1".into(),
            content: "secret todo body".into(),
            status: TodoStatus::InProgress,
        }]);
        let entry = SessionEntry::Custom(CustomEntry {
            base: EntryBase {
                entry_type: "custom".into(),
                id: "t1".into(),
                parent_id: None,
                timestamp: 0,
            },
            custom_type: CUSTOM_TYPE_AGENT_TODO.into(),
            data: list.to_data_value(),
        });
        assert!(entry.as_agent_message().is_none());
        let history = vec![
            AgentMessage::user("hi"),
            // Only messages that survive as_agent_message reach project_for_llm.
        ];
        let projected = project_for_llm(&history);
        let blob = serde_json::to_string(&projected).unwrap();
        assert!(!blob.contains("secret todo body"));
        assert!(!blob.contains(CUSTOM_TYPE_AGENT_TODO));
    }

    #[test]
    fn bash_folds_to_user_llm() {
        let history = vec![
            AgentMessage::user("hi"),
            AgentMessage::bash("ls", "a.txt", Some(0)),
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 2);
        assert_eq!(projected[1].role_name(), "user");
        assert_eq!(
            projected[1].text(),
            fold_bash_for_llm("ls", "a.txt"),
            "bash fold shape is cache-prefix stable (c1930)"
        );
    }

    #[test]
    fn bash_excluded_from_context_is_skipped() {
        let history = vec![AgentMessage::Env(EnvMessage::BashExecutionMessage {
            bash_id: String::new(),
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
            status: xylitol_plugin_sdk::message::BashExecutionStatus::Done,
        })];
        assert!(project_for_llm(&history).is_empty());
    }

    #[test]
    fn compaction_and_branch_fold() {
        let history = vec![
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "compressed".into(),
                tokens_before: 100,
                tokens_after: 10,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Env(EnvMessage::BranchSummaryMessage {
                summary: "forked".into(),
                from_id: "abc".into(),
            }),
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 2);
        assert_eq!(
            projected[0].text(),
            fold_context_summary_for_llm("compressed"),
            "compaction fold shape is cache-prefix stable (c1930)"
        );
        assert_eq!(
            projected[1].text(),
            fold_context_summary_for_llm("forked"),
            "branch-summary fold shape is cache-prefix stable (c1930)"
        );
    }

    #[test]
    fn tool_result_passthrough_as_llm() {
        let history = vec![AgentMessage::tool_result(
            "c1",
            "read",
            vec![AgentPart::text("ok")],
            false,
        )];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].text(), "ok");
    }

    /// After /reload removes an MCP tool, past ToolCall + toolResult MUST still
    /// project into the next LLM prefix (history is not filtered by live tools).
    #[test]
    fn history_keeps_removed_mcp_tool_call_in_llm_projection() {
        let history = vec![
            AgentMessage::user("use ping"),
            AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![AgentPart::ToolCall {
                    id: "call-gone".into(),
                    name: "mcp__fixture__ping".into(),
                    arguments: serde_json::json!({}),
                }],
                stop_reason: Some(xylitol_plugin_sdk::message::XyStopReason::ToolUse),
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: 1,
                diagnostics: Vec::new(),
            }),
            AgentMessage::tool_result(
                "call-gone",
                "mcp__fixture__ping",
                vec![AgentPart::text("pong")],
                false,
            ),
            AgentMessage::user("again?"),
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 4, "must keep full prefix: {projected:?}");
        let asst = &projected[1];
        assert!(
            asst.content().iter().any(
                |p| matches!(p, AgentPart::ToolCall { name, .. } if name == "mcp__fixture__ping")
            ),
            "assistant ToolCall for removed tool MUST remain in LLM projection"
        );
        assert_eq!(projected[2].role_name(), "toolResult");
        assert_eq!(projected[2].text(), "pong");
    }

    #[test]
    fn edit_tool_result_content_short_details_not_in_text() {
        let history = vec![AgentMessage::tool_result_with_details(
            "e1",
            "edit",
            vec![AgentPart::text("Successfully replaced 1 block(s) in a.rs.")],
            Some(serde_json::json!({
                "success": true,
                "display_diff": "big-diff-wall",
            })),
            false,
        )];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].role_name(), "toolResult");
        assert!(projected[0].text().contains("Successfully replaced"));
        assert!(!projected[0].text().contains("display_diff"));
        assert!(!projected[0].text().contains("big-diff-wall"));
    }

    #[test]
    fn aborted_and_error_assistants_skipped_for_llm() {
        use xylitol_plugin_sdk::message::XyStopReason;

        let aborted = AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            content: vec![AgentPart::text("partial draft")],
            stop_reason: Some(XyStopReason::Aborted),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: now_ms(),
            diagnostics: Vec::new(),
        });
        let errored = AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            content: vec![AgentPart::text("failed")],
            stop_reason: Some(XyStopReason::Error),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: now_ms(),
            diagnostics: Vec::new(),
        });
        let ok = AgentMessage::assistant("kept");
        let history = vec![
            AgentMessage::user("hi"),
            aborted,
            AgentMessage::user("again"),
            errored,
            ok,
        ];
        let projected = project_for_llm(&history);
        assert_eq!(projected.len(), 3);
        assert_eq!(projected[0].role_name(), "user");
        assert_eq!(projected[1].role_name(), "user");
        assert_eq!(projected[2].role_name(), "assistant");
        assert_eq!(projected[2].text(), "kept");
    }

    #[test]
    fn llm_passthrough_is_identity() {
        let user = AgentMessage::user("hi");
        let projected = project_for_llm(std::slice::from_ref(&user));
        assert_eq!(projected.len(), 1);
        // Same type as bridge DTO (alias).
        let _: xylitol_plugin_sdk::XyLlmMessage = projected[0].clone();
    }

    /// c27 (agent seam): post-compact working history is CompactionSummary + firstKept.
    /// Summarized-away assistants are absent from the leaf — projection must not
    /// resurrect their Thinking; kept assistants with signature still pass through.
    #[test]
    fn compact_working_history_projects_summary_and_keeps_retained_signature() {
        let kept_sig = r#"{"type":"reasoning","id":"rs_kept","summary":[]}"#;
        let working = vec![
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "prior turns summarized".into(),
                tokens_before: 9_000,
                tokens_after: 400,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::user("continue after compact"),
            AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                content: vec![
                    AgentPart::Thinking {
                        thinking: "kept plan".into(),
                        redacted: false,
                        thinking_signature: Some(kept_sig.into()),
                    },
                    AgentPart::text("kept reply"),
                ],
                stop_reason: None,
                usage: None,
                api: String::new(),
                provider: String::new(),
                model: String::new(),
                response_id: None,
                error_message: None,
                timestamp: now_ms(),
                diagnostics: Vec::new(),
            }),
        ];
        let projected = project_for_llm(&working);
        assert_eq!(projected.len(), 3);
        assert_eq!(
            projected[0].text(),
            fold_context_summary_for_llm("prior turns summarized"),
            "compaction folds to context summary user row"
        );
        // No synthesized assistant for summarized-away turns (would be a 4th+ row).
        assert!(
            projected
                .iter()
                .filter(|m| m.role_name() == "assistant")
                .count()
                == 1,
            "only kept assistant remains: {projected:?}"
        );
        match &projected[2] {
            XyLlmMessage::AssistantMessage { content, .. } => {
                let sig = content.iter().find_map(|p| match p {
                    AgentPart::Thinking {
                        thinking_signature: Some(s),
                        ..
                    } => Some(s.as_str()),
                    _ => None,
                });
                assert_eq!(
                    sig,
                    Some(kept_sig),
                    "retained thinkingSignature must survive project_for_llm"
                );
                assert!(
                    !content.iter().any(|p| matches!(
                        p,
                        AgentPart::Thinking {
                            thinking_signature: Some(s),
                            ..
                        } if s.contains("rs_summarized_away")
                    )),
                    "must not invent summarized-away signature: {content:?}"
                );
            }
            other => panic!("expected kept assistant, got {other:?}"),
        }
    }
}
