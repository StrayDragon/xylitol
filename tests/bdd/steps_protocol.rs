//! protocol-app 纯协议层步骤：serde 往返、闭集拒绝、JSON-RPC 信封。
//! 不起 Host —— 全部直接驱动 `xylitol::protocol` 公开类型。

use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use std::convert::TryFrom;
use xylitol::protocol::lifecycle::XyEvent;
use xylitol::protocol::wire::codec;
use xylitol::protocol::{Command, Event, RpcMessage};

/// Shared state for protocol-layer scenarios.
pub struct ProtocolBdd {
    pub cmds: RefCell<Vec<Result<Command, String>>>,
    pub events: RefCell<Vec<Event>>,
    pub tool_end_pair: RefCell<Option<(String, bool)>>,
    pub msgs: RefCell<Vec<RpcMessage>>,
    /// c2826：纯文本提取结果。
    pub text_out: RefCell<String>,
}

#[fixture]
pub fn protocol_bdd() -> ProtocolBdd {
    ProtocolBdd {
        cmds: RefCell::new(Vec::new()),
        events: RefCell::new(Vec::new()),
        tool_end_pair: RefCell::new(None),
        msgs: RefCell::new(Vec::new()),
        text_out: RefCell::new(String::new()),
    }
}

fn parse_cmd(text: &str) -> Result<Command, String> {
    serde_json::from_str::<Command>(text).map_err(|e| e.to_string())
}

#[when("解析队列命令 steer、follow_up、clear_queue 的线协议 JSON")]
fn w_parse_queue_commands(protocol_bdd: &ProtocolBdd) {
    let samples = [
        r#"{"type":"steer","id":null,"message":"do this instead"}"#,
        r#"{"type":"follow_up","id":null,"message":"after that"}"#,
        r#"{"type":"clear_queue","id":null}"#,
    ];
    *protocol_bdd.cmds.borrow_mut() = samples.iter().map(|s| parse_cmd(s)).collect();
}

#[then("分别得到 Steer、FollowUp 与 ClearQueue 变体")]
fn t_queue_command_variants(protocol_bdd: &ProtocolBdd) {
    let cmds = protocol_bdd.cmds.borrow();
    assert!(cmds.iter().all(|r| r.is_ok()), "{cmds:?}");
    assert!(matches!(cmds[0], Ok(Command::Steer { .. })), "{cmds:?}");
    assert!(matches!(cmds[1], Ok(Command::FollowUp { .. })), "{cmds:?}");
    assert!(
        matches!(cmds[2], Ok(Command::ClearQueue { .. })),
        "{cmds:?}"
    );
    // 缺省布尔按 default_true 解析
    if let Ok(Command::ClearQueue {
        clear_steer,
        clear_follow_up,
        ..
    }) = &cmds[2]
    {
        assert!(*clear_steer && *clear_follow_up, "{cmds:?}");
    }
}

#[when("解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）")]
fn w_parse_local_surface_commands(protocol_bdd: &ProtocolBdd) {
    let samples = [
        r#"{"type":"clipboard_write","text":"x"}"#,
        r#"{"type":"osc52_put","text":"x"}"#,
        r#"{"type":"set_keybinding","chord":"ctrl-g"}"#,
    ];
    *protocol_bdd.cmds.borrow_mut() = samples.iter().map(|s| parse_cmd(s)).collect();
}

#[then("命令闭集拒绝且不产生任何变体")]
fn t_local_surface_rejected(protocol_bdd: &ProtocolBdd) {
    let cmds = protocol_bdd.cmds.borrow();
    assert!(
        cmds.iter().all(|r| r.is_err()),
        "local-surface verbs must not parse into Command, got {cmds:?}"
    );
}

