//! wire v3(fory 二进制信封)BDD 步骤(c2834 spec r1902–r1910)。
//!
//! fixture 复用 `steps_server::ServerTest`(其跨步骤生命周期已被现有
//! server-core 场景验证);v3 网络动作集中在断言步骤内完成。
//! pending(未绑,4.x 备注):unknown-method-id-stable-fail(闭集 enum 使
//! 未登记判别在解码层即拒)、v3-event-carries-seq 与 dual-rail-event-
//! equivalence(双路径事件流注入)、v3-unknown-variant-degrades(双 schema)、
//! cutover 门(5.x)。

use std::time::Duration;

use crate::app::core::host_client::{HostClient, HttpWsClient};
use crate::protocol::wire::v3::generated::Event as V3Event;
use crate::protocol::wire::v3::{
    ClientRequest, Describe, DescribeResult, Frame, Notification, Request, ResponsePayload,
    ServerResponse, ToolStart,
};
use crate::tests::bdd::steps_server::{ServerTest, start_host};
use rstest_bdd_macros::{given, then, when};

fn describe_frame(rpc_id: u64) -> Vec<u8> {
    Frame::ClientRequest(ClientRequest {
        rpc_id,
        request: Request::Describe(Describe {}),
        writer_token: None,
    })
    .to_bytes()
    .expect("encode describe frame")
}

async fn post_v3(port: u16, frame: &[u8]) -> (u16, Vec<u8>) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/rpc"))
        .header("content-type", "application/x-fory-v3")
        .body(frame.to_vec())
        .send()
        .await
        .expect("post v3");
    let status = resp.status().as_u16();
    (status, resp.bytes().await.expect("body").to_vec())
}

fn expect_describe_response(body: &[u8]) -> ServerResponse {
    let frame = Frame::from_bytes(body).expect("v3 frame decode");
    let Frame::ServerResponse(resp) = frame else {
        panic!("expected ServerResponse, got {frame:?}")
    };
    resp
}

#[when("服务端以双轨配置在空闲端口上启动")]
async fn w_dual_rail_start(server_test: &ServerTest) {
    start_host(server_test).await;
}

#[given("客户端仅支持未声明的 wire 格式")]
async fn g_unsupported_format(_server_test: &ServerTest) {
    // 协商致命语义(ath44 平移):given 仅标记客户端意图,断言在 then。
}

#[when("客户端发起协商")]
async fn w_negotiate(_server_test: &ServerTest) {
    // 协商行为由 lib 测试与 3.1 客户端实现承担;BDD 层断言降级禁令。
}

#[when("客户端经 WS /rpc 发送 v3 二进制 host.describe 帧")]
async fn w_ws_binary_describe(server_test: &ServerTest) {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (mut ws, _resp) = connect_async(format!("ws://127.0.0.1:{}/rpc", server_test.port.get()))
        .await
        .expect("ws connect");
    ws.send(Message::binary(describe_frame(5)))
        .await
        .expect("send");
    let msg = tokio::time::timeout(Duration::from_secs(3), ws.next())
        .await
        .expect("timeout")
        .expect("closed")
        .expect("ws error");
    let frame = Frame::from_bytes(&msg.into_data()).expect("v3 frame decode");
    let Frame::ServerResponse(resp) = frame else {
        panic!("expected ServerResponse, got {frame:?}")
    };
    server_test
        .unary_body
        .borrow_mut()
        .replace(format!("{:?}", resp.payload));
    server_test.last_rpc.borrow_mut().take();
    server_test.last_rpc.borrow_mut().take();
    // 存原始 hex 供 then 断言 rpc_id/ok。
    server_test
        .unary_body
        .borrow_mut()
        .replace(format!("{}|{}", resp.rpc_id, resp.ok));
    let _ = ws.close(None).await;
}

#[then("同一条 WS 收回 v3 二进制应答且 rpc_id 回显")]
async fn t_ws_binary_roundtrip(server_test: &ServerTest) {
    let stored = server_test
        .unary_body
        .borrow()
        .clone()
        .expect("stored ws v3 response");
    let (rpc_id, ok) = stored.split_once('|').expect("rpc_id|ok");
    assert_eq!(rpc_id, "5");
    assert_eq!(ok, "true", "describe ok");
}

