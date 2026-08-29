//! Steps for `app-tui-bridge` — 桥缝模型级族（atb1–atb15 可转子集）。
//!
//! 全部经产品单一桥缝 [`apply_xy_event`]（或其 UiModel API）驱动；
//! 断言落在 UI-only 模型的可观察字段上，不触碰渲染层。

use crate::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol::app::tui::{BashBlockStatus, UiEntry, UiModel, UiPhase, apply_xy_event};
use xylitol::protocol::{AgentMessage, AgentPart, LlmMessage};

/// One UiModel driven through the bridge seam.
pub struct BridgeBdd {
    pub model: RefCell<UiModel>,
}

#[fixture]
pub fn bridge_bdd() -> BridgeBdd {
    BridgeBdd {
        model: RefCell::new(UiModel::default()),
    }
}

fn apply(bdd: &BridgeBdd, event: XyEvent) {
    apply_xy_event(&mut bdd.model.borrow_mut(), &event);
}

fn assistant_message(parts: Vec<AgentPart>) -> AgentMessage {
    AgentMessage::Llm(LlmMessage::AssistantMessage {
        content: parts,
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
}

// atb1 ─────────────────────────────────────────────────────────

#[when("以桥缝注入元数据事件（模型选择与 thinking 档位）")]
fn w_atb1_metadata(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::ModelSelect {
            provider: "fake".into(),
            model_id: "m1".into(),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ThinkingLevelChanged {
            level: "off".into(),
        },
    );
}

#[then("模型不 panic 且条目与相位不变")]
fn t_atb1_unchanged(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.is_empty(),
        "metadata events must not create rows: {:?}",
        model.entries
    );
    assert_eq!(model.phase, UiPhase::Idle);
}

// atb2 ─────────────────────────────────────────────────────────

#[when("以桥缝注入 AgentStart 与中间 TurnEnd")]
fn w_atb2_busy(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::AgentStart {
            session_id: "s".into(),
            model: "fake".into(),
        },
    );
    apply(bridge_bdd, XyEvent::TurnEnd { turn_index: 0 });
}

#[then("相位保持忙碌且 status 不空")]
fn t_atb2_still_busy(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.phase, UiPhase::Busy, "TurnEnd is intermediate");
    assert!(model.status.is_some(), "status must not reset mid-turn");
}

#[when("注入带 follow_up 队列的 AgentEnd")]
fn w_atb2_end_with_followup(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 1,
        },
    );
    apply(
        bridge_bdd,
        XyEvent::AgentEnd {
            messages: Vec::new(),
        },
    );
}

#[then("相位仍忙碌且 status 为 Follow-up pending")]
fn t_atb2_followup_pending(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.phase, UiPhase::Busy);
    assert_eq!(model.status.as_deref(), Some("Follow-up pending"));
}

#[when("注入空队列的 AgentEnd")]
fn w_atb2_end_idle(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::QueueUpdate {
            steer_count: 0,
            follow_up_count: 0,
        },
    );
    apply(
        bridge_bdd,
        XyEvent::AgentEnd {
            messages: Vec::new(),
        },
    );
}

#[then("相位回到 idle")]
fn t_atb2_idle(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.phase, UiPhase::Idle);
    assert!(model.status.is_none());
}

// atb3 ─────────────────────────────────────────────────────────

#[when("以桥缝注入 QueueUpdate steer=2 follow_up=1")]
fn w_atb3_queue(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::QueueUpdate {
            steer_count: 2,
            follow_up_count: 1,
        },
    );
}

#[then("队列计数同步为 2/1")]
fn t_atb3_queue_synced(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.queue.steer_count, 2);
    assert_eq!(model.queue.follow_up_count, 1);
}

#[when("注入 edit 工具成功结果（JSON 含 display_diff）")]
fn w_atb3_edit_end(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionStart {
            id: "e1".into(),
            name: "edit".into(),
            args: serde_json::json!({ "path": "a.rs" }),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionEnd {
            id: "e1".into(),
            name: "edit".into(),
            result: serde_json::json!({
                "path": "a.rs",
                "success": true,
                "display_diff": "@@ -1 +1 @@\n-old\n+new"
            })
            .to_string(),
            is_error: false,
        },
    );
}