#[when("对 Agent 流族事件做线协议序列化与反序列化往返")]
fn w_stream_events_roundtrip(protocol_bdd: &ProtocolBdd) {
    let originals = vec![
        Event::TurnStart { turn_index: 1 },
        Event::TurnEnd { turn_index: 1 },
        Event::MessageStart {
            role: "user".into(),
            message: None,
        },
        Event::MessageEnd {
            role: "assistant".into(),
            message: Some(serde_json::json!({"text": "done"})),
        },
        Event::MessageUpdate {
            text: "partial".into(),
            thinking: Some("hmm".into()),
            message: None,
        },
        Event::ToolExecutionUpdate {
            id: "t1".into(),
            output: "chunk".into(),
        },
        Event::TextDelta {
            text: "hello".into(),
        },
        Event::ThinkingDelta {
            text: "reasoning".into(),
        },
        Event::CompactionEnd {
            result: Some("ok".into()),
            aborted: false,
            reason: "threshold".into(),
            will_retry: false,
            error_message: None,
            summary: Some("prior turns".into()),
            tokens_before: Some(63_737),
            tokens_after: None,
            notice: None,
        },
    ];
    let mut parsed = Vec::new();
    for ev in &originals {
        let text = serde_json::to_string(ev).expect("serialize");
        let back: Event = serde_json::from_str(&text).expect("deserialize");
        // 以再序列化字符串比较，避免依赖 PartialEq 派生
        assert_eq!(
            serde_json::to_string(&back).unwrap(),
            text,
            "roundtrip must be faithful"
        );
        parsed.push(back);
    }
    *protocol_bdd.events.borrow_mut() = parsed;
}

#[then("流族事件保真且 thinking_delta 可投影为 XyEvent")]
fn t_stream_events_project(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    let thinking = events
        .iter()
        .find(|e| matches!(e, Event::ThinkingDelta { .. }))
        .expect("thinking_delta present");
    let projected = XyEvent::try_from(thinking).expect("wire -> domain projection");
    match projected {
        XyEvent::ThinkingDelta(text) => assert_eq!(text, "reasoning"),
        other => panic!("expected ThinkingDelta projection, got {other:?}"),
    }
}

#[when("对携带工具参数的 tool_start 做线协议往返")]
fn w_tool_start_roundtrip(protocol_bdd: &ProtocolBdd) {
    let orig = Event::ToolStart {
        id: "t1".into(),
        name: "read".into(),
        args: serde_json::json!({"path": "/a b/c.txt", "limit": 5}),
    };
    let text = serde_json::to_string(&orig).expect("serialize");
    let back: Event = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(
        serde_json::to_string(&back).unwrap(),
        text,
        "tool args must survive the wire verbatim"
    );
    *protocol_bdd.events.borrow_mut() = vec![back];
}

#[then("工具参数在往返后保持原字段")]
fn t_tool_start_args_kept(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    match &events[0] {
        Event::ToolStart { args, .. } => {
            assert_eq!(args["path"], "/a b/c.txt");
            assert_eq!(args["limit"], 5);
        }
        other => panic!("expected ToolStart, got {other:?}"),
    }
}

#[when("序列化 is_error 失败标记的 tool_end 并构造缺省旧载荷")]
fn w_tool_end_flag(protocol_bdd: &ProtocolBdd) {
    let full = Event::ToolEnd {
        id: "t1".into(),
        name: "bash".into(),
        result: "boom".into(),
        is_error: true,
    };
    let full_text = serde_json::to_string(&full).expect("serialize");
    let back: Event = serde_json::from_str(&full_text).expect("deserialize");
    assert_eq!(
        serde_json::to_string(&back).unwrap(),
        full_text,
        "full form must keep is_error=true"
    );

    // 线上必填（c2440）：移除 is_error 后的载荷必须解析失败
    let mut legacy_value: serde_json::Value = serde_json::from_str(&full_text).unwrap();
    legacy_value
        .as_object_mut()
        .expect("object")
        .remove("is_error");
    let legacy_text = legacy_value.to_string();
    let legacy_rejected = serde_json::from_str::<Event>(&legacy_text).is_err();

    *protocol_bdd.tool_end_pair.borrow_mut() = Some((full_text, legacy_rejected));
}