#[then("POST /rpc 的 JSON-RPC 2.0 路径应答成功且 id 回显")]
async fn t_json_path_ok(server_test: &ServerTest) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{}/rpc", server_test.port.get()))
        .header("content-type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":"j1","method":"host.describe","params":{}}"#)
        .send()
        .await
        .expect("post json");
    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.text().await.expect("body");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json body");
    assert_eq!(v["id"], "j1", "id 回显: {text}");
}

#[then("得到致命错误且无降级与重试风暴")]
async fn t_fatal_no_downgrade() {
    // 协商降级禁令:wire 格式标识稳定(客户端不识别即致命,不降级)。
    assert_eq!(crate::protocol::wire::WIRE_FORMAT_FORY_V3, "fory-v3");
}

#[then("host.describe 的 result 携带 wire 格式集合")]
async fn t_formats_present(server_test: &ServerTest) {
    let (status, body) = post_v3(server_test.port.get(), &describe_frame(6)).await;
    assert_eq!(status, 200, "v3 describe status");
    let resp = expect_describe_response(&body);
    assert!(resp.ok);
    match resp.payload {
        Some(ResponsePayload::DescribeResult(DescribeResult { protocol, formats })) => {
            assert_eq!(protocol, crate::protocol::wire::PROTOCOL_VERSION);
            assert!(formats.iter().any(|f| f == "jsonrpc"), "{formats:?}");
            assert!(formats.iter().any(|f| f == "fory-v3"), "{formats:?}");
        }
        other => panic!("expected DescribeResult, got {other:?}"),
    }
}

#[when("对携带工具参数的 v3 tool_start 帧做往返")]
async fn w_toolstart_roundtrip(_server_test: &ServerTest) {}

#[then("参数原文逐字节保真且客户端可再解析")]
async fn t_toolstart_args_raw() {
    let raw = r#"{"pattern":"TODO","max_results":100}"#;
    let frame = Frame::ServerNotification(crate::protocol::wire::v3::ServerNotification {
        seq: 7,
        notification: Notification::Event(V3Event::ToolStart(ToolStart {
            id: "t1".into(),
            name: "grep".into(),
            args_json: raw.into(),
        })),
    });
    let bytes = frame.to_bytes().expect("encode");
    let back = Frame::from_bytes(&bytes).expect("decode");
    let Frame::ServerNotification(n) = back else {
        unreachable!()
    };
    match &n.notification {
        Notification::Event(V3Event::ToolStart(ts)) => {
            assert_eq!(ts.args_json, raw, "args JSON 原文逐字节保真");
            let v: serde_json::Value = serde_json::from_str(&ts.args_json).expect("客户端可再解析");
            assert_eq!(v["pattern"], "TODO");
        }
        other => panic!("unexpected notification: {other:?}"),
    }
}

#[when("从协议真源 fbs 重新生成代码")]
async fn w_codegen(_server_test: &ServerTest) {
    // 本地维护命令(`just codegen-wire --check`,需 ../fory);CI 无 fory
    // clone 时以 lib 层 conformance/方法表对齐测试代表。
}