#[then("同一条工具行的 display_diff 被填入")]
fn t_atb3_diff_filled(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let tool = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool {
            id, display_diff, ..
        } if id == "e1" => Some(display_diff.clone()),
        _ => None,
    });
    let diff = tool.expect("tool row");
    assert!(
        diff.as_deref().is_some_and(|d| d.contains("+new")),
        "atb3: display_diff extracted on End: {diff:?}"
    );
}

// atb5 ─────────────────────────────────────────────────────────

#[when("以桥缝注入 CompactionStart")]
fn w_atb5_start(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::AgentStart {
            session_id: "s".into(),
            model: "fake".into(),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::CompactionStart {
            reason: "threshold".into(),
        },
    );
}

fn compaction_of(bdd: &BridgeBdd) -> (String, String, u64) {
    let model = bdd.model.borrow();
    model
        .entries
        .iter()
        .rev()
        .find_map(|e| match e {
            UiEntry::Compaction {
                status,
                summary,
                tokens_before,
                ..
            } => Some((format!("{status:?}"), summary.clone(), *tokens_before)),
            _ => None,
        })
        .expect("a compaction block")
}

#[then("busy status 为 Compacting 且出现 pending 占位块")]
fn t_atb5_pending(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.status.as_deref(), Some("Compacting"));
    drop(model);
    let (status, _, _) = compaction_of(bridge_bdd);
    assert_eq!(status, "Pending", "atb5: placeholder block inserted");
}

#[when("注入成功 CompactionEnd（含 summary 与 tokens_before）")]
fn w_atb5_end_ok(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::CompactionEnd {
            result: None,
            aborted: false,
            reason: "threshold".into(),
            will_retry: false,
            error_message: None,
            summary: Some("已压缩".into()),
            tokens_before: Some(1200),
        },
    );
}

#[then("占位就地变为完成块且 status 恢复 Working")]
fn t_atb5_complete(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.status.as_deref(), Some("Working"));
    drop(model);
    let (status, summary, tokens) = compaction_of(bridge_bdd);
    assert_eq!(status, "Complete");
    assert_eq!(summary, "已压缩");
    assert_eq!(tokens, 1200);
    let model = bridge_bdd.model.borrow();
    assert_eq!(
        model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Compaction { .. }))
            .count(),
        1,
        "atb5: in-place finish — no second block"
    );
}

#[when("注入失败 CompactionEnd")]
fn w_atb5_end_failed(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::CompactionEnd {
            result: None,
            aborted: false,
            reason: "threshold".into(),
            will_retry: false,
            error_message: Some("boom".into()),
            summary: None,
            tokens_before: None,
        },
    );
}

#[then("块变为短失败态")]
fn t_atb5_failed(bridge_bdd: &BridgeBdd) {
    let (status, _, _) = compaction_of(bridge_bdd);
    assert_eq!(status, "Failed");
}

// atb6 ─────────────────────────────────────────────────────────

#[when("以桥缝注入 AutoRetryStart attempt=2 max=5")]
fn w_atb6_start(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::AgentStart {
            session_id: "s".into(),
            model: "fake".into(),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::AutoRetryStart {
            attempt: 2,
            max_retries: 5,
            delay_ms: 100,
        },
    );
}

#[then("busy status 为 Retry 2/5")]
fn t_atb6_retry_status(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.status.as_deref(), Some("Retry 2/5"));
}

#[when("注入失败 AutoRetryEnd")]
fn w_atb6_end_failed(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::AutoRetryEnd {
            success: false,
            attempt: 2,
        },
    );
}

#[then("滚动提示说明失败且 status 恢复 Working")]
fn t_atb6_failed_note(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(model.status.as_deref(), Some("Working"));
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::ScrollNotice { text } if text.contains("retry failed")
        )),
        "atb6: failure note appended: {:?}",
        model.entries
    );
}

// atb7 ─────────────────────────────────────────────────────────

