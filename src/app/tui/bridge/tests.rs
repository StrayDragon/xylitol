use super::*;
use crate::protocol::message::AgentMessage;

fn assistant_end() -> XyEvent {
    XyEvent::AgentEnd {
        messages: Vec::<AgentMessage>::new(),
    }
}

#[test]
fn text_deltas_commit_on_message_end() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::MessageStart {
            role: "assistant".into(),
            message: None,
        },
    );
    apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
    apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
    apply_xy_event(
        &mut model,
        &XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
    );
    assert!(
        model
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Assistant { text } if text == "Hello!"))
    );
    assert_eq!(model.streaming_assistant, "");
}

#[test]
fn turn_end_does_not_idle() {
    let mut model = UiModel::new();
    model.begin_run("x");
    apply_xy_event(&mut model, &XyEvent::TurnStart { turn_index: 0 });
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionStart {
            id: "1".into(),
            name: "read".into(),
            args: Value::Null,
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionEnd {
            id: "1".into(),
            name: "read".into(),
            result: "ok".into(),
            is_error: false,
        },
    );
    apply_xy_event(&mut model, &XyEvent::TurnEnd { turn_index: 0 });
    assert_eq!(model.phase, UiPhase::Busy);
    assert!(model.status.is_some());
}

#[test]
fn agent_end_idles_when_follow_up_empty() {
    let mut model = UiModel::new();
    model.begin_run("x");
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 0,
        },
    );
    apply_xy_event(&mut model, &assistant_end());
    assert_eq!(model.phase, UiPhase::Idle);
    assert_eq!(model.status, None);
}

#[test]
fn agent_end_stays_busy_with_follow_up() {
    let mut model = UiModel::new();
    model.begin_run("x");
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 1,
        },
    );
    apply_xy_event(&mut model, &assistant_end());
    assert_eq!(model.phase, UiPhase::Busy);
    assert_eq!(model.queue.follow_up_count, 1);
}

#[test]
fn queue_update_sets_badge() {
    let mut model = UiModel::new();
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 2,
            follow_up_count: 3,
        },
    );
    assert_eq!(
        model.queue,
        QueueBadge {
            steer_count: 2,
            follow_up_count: 3
        }
    );
}

#[test]
fn queue_update_trims_pending_strip_fifo() {
    let mut model = UiModel::new();
    model.enqueue_steer_strip("a".into());
    model.enqueue_steer_strip("b".into());
    model.enqueue_follow_up_strip("c".into());
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 1,
            follow_up_count: 0,
        },
    );
    assert_eq!(model.pending_steer, vec!["b".to_string()]);
    assert!(model.pending_follow_up.is_empty());
}

#[test]
fn user_message_start_commits_steer_to_scrollback() {
    use crate::protocol::message::AgentMessage;

    let mut model = UiModel::new();
    model.begin_run("hello");
    model.enqueue_steer_strip("nudge".into());
    apply_xy_event(
        &mut model,
        &XyEvent::MessageStart {
            role: "user".into(),
            message: Some(AgentMessage::user("nudge")),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::MessageEnd {
            role: "user".into(),
            message: Some(AgentMessage::user("nudge")),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 0,
        },
    );
    assert!(model.pending_steer.is_empty());
    let users: Vec<_> = model
        .entries
        .iter()
        .filter_map(|e| match e {
            UiEntry::User { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(users, ["hello", "nudge"]);
}

#[test]
fn aborted_error_is_system_note_and_idles() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(&mut model, &XyEvent::aborted());
    assert_eq!(model.phase, UiPhase::Idle);
    assert!(model.status.is_none());
    assert!(model.entries.iter().any(|e| matches!(
        e,
        UiEntry::ScrollNotice { text } if text == "Operation aborted" || text == "Aborted"
    )));
    assert!(
        !model
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::Error { .. }))
    );
}

#[test]
fn note_bash_cancelled_does_not_emit_aborted() {
    let mut model = UiModel::new();
    model.begin_bash_block("sleep 1", false);
    model.note_bash_cancelled();
    assert!(model.entries.iter().any(|e| matches!(
        e,
        UiEntry::Bash {
            status: BashBlockStatus::Cancelled,
            output,
            ..
        } if output.contains("(cancelled)")
    )));
    assert!(!model.entries.iter().any(|e| matches!(
        e,
        UiEntry::ScrollNotice { text }
            if text == "Aborted" || text == "Operation aborted"
    )));
}

#[test]
fn note_user_abort_allows_second_abort_after_new_command() {
    let mut model = UiModel::new();
    model.note_user_abort();
    model.entries.push(UiEntry::ScrollNotice {
        text: "$ sleep 2".into(),
    });
    model.note_user_abort();
    let n = model
        .entries
        .iter()
        .filter(|e| {
            matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Operation aborted" || text == "Aborted"
            )
        })
        .count();
    assert_eq!(
        n, 2,
        "each agent abort must show abort note: {:?}",
        model.entries
    );
}