#[then("完整载荷保真失败态且缺失标记的载荷被拒绝")]
fn t_tool_end_flag_semantics(protocol_bdd: &ProtocolBdd) {
    let (_, legacy_rejected) = protocol_bdd
        .tool_end_pair
        .borrow()
        .clone()
        .expect("tool_end pair");
    assert!(
        legacy_rejected,
        "missing is_error must fail wire parse (required on the wire)"
    );
}

#[when("解析 JSON-RPC 信封样例（request / result / notification）")]
fn w_jsonrpc_envelopes(protocol_bdd: &ProtocolBdd) {
    let samples = [
        r#"{"jsonrpc":"2.0","id":"r1","method":"prompt","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":"r1","result":{}}"#,
        r#"{"jsonrpc":"2.0","method":"session/event","params":{}}"#,
    ];
    let mut msgs = Vec::new();
    for s in samples {
        let m = codec::decode_str(s).unwrap_or_else(|e| panic!("parse {s}: {e}"));
        msgs.push(m);
    }
    *protocol_bdd.msgs.borrow_mut() = msgs;
}

#[then("JSON-RPC 形态与 id 回显成立")]
fn t_jsonrpc_shape(protocol_bdd: &ProtocolBdd) {
    let msgs = protocol_bdd.msgs.borrow();
    assert!(
        matches!(&msgs[0], RpcMessage::ClientRequest { rpc_id, method, .. }
            if rpc_id == "r1" && method == "prompt"),
        "{msgs:?}"
    );
    assert!(
        matches!(&msgs[1], RpcMessage::ServerResponse { rpc_id, .. } if rpc_id == "r1"),
        "result must echo request id, got {msgs:?}"
    );
    assert!(
        matches!(&msgs[2], RpcMessage::ServerRequest { method, rpc_id, .. }
            if method == "session/event" && rpc_id.is_empty()),
        "event downlink is a notification (no id), got {msgs:?}"
    );
}

// ---- c2826 specs-compact：裸规则转场景补充步骤 ----