#[when("以桥缝注入流式正文与 aborted 错误")]
fn w_atb7_abort(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::AgentStart {
            session_id: "s".into(),
            model: "fake".into(),
        },
    );
    apply(bridge_bdd, XyEvent::TextDelta("draft".into()));
    apply(bridge_bdd, XyEvent::aborted());
}

fn aborted_notice_count(model: &UiModel) -> usize {
    model
        .entries
        .iter()
        .filter(|e| {
            matches!(e, UiEntry::ScrollNotice { text }
            if text == "Operation aborted" || text == "Aborted")
        })
        .count()
}

#[then("已流式正文保留且至多一行 aborted 提示且相位 idle")]
fn t_atb7_partial_and_note(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("draft")
        )),
        "partial flushed: {:?}",
        model.entries
    );
    assert_eq!(aborted_notice_count(&model), 1);
    assert_eq!(model.phase, UiPhase::Idle);
}

#[when("再注入同轮 aborted 错误")]
fn w_atb7_abort_again(bridge_bdd: &BridgeBdd) {
    apply(bridge_bdd, XyEvent::aborted());
}

#[then("不追加重复 aborted 行")]
fn t_atb7_no_dup(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(
        aborted_notice_count(&model),
        1,
        "atb7: deduped against the trailing note"
    );
}

// atb8 / atb9 ──────────────────────────────────────────────────

#[when("以桥缝创建 pending Bash 块并增量追加输出")]
fn w_atb8_append(bridge_bdd: &BridgeBdd) {
    let mut model = bridge_bdd.model.borrow_mut();
    model.begin_bash_block("echo chunky", false);
    model.append_bash_output(b"chu");
    model.append_bash_output(b"nky\n");
}

#[then("条目为 Bash 块而非裸滚动提示且 status 保持 pending")]
fn t_atb8_pending_block(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let bash = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Bash { .. }))
        .count();
    assert_eq!(bash, 1, "one block, not per-chunk notice rows");
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                command, output, status, ..
            } if command == "echo chunky"
                && output.contains("chunky")
                && *status == BashBlockStatus::Pending
        )),
        "pending tint held during streaming: {:?}",
        model.entries
    );
}

#[when("注入完成收口")]
fn w_atb8_finish(bridge_bdd: &BridgeBdd) {
    let mut model = bridge_bdd.model.borrow_mut();
    model.finish_bash_block(BashBlockStatus::Success, "chunky\nok".into());
}

#[then("块状态切换为终态")]
fn t_atb8_finished(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Bash {
                status: BashBlockStatus::Success,
                ..
            }
        )),
        "atb8/9: finish switches the terminal state: {:?}",
        model.entries
    );
}

// atb10 ────────────────────────────────────────────────────────

fn tool_call_part(id: &str, name: &str, args: serde_json::Value) -> AgentPart {
    AgentPart::ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: args,
    }
}

#[when("以桥缝注入含 ToolCall 的助手快照")]
fn w_atb10_intent(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "t1",
                "read",
                serde_json::json!({ "path": "old.rs" }),
            )])),
        },
    );
}

#[then("按工具 id 恰好一行且路径进 args_preview")]
fn t_atb10_one_row(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let tools: Vec<_> = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Tool { id, .. } if id == "t1"))
        .collect();
    assert_eq!(tools.len(), 1, "upsert by tool id: {:?}", model.entries);
    assert!(
        matches!(
            tools[0],
            UiEntry::Tool { args_preview, .. } if args_preview == "old.rs"
        ),
        "path lands in args_preview immediately: {:?}",
        tools[0]
    );
}

#[when("注入同 id 的新快照与 ToolExecutionStart")]
fn w_atb10_upsert(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "t1",
                "read",
                serde_json::json!({ "path": "new.rs" }),
            )])),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionStart {
            id: "t1".into(),
            name: "read".into(),
            args: serde_json::json!({ "path": "new.rs" }),
        },
    );
}

#[then("行被 upsert 而非再 push")]
fn t_atb10_still_one_row(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let tools: Vec<_> = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Tool { id, .. } if id == "t1"))
        .collect();
    assert_eq!(tools.len(), 1, "atb10: same row upserted");
}