#[test]
fn note_user_abort_dedupes_with_error_aborted() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    model.note_user_abort();
    apply_xy_event(&mut model, &XyEvent::aborted());
    let n = model
        .entries
        .iter()
        .filter(|e| {
            matches!(
                e,
                UiEntry::ScrollNotice { text }
                    if text == "Operation aborted"
                        || text == "Aborted"
                        || text == "aborted"
            )
        })
        .count();
    assert_eq!(n, 1, "must not duplicate abort notes");
    assert_eq!(model.phase, UiPhase::Idle);
}

#[test]
fn note_user_abort_keeps_flushed_partial() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(&mut model, &XyEvent::TextDelta("hello partial".into()));
    model.note_user_abort();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("hello partial")
        )),
        "abort must keep streamed assistant: {:?}",
        model.entries
    );
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::ScrollNotice { text } if text == "Operation aborted"
        )),
        "expected abort footer: {:?}",
        model.entries
    );
    assert!(model.streaming_assistant.is_empty());
}

#[test]
fn metadata_events_do_not_panic() {
    let mut model = UiModel::new();
    apply_xy_event(
        &mut model,
        &XyEvent::ModelSelect {
            provider: "openai".into(),
            model_id: "gpt".into(),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::ThinkingLevelChanged {
            level: "high".into(),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::SessionInfoChanged {
            key: "k".into(),
            value: Value::Null,
        },
    );
    assert!(model.entries.is_empty());
    assert_eq!(model.phase, UiPhase::Idle);
}

#[test]
fn message_update_does_not_duplicate_deltas() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(&mut model, &XyEvent::TextDelta("Hello".into()));
    apply_xy_event(
        &mut model,
        &XyEvent::MessageUpdate {
            text: "Hello".into(),
            thinking: None,
            message: None,
        },
    );
    apply_xy_event(&mut model, &XyEvent::TextDelta("!".into()));
    apply_xy_event(
        &mut model,
        &XyEvent::MessageEnd {
            role: "assistant".into(),
            message: None,
        },
    );
    let assistants: Vec<_> = model
        .entries
        .iter()
        .filter_map(|e| match e {
            UiEntry::Assistant { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(assistants, ["Hello!"]);
}

#[test]
fn message_update_tool_intent_flushes_thinking_first() {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(&mut model, &XyEvent::ThinkingDelta("plan…".into()));
    assert!(
        model
            .entries
            .iter()
            .all(|e| !matches!(e, UiEntry::Thinking { .. })),
        "thinking still in streaming buffer"
    );

    let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: vec![
            AgentPart::thinking("plan…"),
            AgentPart::ToolCall {
                id: "call-1".into(),
                name: "find".into(),
                arguments: serde_json::json!({"pattern": "**/*.md"}),
            },
        ],
        stop_reason: None,
        usage: None,
        api: String::new(),
        provider: String::new(),
        model: String::new(),
        response_id: None,
        error_message: None,
        timestamp: 0,
        diagnostics: Vec::new(),
    });
    apply_xy_event(
        &mut model,
        &XyEvent::MessageUpdate {
            text: String::new(),
            thinking: Some("plan…".into()),
            message: Some(partial),
        },
    );

    let kinds: Vec<&str> = model
        .entries
        .iter()
        .filter_map(|e| match e {
            UiEntry::Thinking { .. } => Some("thinking"),
            UiEntry::Tool { name, .. } => Some(name.as_str()),
            UiEntry::Assistant { .. } => Some("assistant"),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        ["thinking", "find"],
        "scrollback must be thinking then tool, got {kinds:?}"
    );
    assert!(model.streaming_thinking.is_empty());
}

#[test]
fn message_update_creates_tool_before_execution() {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let mut model = UiModel::new();
    model.begin_run("hi");
    let partial = AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: vec![AgentPart::ToolCall {
            id: "call-1".into(),
            name: "bash".into(),
            arguments: serde_json::json!({"command": "echo hi"}),
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
    });
    apply_xy_event(
        &mut model,
        &XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(partial),
        },
    );
    let tools: Vec<_> = model
        .entries
        .iter()
        .filter_map(|e| match e {
            UiEntry::Tool {
                id,
                args_preview,
                done,
                ..
            } => Some((id.as_str(), args_preview.as_str(), *done)),
            _ => None,
        })
        .collect();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].0, "call-1");
    assert!(tools[0].1.starts_with("$ echo"), "got {}", tools[0].1);
    assert!(!tools[0].2);

    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionStart {
            id: "call-1".into(),
            name: "bash".into(),
            args: serde_json::json!({"command": "echo hi"}),
        },
    );
    let tool_count = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Tool { .. }))
        .count();
    assert_eq!(
        tool_count, 1,
        "ToolExecutionStart must upsert, not duplicate"
    );
}

