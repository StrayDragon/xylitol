//! Session → LLM projection: fold environment roles into [`LlmMessage`] rows.
//!
//! [`AgentMessage`] remains the session SSOT (`Llm` ∪ `Env`). Before any
//! provider call, history MUST pass through [`project_for_llm`].
//! Llm arm is passthrough (`LlmMessage` ≡ bridge `AiBridgeMessage`).
//!
//! Env fold **shapes** are part of the resume/import provider-prefix contract
//! (c1930 / as48): changing the bash or context-summary templates breaks
//! Responses `input` byte prefixes and prompt cache. Edit only via explicit change.

use serde_json::Value;

use crate::protocol::message::{AgentMessage, AgentPart, EnvMessage, LlmMessage, now_ms};

/// Stable bash → LLM user-row fold (`$ {command}\n{output}`).
pub(crate) fn fold_bash_for_llm(command: &str, output: &str) -> String {
    format!("$ {command}\n{output}")
}

/// Stable compaction / branch summary → LLM user-row fold.
pub(crate) fn fold_context_summary_for_llm(summary: &str) -> String {
    format!("[Context summary: {summary}]")
}

/// Project session history into LLM-visible [`LlmMessage`] / `AiBridgeMessage` rows.
pub fn project_for_llm(messages: &[AgentMessage]) -> Vec<LlmMessage> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg {
            // c1595 / pi: aborted|error assistants stay in session scrollback but
            // MUST NOT be replayed to the next LLM call (`LlmMessage::is_error`).
            AgentMessage::Llm(m) => {
                if m.is_error() {
                    continue;
                }
                out.push(m.clone());
            }
            AgentMessage::Env(EnvMessage::BashExecutionMessage {
                command,
                output,
                exclude_from_context,
                ..
            }) => {
                if *exclude_from_context {
                    continue;
                }
                out.push(user_text(fold_bash_for_llm(command, output)));
            }
            AgentMessage::Env(
                EnvMessage::CompactionSummaryMessage { summary, .. }
                | EnvMessage::BranchSummaryMessage { summary, .. },
            ) => {
                out.push(user_text(fold_context_summary_for_llm(summary)));
            }
            AgentMessage::Env(EnvMessage::CustomMessage { content, .. }) => {
                if let Some(text) = custom_text(content)
                    && !text.is_empty()
                {
                    out.push(user_text(text));
                }
            }
        }
    }
    out
}

fn user_text(text: String) -> LlmMessage {
    LlmMessage::UserMessage {
        content: vec![AgentPart::text(text)],
        timestamp: now_ms(),
    }
}