#[then("正文快照不写入 streaming 缓冲")]
fn t_atb10_no_snapshot_dup(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: "snapshot body".into(),
            thinking: None,
            message: None,
        },
    );
    apply(
        bridge_bdd,
        XyEvent::AgentEnd {
            messages: Vec::new(),
        },
    );
    let model = bridge_bdd.model.borrow();
    assert!(
        !model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Assistant { text } if text.contains("snapshot body")
        )),
        "text snapshot must not stream into a body row: {:?}",
        model.entries
    );
}

// atb11 / atb12 ────────────────────────────────────────────────

#[when("以桥缝注入 write 意图（含 content）与成功 JSON 结果")]
fn w_atb11_write(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionStart {
            id: "w1".into(),
            name: "write".into(),
            args: serde_json::json!({ "path": "docs/x.md", "content": "hello body" }),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionEnd {
            id: "w1".into(),
            name: "write".into(),
            result: serde_json::json!({ "path": "docs/x.md", "success": true }).to_string(),
            is_error: false,
        },
    );
}

#[then("正文进同一工具块且成功 JSON 不外显")]
fn t_atb11_write_quiet(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let tool = model.entries.iter().find_map(|e| match e {
        UiEntry::Tool { id, .. } if id == "w1" => Some(e.clone()),
        _ => None,
    });
    let tool = tool.expect("write tool row");
    assert!(
        matches!(&tool, UiEntry::Tool { write_content: Some(c), .. } if c.contains("hello body")),
        "args.content lands in the renderable body: {tool:?}"
    );
    assert!(
        matches!(&tool, UiEntry::Tool { output, .. } if !output.contains("\"success\"")),
        "machine JSON must not paint a wall: {tool:?}"
    );
}

#[when("注入 edit 成功 JSON 结果（含 display_diff）")]
fn w_atb11_edit_diff(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionStart {
            id: "e9".into(),
            name: "edit".into(),
            args: serde_json::json!({ "path": "b.rs" }),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionEnd {
            id: "e9".into(),
            name: "edit".into(),
            result: serde_json::json!({
                "path": "b.rs",
                "success": true,
                "display_diff": "+added line"
            })
            .to_string(),
            is_error: false,
        },
    );
}

#[then("diff 合入同一条工具块且不另起 Diff 行")]
fn t_atb11_diff_merged(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    let diffs = model
        .entries
        .iter()
        .filter(|e| matches!(e, UiEntry::Diff { .. }))
        .count();
    assert_eq!(diffs, 0, "no standalone Diff row");
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id, display_diff: Some(d), ..
            } if id == "e9" && d.contains("+added line")
        )),
        "diff merged into the same tool row: {:?}",
        model.entries
    );
}

#[when("注入 edit 失败结果")]
fn w_atb11_edit_error(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionStart {
            id: "e8".into(),
            name: "edit".into(),
            args: serde_json::json!({ "path": "c.rs" }),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionEnd {
            id: "e8".into(),
            name: "edit".into(),
            result: "path not found".into(),
            is_error: true,
        },
    );
}

#[then("错误文案保留在块末")]
fn t_atb11_error_kept(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                is_error: true,
                output,
                ..
            } if id == "e8" && output.contains("path not found")
        )),
        "error text kept for block-end view: {:?}",
        model.entries
    );
}

// atb12 ────────────────────────────────────────────────────────

#[when("以桥缝在 End 前注入 write 快照（arguments.content）")]
fn w_atb12_intent_body(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "w2",
                "write",
                serde_json::json!({ "path": "d.md", "content": "early body" }),
            )])),
        },
    );
}

#[then("工具行正文在 End 前即出现")]
fn t_atb12_body_before_end(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                write_content: Some(c),
                done: false,
                ..
            } if id == "w2" && c.contains("early body")
        )),
        "atb12: body visible at intent time: {:?}",
        model.entries
    );
}

// atb13 ────────────────────────────────────────────────────────

