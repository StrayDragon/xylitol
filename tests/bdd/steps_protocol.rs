use crate::bdd::prelude::*;
use crate::bdd::steps_server::ServerTest;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_protocol::codec::{self, Inbound, Rejected};
use xylitol_protocol::{Command, Event, Message, Outcome, RequestId};
use xylitol_serve::MAX_BODY_BYTES;
use xylitol_wire::event_from_wire;

pub struct ProtocolBdd {
    pub cmds: RefCell<Vec<Result<Command, String>>>,
    pub events: RefCell<Vec<Event>>,
    pub text_out: RefCell<String>,
    pub legacy_rejected: Cell<Option<bool>>,
    pub import_kind: RefCell<Option<(String, bool)>>,
    pub depth_guard: Cell<Option<bool>>,
    pub last_decode: RefCell<Option<Result<Message, Rejected>>>,
}

#[fixture]
pub fn protocol_bdd() -> ProtocolBdd {
    ProtocolBdd {
        cmds: RefCell::new(Vec::new()),
        events: RefCell::new(Vec::new()),
        text_out: RefCell::new(String::new()),
        legacy_rejected: Cell::new(None),
        import_kind: RefCell::new(None),
        depth_guard: Cell::new(None),
        last_decode: RefCell::new(None),
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
    let projected: XyEvent = event_from_wire(thinking).expect("wire -> domain projection");
    match projected {
        XyEvent::ThinkingDelta(text) => assert_eq!(text, "reasoning"),
        other => panic!("expected ThinkingDelta projection, got {other:?}"),
    }
}

#[when("对未知 type 的线协议事件做反序列化")]
fn w_unknown_wire_event(protocol_bdd: &ProtocolBdd) {
    let event: Event = serde_json::from_str(r#"{"type":"totally_unknown_variant","x":1}"#)
        .expect("unknown wire type MUST degrade to Unknown, not fail");
    *protocol_bdd.events.borrow_mut() = vec![event];
}

#[then("落入 Unknown 变体且不 panic")]
fn t_unknown_wire_event_degrades(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    assert!(
        matches!(events.as_slice(), [Event::Unknown]),
        "unknown wire type MUST land in Event::Unknown: {events:?}"
    );
}

#[when("对携带工具参数的 tool_execution_start 做线协议往返")]
fn w_tool_start_roundtrip(protocol_bdd: &ProtocolBdd) {
    let orig = Event::ToolExecutionStart {
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
        Event::ToolExecutionStart { args, .. } => {
            assert_eq!(args["path"], "/a b/c.txt");
            assert_eq!(args["limit"], 5);
        }
        other => panic!("expected ToolExecutionStart, got {other:?}"),
    }
}

#[when("序列化 is_error 失败标记的 tool_execution_end 并构造缺省旧载荷")]
fn w_tool_end_flag(protocol_bdd: &ProtocolBdd) {
    let full = Event::ToolExecutionEnd {
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

    let mut legacy_value: serde_json::Value = serde_json::from_str(&full_text).unwrap();
    legacy_value
        .as_object_mut()
        .expect("object")
        .remove("is_error");
    let legacy_text = legacy_value.to_string();
    let legacy_rejected = serde_json::from_str::<Event>(&legacy_text).is_err();

    protocol_bdd.legacy_rejected.set(Some(legacy_rejected));
    *protocol_bdd.events.borrow_mut() = vec![back];
}

#[then("完整载荷保真失败态且缺失标记的载荷被拒绝")]
fn t_tool_end_flag_semantics(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    assert!(
        matches!(
            events.first(),
            Some(Event::ToolExecutionEnd { is_error: true, .. })
        ),
        "failure state must survive the wire: {events:?}"
    );
    assert_eq!(
        protocol_bdd.legacy_rejected.get(),
        Some(true),
        "missing is_error must fail wire parse (required on the wire)"
    );
}

#[when("对 QueueUpdate 事件做线协议序列化与反序列化往返")]
fn w_queue_update_roundtrip(protocol_bdd: &ProtocolBdd) {
    let event = Event::QueueUpdate {
        steer_count: 2,
        follow_up_count: 1,
    };
    let text = serde_json::to_string(&event).expect("encode QueueUpdate");
    let back: Event = serde_json::from_str(&text).expect("decode QueueUpdate");
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    let unknown: Event = serde_json::from_str(r#"{"type":"totally_unknown_variant","x":1}"#)
        .expect("unknown type MUST degrade, not panic");
    *protocol_bdd.events.borrow_mut() = vec![back, unknown];
}

#[then("队列计数保真且未知 type 落入 Unknown 而非 panic")]
fn t_queue_update_fidelity(protocol_bdd: &ProtocolBdd) {
    let events = protocol_bdd.events.borrow();
    assert!(
        matches!(
            events.first(),
            Some(Event::QueueUpdate {
                steer_count: 2,
                follow_up_count: 1,
            })
        ),
        "c2826: 期望 QueueUpdate 计数保真: {events:?}"
    );
    assert!(
        matches!(events.get(1), Some(Event::Unknown)),
        "c2826: 未知 type MUST 落入 Unknown 而非 panic: {events:?}"
    );
}

#[when("序列化含 thinking 与 text 的 assistant content 为会话部件")]
fn w_agent_part_tagged(protocol_bdd: &ProtocolBdd) {
    use xylitol_plugin_sdk::AgentPart;
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
    use xylitol_plugin_sdk::session::message_text;
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
    *protocol_bdd.events.borrow_mut() = vec![back];
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
    use xylitol_protocol::{
        TodoItem as WireTodoItem, TodoList as WireTodoList, TodoStatus as WireTodoStatus,
    };
    let event = Event::TodoUpdated {
        list: WireTodoList::new(vec![
            WireTodoItem {
                id: "t1".into(),
                content: "压降 specs".into(),
                status: WireTodoStatus::InProgress,
            },
            WireTodoItem {
                id: "t2".into(),
                content: "收口报告".into(),
                status: WireTodoStatus::Pending,
            },
        ]),
    };
    let text = serde_json::to_string(&event).expect("encode TodoUpdated");
    let back: Event = serde_json::from_str(&text).expect("decode TodoUpdated");
    assert_eq!(serde_json::to_string(&back).unwrap(), text);
    *protocol_bdd.events.borrow_mut() = vec![back];
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
    let legacy: Event = serde_json::from_str(r#"{"type":"compaction_end"}"#)
        .expect("c2826: 旧无载荷形态必须可解码为缺省载荷");
    *protocol_bdd.events.borrow_mut() = vec![back, legacy];
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
    use xylitol_kernel::export::StdExportIo;
    use xylitol_official_plugin_session::SessionManager;
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
        .import_kind
        .replace(Some((e1.kind().to_string(), e1.kind() == e2.kind())));
}

#[then("错误携带稳定 kind 且非文案猜测")]
fn t_import_stable_kind(protocol_bdd: &ProtocolBdd) {
    let (kind, stable) = protocol_bdd
        .import_kind
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

fn deep_array_envelope(depth: usize) -> String {
    let deep = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
    format!(r#"{{"jsonrpc":"2.0","id":"d1","method":"host.describe","params":{deep}}}"#)
}

fn decode_envelope(protocol_bdd: &ProtocolBdd, depth: usize) {
    let env = deep_array_envelope(depth);
    protocol_bdd
        .depth_guard
        .set(Some(codec::json_nesting_exceeds_limit(&env)));
    protocol_bdd
        .last_decode
        .replace(Some(codec::decode_message(&env)));
}

#[when("对超过深度上限的线上 JSON 信封做解码")]
fn w_json_depth_over_limit(protocol_bdd: &ProtocolBdd) {
    decode_envelope(protocol_bdd, codec::JSON_DEPTH_LIMIT as usize);
}

#[when("对实机深树量级的合法线上信封做解码")]
fn w_json_depth_tree_scale(protocol_bdd: &ProtocolBdd) {
    decode_envelope(protocol_bdd, 200);
}

#[then("深解析有界早退且超限按降级语义拒绝")]
fn t_json_depth_bounded(protocol_bdd: &ProtocolBdd) {
    assert_eq!(
        protocol_bdd.depth_guard.get(),
        Some(true),
        "超限信封 MUST 在深解析前被探测为超限"
    );
    let rejected = protocol_bdd
        .last_decode
        .borrow()
        .clone()
        .expect("decode executed")
        .expect_err("超限深解析 MUST 拒绝");
    assert_eq!(rejected.error.code, -32700, "{rejected:?}");
    assert!(rejected.id.is_none(), "{rejected:?}");
}

#[then("深度上限不误伤常规深树")]
fn t_json_depth_tree_ok(protocol_bdd: &ProtocolBdd) {
    assert_eq!(protocol_bdd.depth_guard.get(), Some(false));
    let decoded = protocol_bdd
        .last_decode
        .borrow()
        .clone()
        .expect("decode executed");
    assert!(
        matches!(&decoded, Ok(Message::Request(req)) if req.method == "host.describe"),
        "实机深树量级 MUST 通过: {decoded:?}"
    );
}

async fn post_rpc_text(server_test: &ServerTest, body: &str) {
    let url = format!("http://127.0.0.1:{}/rpc", server_test.port.get());
    let resp = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .expect("POST /rpc");
    let status = resp.status().as_u16();
    let text = resp.text().await.expect("read /rpc body");
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(text);
}

fn stored_unary(server_test: &ServerTest) -> (u16, String) {
    let body = server_test.unary_body.borrow().clone().unwrap_or_default();
    (server_test.unary_status.get(), body)
}

fn assert_error_response(
    server_test: &ServerTest,
    status: u16,
    code: i64,
    data_code: Option<&str>,
) {
    let (actual, body) = stored_unary(server_test);
    assert_eq!(actual, status, "{body}");
    let Ok(Message::Response(resp)) = codec::decode_message(&body) else {
        panic!("JSON-RPC response expected: {body}");
    };
    let Outcome::Error(err) = resp.outcome else {
        panic!("error response expected: {body}");
    };
    assert_eq!(err.code, code, "{body}");
    if let Some(data_code) = data_code {
        assert_eq!(err.data_code.as_deref(), Some(data_code), "{body}");
    }
}

#[when("POST /rpc 发送 host.describe 的 JSON-RPC 请求且 id 为 1")]
async fn w_jsonrpc_describe_id_one(server_test: &ServerTest) {
    post_rpc_text(
        server_test,
        r#"{"jsonrpc":"2.0","id":1,"method":"host.describe","params":{}}"#,
    )
    .await;
}

#[then("应答为 JSON-RPC 成功 response 且 id 回显为 1")]
fn t_jsonrpc_success_echoes_id_one(server_test: &ServerTest) {
    let (status, body) = stored_unary(server_test);
    assert_eq!(status, 200, "{body}");
    let Ok(Message::Response(resp)) = codec::decode_message(&body) else {
        panic!("JSON-RPC response expected: {body}");
    };
    assert_eq!(resp.id, Some(RequestId::Int(1)), "{body}");
    assert!(matches!(resp.outcome, Outcome::Result(_)), "{body}");
}

#[when("POST /rpc 发送无 id 的 host.describe notification")]
async fn w_jsonrpc_describe_notification(server_test: &ServerTest) {
    post_rpc_text(
        server_test,
        r#"{"jsonrpc":"2.0","method":"host.describe","params":{}}"#,
    )
    .await;
}

#[then("HTTP 200 且 body 为空")]
fn t_http_200_empty_body(server_test: &ServerTest) {
    let (status, body) = stored_unary(server_test);
    assert_eq!(status, 200, "{body}");
    assert!(body.is_empty(), "notification MUST NOT be answered: {body}");
}

#[when("POST /rpc 发送含两个 request 与一个 notification 的 batch")]
async fn w_jsonrpc_mixed_batch(server_test: &ServerTest) {
    post_rpc_text(
        server_test,
        r#"[{"jsonrpc":"2.0","id":"b1","method":"host.describe","params":{}},{"jsonrpc":"2.0","method":"host.describe","params":{}},{"jsonrpc":"2.0","id":"b2","method":"host.describe","params":{}}]"#,
    )
    .await;
}

#[then("返回两个 response 且 id 对应、notification 无应答")]
fn t_batch_responses_in_order(server_test: &ServerTest) {
    let (status, body) = stored_unary(server_test);
    assert_eq!(status, 200, "{body}");
    let Ok(Inbound::Batch(items)) = codec::parse_text(&body) else {
        panic!("batch answer must be a JSON array: {body}");
    };
    let ids: Vec<Option<RequestId>> = items
        .into_iter()
        .map(|item| match item {
            Ok(Message::Response(resp)) => resp.id,
            other => panic!("only responses expected, got {other:?}: {body}"),
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            Some(RequestId::Str("b1".into())),
            Some(RequestId::Str("b2".into())),
        ],
        "{body}"
    );
}

#[when("POST /rpc 发送空数组 batch")]
async fn w_jsonrpc_empty_batch(server_test: &ServerTest) {
    post_rpc_text(server_test, "[]").await;
}

#[then("HTTP 400 且 error.code 为 -32600")]
fn t_http_400_invalid_request(server_test: &ServerTest) {
    assert_error_response(server_test, 400, -32600, None);
}

#[when("POST /rpc 发送非法 JSON")]
async fn w_jsonrpc_illegal_json(server_test: &ServerTest) {
    post_rpc_text(server_test, r#"{"jsonrpc":"2.0","#).await;
}

#[then("HTTP 400 且 error.code 为 -32700")]
fn t_http_400_parse_error(server_test: &ServerTest) {
    assert_error_response(server_test, 400, -32700, None);
}

#[when("POST /rpc 发送缺少 method 的 JSON-RPC 对象")]
async fn w_jsonrpc_missing_method(server_test: &ServerTest) {
    post_rpc_text(server_test, r#"{"jsonrpc":"2.0","id":"m1","params":{}}"#).await;
}

#[when("POST /rpc 调用 set_model 且缺少 model_id")]
async fn w_jsonrpc_set_model_without_model_id(server_test: &ServerTest) {
    post_rpc_text(
        server_test,
        r#"{"jsonrpc":"2.0","id":"sm1","method":"set_model","params":{}}"#,
    )
    .await;
}

#[then("error.code 为 -32602 且 data.code 为 invalid_input")]
fn t_invalid_params_stable_code(server_test: &ServerTest) {
    assert_error_response(server_test, 200, -32602, Some("invalid_input"));
}

#[then("error.code 为 -32000 且 data.code 为 not_found")]
fn t_business_error_stable_code(server_test: &ServerTest) {
    assert_error_response(server_test, 200, -32000, Some("not_found"));
}

#[when("POST /rpc 发送超过 4 MiB 的正文")]
async fn w_jsonrpc_oversized_body(server_test: &ServerTest) {
    let url = format!("http://127.0.0.1:{}/rpc", server_test.port.get());
    let resp = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .body(vec![b' '; MAX_BODY_BYTES + 1])
        .send()
        .await
        .expect("POST /rpc oversized body");
    let status = resp.status().as_u16();
    let text = resp.text().await.expect("read /rpc body");
    server_test.unary_status.set(status);
    *server_test.unary_body.borrow_mut() = Some(text);
}

#[then("HTTP 413 且 error.data.code 为 payload_too_large")]
fn t_http_413_payload_too_large(server_test: &ServerTest) {
    assert_error_response(server_test, 413, -32600, Some("payload_too_large"));
}

#[when("POST /rpc 调用 plugin/call")]
async fn w_jsonrpc_plugin_call(server_test: &ServerTest) {
    post_rpc_text(
        server_test,
        r#"{"jsonrpc":"2.0","id":"pc1","method":"plugin/call","params":{}}"#,
    )
    .await;
}

#[then("error.code 为 -32601 且 data.code 为 unregistered_method")]
fn t_plugin_prefix_unregistered(server_test: &ServerTest) {
    assert_error_response(server_test, 200, -32601, Some("unregistered_method"));
}