#[test]
fn message_update_streams_args_preview() {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let mut model = UiModel::new();
    model.begin_run("hi");
    let mk = |args: serde_json::Value| {
        AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::ToolCall {
                id: "call-1".into(),
                name: "read".into(),
                arguments: args,
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
        })
    };
    apply_xy_event(
        &mut model,
        &XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(mk(serde_json::json!({"path": "/tm"}))),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(mk(serde_json::json!({"path": "/tmp/x.rs"}))),
        },
    );
    let preview = model
        .entries
        .iter()
        .find_map(|e| match e {
            UiEntry::Tool { args_preview, .. } => Some(args_preview.as_str()),
            _ => None,
        })
        .expect("tool row");
    assert_eq!(preview, "/tmp/x.rs");
    assert_eq!(
        model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Tool { .. }))
            .count(),
        1
    );
}

#[test]
fn compaction_start_sets_status_and_placeholder() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::CompactionStart {
            reason: "auto: 90%".into(),
        },
    );
    assert_eq!(model.phase, UiPhase::Busy);
    assert_eq!(model.status.as_deref(), Some("Compacting"));
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Pending,
                ..
            }
        )),
        "expected pending compaction block, got {:?}",
        model.entries
    );
}

#[test]
fn compaction_end_restores_working() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::CompactionStart {
            reason: "manual".into(),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::CompactionEnd {
            result: Some("ok".into()),
            aborted: false,
            reason: "manual".into(),
            will_retry: false,
            error_message: None,
            summary: Some("session summary".into()),
            tokens_before: Some(42_000),
        },
    );
    assert_eq!(model.status.as_deref(), Some("Working"));
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Complete,
                tokens_before: 42_000,
                ..
            }
        )),
        "expected complete compaction block, got {:?}",
        model.entries
    );
}

#[test]
fn compaction_end_aborted_restores_working() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::CompactionStart {
            reason: "manual".into(),
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::CompactionEnd {
            result: None,
            aborted: true,
            reason: "manual".into(),
            will_retry: false,
            error_message: None,
            summary: None,
            tokens_before: None,
        },
    );
    assert_eq!(model.status.as_deref(), Some("Working"));
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Aborted,
                ..
            }
        )),
        "expected aborted compaction block, got {:?}",
        model.entries
    );
}

#[test]
fn auto_retry_start_sets_status() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::AutoRetryStart {
            attempt: 2,
            max_retries: 5,
            delay_ms: 100,
        },
    );
    assert_eq!(model.status.as_deref(), Some("Retry 2/5"));
}

#[test]
fn auto_retry_end_fail_notes_and_restores() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::AutoRetryStart {
            attempt: 1,
            max_retries: 3,
            delay_ms: 0,
        },
    );
    apply_xy_event(
        &mut model,
        &XyEvent::AutoRetryEnd {
            success: false,
            attempt: 1,
        },
    );
    assert_eq!(model.status.as_deref(), Some("Working"));
    assert!(
        model
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("retry failed")))
    );
}