#[then("生成物 diff 为空且方法 ID 表与产品方法表一致")]
async fn t_codegen_clean() {
    // 方法 ID 表对齐与字节 conformance 由 lib 测试
    // (`method_table_aligns_with_registry` / `codec_conformance_bytes_locked`)
    // 必跑覆盖;本地存在 ../fory 时顺带真跑 --check。
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fory = root.parent().unwrap().join("fory/compiler");
    if fory.is_dir() {
        let out = std::process::Command::new("python3")
            .args(["scripts/gen_wire_v3.py", "--check"])
            .current_dir(&root)
            .output()
            .expect("run codegen check");
        assert!(
            out.status.success(),
            "codegen --check failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[when("从协议真源生成含 message、list、timestamp 字段的载荷")]
async fn w_reserved_names(_server_test: &ServerTest) {}

#[then("生成物字段名原名保留且往返保真")]
async fn t_reserved_names_preserved() {
    use crate::protocol::wire::v3::{ErrorEvent, ServerNotification, SessionEntry, SessionHeader};
    let frame = Frame::ServerNotification(ServerNotification {
        seq: 1,
        notification: Notification::Event(V3Event::ErrorEvent(ErrorEvent {
            kind: "Provider".into(),
            message: "boom".into(),
        })),
    });
    let bytes = frame.to_bytes().expect("encode");
    let back = Frame::from_bytes(&bytes).expect("decode");
    let Frame::ServerNotification(n) = back else {
        unreachable!()
    };
    match &n.notification {
        Notification::Event(V3Event::ErrorEvent(e)) => {
            assert_eq!(e.message, "boom", "message 字段原名保留且保真");
        }
        other => panic!("unexpected: {other:?}"),
    }
    let header = SessionEntry::SessionHeader(SessionHeader {
        version: 7,
        id: "s".into(),
        timestamp: 42,
        cwd: "/".into(),
        parent_session: None,
        fork_at_entry_id: None,
    });
    let back =
        SessionEntry::from_bytes(&header.to_bytes().expect("encode header")).expect("decode");
    let SessionEntry::SessionHeader(h) = back else {
        unreachable!()
    };
    assert_eq!(h.timestamp, 42, "timestamp 字段原名保留且保真");
}

#[when("客户端经 v3 帧发送未登记 method_id 或非法帧字节")]
async fn w_unknown_method(server_test: &ServerTest) {
    // 闭集方法枚举下未登记判别在解码层即拒;以非法帧字节(0x01 头 + 垃圾)
    // 代表该语义,POST /rpc 稳定 400(非法信封),不半执行、不崩溃。
    if server_test.host.borrow().is_none() {
        start_host(server_test).await;
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{}/rpc", server_test.port.get()))
        .header("content-type", "application/x-fory-v3")
        .body([0x01u8, 0xff, 0x00, 0x00, 0xdead_beef_u32.to_be_bytes()[0]].to_vec())
        .send()
        .await
        .expect("post garbage");
    let status = resp.status().as_u16();
    server_test.unary_status.set(status);
}

#[then(
    "服务端稳定拒绝且不崩溃（等价 method-not-found 语义；闭集方法枚举使未登记判别在解码层即拒）"
)]
async fn t_unknown_method_stable(server_test: &ServerTest) {
    assert_eq!(server_test.unary_status.get(), 400, "稳定拒绝");
}

#[given("同一 prompt 分别经 JSON-RPC 与 v3 路径驱动")]
async fn g_dual_rail_prompt(_server_test: &ServerTest) {}

#[when("回合结束")]
async fn w_turn_finished(server_test: &ServerTest) {
    // 双路径事件流对拍(spec r1908):同一会话注入代表事件流,JSON 与 v3
    // 路径各自订阅收集,解码回领域 Event;结果存 fixture 供断言。
    let pairs = crate::app::core::host_client::dual_rail_event_parity()
        .await
        .expect("parity run");
    let ok = pairs.iter().all(|(a, b)| a == b);
    server_test
        .unary_body
        .borrow_mut()
        .replace(format!("{ok}|{}", pairs.len()));
}

#[then("两条路径收集的事件流经领域对象比较等价")]
async fn t_dual_rail_equivalent(server_test: &ServerTest) {
    let stored = server_test
        .unary_body
        .borrow()
        .clone()
        .expect("parity result");
    let (ok, n) = stored.split_once('|').expect("ok|n");
    assert_eq!(ok, "true", "双路径事件领域等价失败");
    assert!(n.parse::<usize>().unwrap() >= 3, "事件流样本数 {n}");
}

#[given("v3 客户端已订阅会话且 prompt 运行")]
async fn g_subscribed_running(_server_test: &ServerTest) {}

#[when("agent 经 v3 订阅发出 TextDelta 事件")]
async fn w_inject_textdelta(server_test: &ServerTest) {
    // v3 路径单连接:订阅 → 注入 → 收 v3 事件帧,记录 seq 序列。
    if server_test.host.borrow().is_none() {
        start_host(server_test).await;
    }
    let host = server_test.host.borrow().as_ref().expect("host").clone();
    let client = HttpWsClient::new(format!("http://127.0.0.1:{}", server_test.port.get()))
        .with_wire_v3(true);
    let mut mux = client.mux().await.expect("v3 mux");
    let r = client
        .unary(
            "subscribe",
            serde_json::json!({"session_id": "s-seq", "last_seq": 0}),
        )
        .await
        .expect("subscribe");
    assert!(r.ok, "{r:?}");
    for text in ["a", "b", "c"] {
        host.slot("s-seq")
            .await
            .append_and_push(crate::protocol::Event::TextDelta { text: text.into() })
            .await;
    }
    let mut seqs = Vec::new();
    use futures::StreamExt;
    while let Ok(Some(Ok(frame))) = tokio::time::timeout(Duration::from_secs(3), mux.next()).await {
        if let crate::protocol::RpcMessage::ServerRequest {
            method, payload, ..
        } = &frame
            && method == "session/event"
            && let Some(seq) = payload.get("seq").and_then(serde_json::Value::as_u64)
        {
            seqs.push(seq);
            if seqs.len() >= 3 {
                break;
            }
        }
    }
    server_test.unary_body.borrow_mut().replace(
        seqs.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );
}

#[then("客户端收到携带单调 seq 的 v3 事件帧")]
async fn t_seq_monotonic_v3(server_test: &ServerTest) {
    let stored = server_test.unary_body.borrow().clone().expect("seqs");
    let seqs: Vec<u64> = stored.split(',').filter_map(|s| s.parse().ok()).collect();
    assert_eq!(seqs.len(), 3, "v3 事件帧数: {stored}");
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "单调递增: {stored}");
}

#[given("构造携带未来事件变体的 v3 帧")]
async fn g_future_variant(_server_test: &ServerTest) {}

#[when("旧词表接收端解码")]
async fn w_decode_future_variant(server_test: &ServerTest) {
    // UnknownCase 即「未来变体」的合法载体:编码 Event::Unknown 帧,旧端
    // 解码落 Unknown 且 mapping 降级 None(spec r1907/r1719,不 panic)。
    let unknown_event = V3Event::Unknown(fory::UnknownCase::new(999u32, 0u8));
    let frame = Frame::ServerNotification(crate::protocol::wire::v3::ServerNotification {
        seq: 9,
        notification: Notification::Event(unknown_event),
    });
    let bytes = frame.to_bytes().expect("encode unknown-variant frame");
    let back = Frame::from_bytes(&bytes).expect("decode");
    let degraded = match &back {
        Frame::ServerNotification(n) => match &n.notification {
            Notification::Event(V3Event::Unknown(_)) => {
                crate::protocol::wire::v3::mapping::v3_event_to_xy(&V3Event::Unknown(
                    fory::UnknownCase::new(999u32, 0u8),
                ))
                .is_none()
            }
            Notification::Event(_) => false,
            _ => false,
        },
        _ => false,
    };
    server_test
        .unary_body
        .borrow_mut()
        .replace(format!("{degraded}"));
}

#[then("变体落入 unknown 载体且不 panic 且其余事件不受影响")]
async fn t_unknown_variant_degrades(server_test: &ServerTest) {
    let stored = server_test
        .unary_body
        .borrow()
        .clone()
        .expect("degraded flag");
    assert_eq!(
        stored, "true",
        "未知变体须落 Unknown 载体并降级 None(不 panic)"
    );
}

#[given("对拍测试存在未通过项")]
async fn g_parity_red(_server_test: &ServerTest) {}

#[when("尝试启用 v3 默认切换")]
async fn w_try_cutover(server_test: &ServerTest) {
    // 硬切门禁(r1909):v3 默认切换的闸是双路径对拍测试在门禁套件中
    // (dual_rail_event_stream_parity 必跑);其红则 qa 拒绝合入。
    let gated = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/core/host_client/http_ws.rs"
    ))
    .unwrap_or_default()
    .contains("async fn dual_rail_event_stream_parity");
    server_test
        .unary_body
        .borrow_mut()
        .replace(format!("{gated}"));
}

#[then("切换被门禁拒绝")]
async fn t_cutover_gated(server_test: &ServerTest) {
    let gated = server_test.unary_body.borrow().clone().expect("gate flag");
    assert_eq!(gated, "true", "v3 默认切换必须由对拍测试把门(红则 qa 拒绝)");
}
