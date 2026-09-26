//! protocol-app 纯协议层步骤：serde 往返、闭集拒绝、JSON-RPC 信封。
//! 不起 Host —— 全部直接驱动 `crate::protocol` 公开类型。

use crate::protocol::lifecycle::XyEvent;
use crate::protocol::wire::codec;
use crate::protocol::{Command, Event, RpcMessage};
use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};
use std::convert::TryFrom;

/// Shared state for protocol-layer scenarios.
pub struct ProtocolBdd {
    pub cmds: RefCell<Vec<Result<Command, String>>>,
    pub events: RefCell<Vec<Event>>,
    pub tool_end_pair: RefCell<Option<(String, bool)>>,
    pub msgs: RefCell<Vec<RpcMessage>>,
}

#[fixture]
pub fn protocol_bdd() -> ProtocolBdd {
    ProtocolBdd {
        cmds: RefCell::new(Vec::new()),
        events: RefCell::new(Vec::new()),
        tool_end_pair: RefCell::new(None),
        msgs: RefCell::new(Vec::new()),
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