#[when("以桥缝注入 file_path 别名的 write 意图")]
fn w_atb13_alias(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "w3",
                "write",
                serde_json::json!({ "file_path": "docs/alias.md" }),
            )])),
        },
    );
}

#[then("路径即进 args_preview")]
fn t_atb13_alias_path(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                args_preview,
                ..
            } if id == "w3" && args_preview == "docs/alias.md"
        )),
        "file_path alias resolves immediately: {:?}",
        model.entries
    );
}

#[when("注入缺 path 的后续快照与成功 End")]
fn w_atb13_sticky(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "w3",
                "write",
                serde_json::json!({ "content": "no path here" }),
            )])),
        },
    );
    apply(
        bridge_bdd,
        XyEvent::ToolExecutionEnd {
            id: "w3".into(),
            name: "write".into(),
            result: serde_json::json!({ "success": true }).to_string(),
            is_error: false,
        },
    );
}

#[then("已见路径保持")]
fn t_atb13_path_sticky(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                args_preview,
                done: true,
                ..
            } if id == "w3" && args_preview == "docs/alias.md"
        )),
        "atb13: sticky path survives path-less upsert and End: {:?}",
        model.entries
    );
}

#[when("注入 read 意图（含 offset 与 limit）")]
fn w_atb13_read_range(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::MessageUpdate {
            text: String::new(),
            thinking: None,
            message: Some(assistant_message(vec![tool_call_part(
                "r9",
                "read",
                serde_json::json!({ "path": "src/lib.rs", "offset": 10, "limit": 5 }),
            )])),
        },
    );
}

#[then("路径附行号区间")]
fn t_atb13_line_range(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                args_preview,
                ..
            } if id == "r9" && args_preview == "src/lib.rs:10-14"
        )),
        "atb13: :start-end suffix: {:?}",
        model.entries
    );
}

// atb14 ────────────────────────────────────────────────────────

#[when("以桥缝注入 Provider 错误")]
fn w_atb14_provider_error(bridge_bdd: &BridgeBdd) {
    apply(
        bridge_bdd,
        XyEvent::Error(xylitol::protocol::lifecycle::XyEventError::new(
            "Provider",
            "gateway 502",
        )),
    );
}

#[then("追加粘性 Error 行")]
fn t_atb14_sticky_error(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Error { text } if text == "gateway 502"
        )),
        "non-abort kinds stay sticky: {:?}",
        model.entries
    );
}

#[when("注入 Aborted 错误")]
fn w_atb14_aborted(bridge_bdd: &BridgeBdd) {
    apply(bridge_bdd, XyEvent::aborted());
}

#[then("走 aborted 提示路径而非粘性 Error")]
fn t_atb14_abort_path(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert_eq!(aborted_notice_count(&model), 1);
    // Provider 错误的既有粘性行保留；aborted 只新增提示，不再加 Error 行。
    assert_eq!(
        model
            .entries
            .iter()
            .filter(|e| matches!(e, UiEntry::Error { .. }))
            .count(),
        1,
        "abort must not add a second sticky Error row"
    );
}

// atb15 ────────────────────────────────────────────────────────

#[when("经 wire 序列化往返 ToolExecutionStart 后注入桥缝")]
fn w_atb15_roundtrip(bridge_bdd: &BridgeBdd) {
    let event = XyEvent::ToolExecutionStart {
        id: "t9".into(),
        name: "read".into(),
        args: serde_json::json!({ "path": "real/path.rs" }),
    };
    let wire = serde_json::to_string(&event).expect("serialize");
    let event: XyEvent = serde_json::from_str(&wire).expect("deserialize");
    apply(bridge_bdd, event);
}

#[then("args_preview 保留真实路径不退化为占位")]
fn t_atb15_real_path(bridge_bdd: &BridgeBdd) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model.entries.iter().any(|e| matches!(
            e,
            UiEntry::Tool {
                id,
                args_preview,
                ..
            } if id == "t9" && args_preview == "real/path.rs"
        )),
        "atb15: wire parity keeps the real path: {:?}",
        model.entries
    );
}