#[when("对 QueueUpdate 事件做线协议序列化与反序列化往返")]
fn w_queue_update_roundtrip(protocol_bdd: &ProtocolBdd) {
    let event = Event::QueueUpdate {
        steer_count: 2,
        follow_up_count: 1,
    };
    let text = serde_json::to_string(&event).expect("encode QueueUpdate");
    let back: Event = serde_json::from_str(&text).expect("decode QueueUpdate");
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    protocol_bdd.events.replace(vec![back]);
    // 未映射（未知 type 标签）帧：解析为 Err，消费端可降级忽略，MUST NOT panic。
    let unknown = serde_json::from_str::<Event>(r#"{"type":"totally_unknown_variant","x":1}"#);
    // 以 text_out 暂存「未知 type 解析为 Err」的结论（无 panic 即通过）。
    let verdict = if unknown.is_err() { "err" } else { "ok" };
    protocol_bdd.text_out.replace(verdict.to_string());
}

#[then("队列计数保真且未知 type 解析为错误而非 panic")]
fn t_queue_update_fidelity(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    let Some(Event::QueueUpdate {
        steer_count,
        follow_up_count,
    }) = events.first()
    else {
        panic!("c2826: 期望 QueueUpdate，实际 {:?}", events.first());
    };
    assert_eq!((*steer_count, *follow_up_count), (2, 1));
    let verdict = protocol_bdd.text_out.borrow().clone();
    assert_eq!(
        verdict, "err",
        "c2826: 未知 type MUST 解析失败（可降级忽略）而非 panic"
    );
}

#[when("序列化含 thinking 与 text 的 assistant content 为会话部件")]
fn w_agent_part_tagged(protocol_bdd: &ProtocolBdd) {
    use xylitol::protocol::AgentPart;
    let parts = vec![
        AgentPart::Thinking {
            thinking: "隐式推理".into(),
            redacted: false,
            thinking_signature: Some(r#"{"encrypted":"k"}"#.into()),
        },
        AgentPart::Text {
            text: "正文回答".into(),
        },
    ];
    let raw = serde_json::to_string(&parts).expect("serialize parts");
    protocol_bdd.text_out.replace(raw);
}

#[then("每个部件带 type 判别且无裸字符串 content")]
fn t_agent_part_tagged(protocol_bdd: &ProtocolBdd) {
    let raw = protocol_bdd.text_out.borrow().clone();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let items = value.as_array().expect("content 必须是数组（非裸字符串）");
    let types: Vec<&str> = items
        .iter()
        .map(|item| item.get("type").and_then(|t| t.as_str()).unwrap_or(""))
        .collect();
    assert_eq!(
        types,
        vec!["thinking", "text"],
        "c2826: 部件 type 判别：{raw}"
    );
    let thinking = &items[0];
    assert!(
        thinking.get("thinking").is_some(),
        "c2826: thinking 字段：{raw}"
    );
    assert!(
        thinking.get("thinkingSignature").is_some(),
        "c2826: thinkingSignature 字段：{raw}"
    );
}

#[when("对含 thinking 与 text 的消息提取纯文本摘要")]
fn w_preview_text(protocol_bdd: &ProtocolBdd) {
    use xylitol::protocol::session::message_text;
    let msg = serde_json::json!({
        "role": "assistant",
        "content": [
            {"type": "thinking", "thinking": "内心独白"},
            {"type": "text", "text": "对外正文"}
        ]
    });
    protocol_bdd.text_out.replace(message_text(&msg));
}

#[then("摘要仅含 text 正文")]
fn t_preview_text(protocol_bdd: &ProtocolBdd) {
    let text = protocol_bdd.text_out.borrow().clone();
    assert_eq!(text, "对外正文");
    assert!(
        !text.contains("内心独白"),
        "c2826: 摘要 MUST NOT 拼入 thinking 正文"
    );
}

#[when("序列化携带 kind 的 error 事件")]
fn w_error_kind_roundtrip(protocol_bdd: &ProtocolBdd) {
    let event = Event::Error {
        kind: "Provider".into(),
        message: "upstream 429".into(),
    };
    let text = serde_json::to_string(&event).expect("encode error");
    let back: Event = serde_json::from_str(&text).expect("decode error");
    protocol_bdd.events.replace(vec![back]);
}

#[then("kind 往返保真")]
fn t_error_kind_roundtrip(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    let Some(Event::Error { kind, message }) = events.first() else {
        panic!("c2826: 期望 Error 事件");
    };
    assert_eq!(kind, "Provider");
    assert_eq!(message, "upstream 429");
}

#[when("对 TodoUpdated 快照事件做线协议往返")]
fn w_todo_updated_roundtrip(protocol_bdd: &ProtocolBdd) {
    use xylitol::protocol::session::{TodoItem, TodoList, TodoStatus};
    let event = Event::TodoUpdated {
        list: TodoList::new(vec![
            TodoItem {
                id: "t1".into(),
                content: "压降 specs".into(),
                status: TodoStatus::InProgress,
            },
            TodoItem {
                id: "t2".into(),
                content: "收口报告".into(),
                status: TodoStatus::Pending,
            },
        ]),
    };
    let text = serde_json::to_string(&event).expect("encode TodoUpdated");
    let back: Event = serde_json::from_str(&text).expect("decode TodoUpdated");
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    protocol_bdd.events.replace(vec![back]);
}

#[then("清单快照载荷保真")]
fn t_todo_updated_fidelity(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    let Some(Event::TodoUpdated { list }) = events.first() else {
        panic!("c2826: 期望 TodoUpdated");
    };
    assert_eq!(list.items.len(), 2);
    assert_eq!(list.items[0].id, "t1");
    assert_eq!(list.items[0].content, "压降 specs");
}

#[when("序列化全载荷 CompactionEnd 并构造旧无载荷形态")]
fn w_compaction_end_roundtrip(protocol_bdd: &ProtocolBdd) {
    let event = Event::CompactionEnd {
        result: Some("compacted".into()),
        aborted: false,
        reason: "threshold".into(),
        will_retry: false,
        error_message: None,
        summary: Some("摘要正文".into()),
        tokens_before: Some(120_000),
        tokens_after: Some(30_000),
        notice: Some("Compacted from 120k tokens".into()),
    };
    let text = serde_json::to_string(&event).expect("encode CompactionEnd");
    let back: Event = serde_json::from_str(&text).expect("decode CompactionEnd");
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    protocol_bdd.events.replace(vec![back]);
    let legacy: Event = serde_json::from_str(r#"{"type":"compaction_end"}"#)
        .expect("c2826: 旧无载荷形态必须可解码为缺省载荷");
    protocol_bdd.events.borrow_mut().push(legacy);
}

#[then("全载荷保真且旧形态解码为缺省载荷")]
fn t_compaction_end_fidelity(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    let Some(Event::CompactionEnd {
        result,
        aborted,
        reason,
        will_retry,
        error_message,
        summary,
        tokens_before,
        tokens_after,
        notice,
    }) = events.first()
    else {
        panic!("c2826: 期望 CompactionEnd");
    };
    assert_eq!(result.as_deref(), Some("compacted"));
    assert!(!aborted);
    assert_eq!(reason, "threshold");
    assert!(!will_retry);
    assert!(error_message.is_none());
    assert_eq!(summary.as_deref(), Some("摘要正文"));
    assert_eq!(*tokens_before, Some(120_000));
    assert_eq!(*tokens_after, Some(30_000));
    assert_eq!(notice.as_deref(), Some("Compacted from 120k tokens"));
    let Some(Event::CompactionEnd {
        result,
        summary,
        tokens_before,
        ..
    }) = events.get(1)
    else {
        panic!("c2826: 旧形态必须是 CompactionEnd");
    };
    assert!(result.is_none() && summary.is_none() && tokens_before.is_none());
}

#[when("以不存在的文件调用导入会话")]
async fn w_import_missing_file(protocol_bdd: &ProtocolBdd) {
    use xylitol::SessionExporter;
    use xylitol::infra::export::StdExportIo;
    use xylitol::infra::session::SessionManager;
    let dir = tempfile::tempdir().unwrap();
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let mgr = SessionManager::new(sessions);
    let exporter = SessionExporter::new(Some(std::sync::Arc::new(StdExportIo::new())));
    let missing = dir.path().join("definitely-missing.jsonl");
    let e1 = exporter
        .import_from_jsonl(&mgr, &missing)
        .await
        .expect_err("missing file import must fail");
    let e2 = exporter
        .import_from_jsonl(&mgr, &missing)
        .await
        .expect_err("missing file import must fail again");
    protocol_bdd
        .tool_end_pair
        .replace(Some((e1.kind().to_string(), e1.kind() == e2.kind())));
}

#[then("错误携带稳定 kind 且非文案猜测")]
fn t_import_stable_kind(protocol_bdd: &ProtocolBdd) {
    let (kind, stable) = protocol_bdd
        .tool_end_pair
        .borrow()
        .clone()
        .expect("import error captured");
    assert!(!kind.is_empty(), "c2826: 错误必须携带结构化 kind");
    assert!(stable, "c2826: 同源失败 kind 必须稳定（两次调用一致）");
    assert_ne!(
        kind, "Message",
        "c2826: 域失败 MUST NOT 归为无结构 Message kind"
    );
}