#[test]
fn auto_retry_end_success_restores_without_fail_note() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::AutoRetryStart {
            attempt: 1,
            max_retries: 3,
            delay_ms: 0,
        },
    );
    let before = model.entries.len();
    apply_xy_event(
        &mut model,
        &XyEvent::AutoRetryEnd {
            success: true,
            attempt: 1,
        },
    );
    assert_eq!(model.status.as_deref(), Some("Working"));
    assert_eq!(model.entries.len(), before);
    assert!(
        !model
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { text } if text.contains("retry failed")))
    );
}

#[test]
fn mcp_tool_shows_pretty_args_and_result() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionStart {
            id: "m1".into(),
            name: "mcp:lspz:get_diagnostics".into(),
            args: serde_json::json!({"uri": "file:///tmp/a.rs"}),
        },
    );
    let pending = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool {
            id,
            args_preview,
            output,
            ..
        } if id == "m1" => Some((args_preview.clone(), output.clone())),
        _ => None,
    });
    let (preview, body) = pending.expect("mcp tool row");
    assert!(
        preview.is_empty(),
        "mcp header must omit args chrome (body owns args:): {preview:?}"
    );
    assert!(
        body.starts_with("args:\n") && body.contains("\"uri\": \"file:///tmp/a.rs\""),
        "pending body must show pretty args: {body}"
    );

    // Simulate provider streaming the raw result before End (regression: duplicated body).
    let raw = r#"{"content":[{"type":"text","text":"ok"}],"isError":false}"#;
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionUpdate {
            id: "m1".into(),
            output: raw.into(),
        },
    );
    let mid = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool { id, output, .. } if id == "m1" => Some(output.clone()),
        _ => None,
    });
    let mid = mid.expect("mcp mid");
    assert!(
        !mid.contains("\"content\""),
        "streamed raw must not glue onto args: {mid}"
    );

    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionEnd {
            id: "m1".into(),
            name: "mcp:lspz:get_diagnostics".into(),
            result: raw.into(),
            is_error: false,
        },
    );
    let done = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool {
            id, output, done, ..
        } if id == "m1" => Some((output.clone(), *done)),
        _ => None,
    });
    let (out, done) = done.expect("mcp done");
    assert!(done);
    assert!(out.contains("args:\n"), "{out}");
    assert!(out.contains("result:\n"), "{out}");
    assert!(out.contains("\"isError\": false"), "pretty result: {out}");
    assert!(out.contains('\n'), "result must be pretty multiline: {out}");
    let raw_hits = out.matches(r#"{"content""#).count();
    assert_eq!(
        raw_hits, 0,
        "minified result must not appear beside args: {out}"
    );
    assert_eq!(
        out.matches("result:").count(),
        1,
        "result section once: {out}"
    );
}

#[test]
fn mcp_end_rebuilds_even_if_buffer_was_polluted() {
    let mut model = UiModel::new();
    model.begin_run("hi");
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionStart {
            id: "m2".into(),
            name: "mcp:lspz:get_diagnostics".into(),
            args: serde_json::json!({}),
        },
    );
    // Force-pollute as if an older build appended Update.
    for e in &mut model.entries {
        if let UiEntry::Tool { id, output, .. } = e
            && id == "m2"
        {
            output.push_str(r#"{"content":[{"type":"text","text":"x"}],"isError":false}"#);
        }
    }
    apply_xy_event(
        &mut model,
        &XyEvent::ToolExecutionEnd {
            id: "m2".into(),
            name: "mcp:lspz:get_diagnostics".into(),
            result: r#"{"content":[{"type":"text","text":"ok"}],"isError":false}"#.into(),
            is_error: false,
        },
    );
    let out = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool { id, output, .. } if id == "m2" => Some(output.as_str()),
        _ => None,
    });
    let out = out.expect("m2");
    assert!(out.starts_with("args:\n{}"), "{out}");
    assert!(out.contains("result:\n"), "{out}");
    assert!(!out.contains("args:\n{}{"), "no glued minified: {out}");
    assert_eq!(out.matches("\"text\": \"ok\"").count(), 1, "{out}");
}