fn custom_text(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::{AgentMessage, AgentPart, EnvMessage, LlmMessage};

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
            command: "secret".into(),
            output: "x".into(),
            exit_code: None,
            cancelled: false,
            truncated: false,
            full_output_path: None,
            exclude_from_context: true,
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
        use crate::protocol::message::XyStopReason;

        let aborted = AgentMessage::Llm(LlmMessage::AssistantMessage {
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
        let errored = AgentMessage::Llm(LlmMessage::AssistantMessage {
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
        let _: xylitol_ai_bridge::dto::AiBridgeMessage = projected[0].clone();
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
            AgentMessage::Llm(LlmMessage::AssistantMessage {
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
        assert!(
            projected[0].text().contains("prior turns summarized"),
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
            LlmMessage::AssistantMessage { content, .. } => {
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

    /// c1930 / as48+pab27: memory history → project → assemble is idempotent;
    /// JSONL render/parse → as_agent_message → same assemble `input`/`tools`.
    #[test]
    fn resume_import_shaped_jsonl_matches_memory_assemble_prefix() {
        use crate::protocol::session::{
            EntryBase, SESSION_VERSION, SessionEntry, SessionHeader, bash_execution_message_entry,
            parse_session_jsonl,
        };
        use xylitol_ai_bridge::AiBridgeGenerateOptions;
        use xylitol_ai_bridge::dto::AiBridgeToolSchema;
        use xylitol_ai_bridge::provider::ResponsesAssembler;

        let sig = r#"{"type":"reasoning","id":"rs_c1930","summary":[]}"#;
        let memory: Vec<AgentMessage> = vec![
            AgentMessage::user("hello"),
            AgentMessage::bash("pwd", "/tmp/lab", Some(0)),
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage {
                summary: "earlier turns".into(),
                tokens_before: 1000,
                tokens_after: 40,
                read_files: None,
                modified_files: None,
            }),
            AgentMessage::Llm(LlmMessage::AssistantMessage {
                content: vec![
                    AgentPart::Thinking {
                        thinking: "plan".into(),
                        redacted: false,
                        thinking_signature: Some(sig.into()),
                    },
                    AgentPart::text("done"),
                    AgentPart::ToolCall {
                        id: "call_1".into(),
                        name: "read".into(),
                        arguments: serde_json::json!({"path": "a.rs"}),
                    },
                ],
                stop_reason: None,
                usage: None,
                api: "openai-responses".into(),
                provider: "test".into(),
                model: "m".into(),
                response_id: None,
                error_message: None,
                timestamp: now_ms(),
                diagnostics: Vec::new(),
            }),
            AgentMessage::tool_result("call_1", "read", vec![AgentPart::text("ok")], false),
        ];

        let projected = project_for_llm(&memory);
        let opts = AiBridgeGenerateOptions {
            system_prompt: Some("Current date: 2026-08-06\ncwd: /tmp/lab".into()),
            thinking_level: "medium".into(),
            ..Default::default()
        };
        let tools = [AiBridgeToolSchema {
            name: "read".into(),
            description: "read a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {"path": {"type": "string"}},
            }),
        }];
        let asm = ResponsesAssembler::default();
        let body_a = asm.assemble("lab-m", projected.clone(), &tools, false, &opts);
        let body_b = asm.assemble("lab-m", projected.clone(), &tools, false, &opts);
        assert_eq!(
            body_a["input"], body_b["input"],
            "assemble twice must be idempotent"
        );
        assert_eq!(body_a["tools"], body_b["tools"]);

        let input = body_a["input"].as_array().expect("input");
        // system/developer first, then history; reasoning before assistant text/tool in same turn.
        assert!(
            input
                .iter()
                .any(|i| i.get("role") == Some(&serde_json::json!("developer"))
                    || i.get("role") == Some(&serde_json::json!("system"))),
            "system prompt item present: {input:?}"
        );
        let asst_idx = input
            .iter()
            .position(|i| i.get("role") == Some(&serde_json::json!("assistant")))
            .expect("assistant text item");
        let reason_idx = input
            .iter()
            .position(|i| i.get("type") == Some(&serde_json::json!("reasoning")))
            .expect("reasoning item");
        assert!(
            reason_idx < asst_idx,
            "reasoning must precede assistant text (c1925/c1930): reason={reason_idx} asst={asst_idx}"
        );

        // JSONL import-shaped path (header + messages).
        let mut entries = vec![SessionEntry::Header(SessionHeader {
            entry_type: "session".into(),
            version: SESSION_VERSION,
            id: "c1930-lab".into(),
            timestamp: "2026-08-06T00:00:00Z".into(),
            cwd: "/tmp/lab".into(),
            parent_session: None,
        })];
        for (i, msg) in memory.iter().enumerate() {
            let base = EntryBase {
                entry_type: "message".into(),
                id: format!("e{i}"),
                parent_id: None,
                timestamp: "2026-08-06T00:00:00Z".into(),
            };
            match msg {
                AgentMessage::Env(EnvMessage::BashExecutionMessage {
                    command,
                    output,
                    exclude_from_context,
                    ..
                }) => {
                    let mut e = bash_execution_message_entry(
                        command.clone(),
                        output.clone(),
                        Some(0),
                        false,
                        false,
                        None,
                        *exclude_from_context,
                    );
                    if let SessionEntry::Message(m) = &mut e {
                        m.base = base;
                    }
                    entries.push(e);
                }
                other => {
                    entries.push(SessionEntry::Message(
                        crate::protocol::session::MessageEntry {
                            base,
                            message: serde_json::to_value(other).expect("serialize AgentMessage"),
                        },
                    ));
                }
            }
        }
        let mut jsonl = String::new();
        for entry in &entries {
            jsonl.push_str(&serde_json::to_string(entry).expect("ser entry"));
            jsonl.push('\n');
        }
        let parsed = parse_session_jsonl(&jsonl).expect("parse");
        let imported: Vec<AgentMessage> =
            parsed.iter().filter_map(|e| e.as_agent_message()).collect();
        assert_eq!(
            imported.len(),
            memory.len(),
            "import must recover all context messages"
        );
        let projected_import = project_for_llm(&imported);
        let body_import = asm.assemble("lab-m", projected_import, &tools, false, &opts);
        assert_eq!(
            body_import["input"], body_a["input"],
            "JSONL resume/import path must match memory assemble input prefix"
        );
        assert_eq!(body_import["tools"], body_a["tools"]);

        // Serde round-trip of projected LLM rows (bridge DTO) must not drift input.
        let wire = serde_json::to_value(&projected).expect("serde");
        let back: Vec<xylitol_ai_bridge::dto::AiBridgeMessage> =
            serde_json::from_value(wire).expect("de");
        let body_serde = asm.assemble("lab-m", back, &tools, false, &opts);
        assert_eq!(body_serde["input"], body_a["input"]);
    }
}
