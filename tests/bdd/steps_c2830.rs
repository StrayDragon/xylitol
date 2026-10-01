//! Steps for c2830 — native HTTP mock 上游（r1462 / r1473 / r1474）。
//!
//! 真 `AnthropicMessagesAdapter`（base_url → 127.0.0.1 脚本化 SSE mock）在
//! 观测闸 + `SpanCollectScope` 下驱动，`llm.request` 断言落在收集到的
//! `SpanRecord`（properties + events）。r1556（SSE idle 上界）由 bridge
//! 包内 paused-time 单测承载：90s 程序权威常量在真实时间等不起，见该
//! 规则的 `# verified-by: fn` 锚。

use std::net::SocketAddr;
use std::time::Duration;

use crate::infra::provider::adapter::AdapterXyModel;
use crate::protocol::model::XyChunk;
use crate::protocol::ports::XyModel;
use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_otel_obs::OtelBdd;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_ai_bridge::provider::AnthropicMessagesAdapter;
use xylitol_ai_bridge::provider::trace::ObservationIoTier;

pub(crate) const BDD_MODEL: &str = "claude-bdd-mock";

/// 脚本化 mock 上游的一帧。
enum Frame {
    Sse {
        event: &'static str,
        data: String,
    },
    /// 头帧后永久静默（连接保持，不再写任何字节）。
    Stall,
}

fn message_start() -> Frame {
    Frame::Sse {
        event: "message_start",
        data: r#"{"type":"message_start","message":{"id":"msg_bdd","role":"assistant","content":[],"model":"claude-bdd-mock","stop_reason":null,"usage":{"input_tokens":12,"output_tokens":1}}}"#.into(),
    }
}

fn block_start() -> Frame {
    Frame::Sse {
        event: "content_block_start",
        data:
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#
                .into(),
    }
}

fn text_delta(text: &str) -> Frame {
    Frame::Sse {
        event: "content_block_delta",
        data: format!(
            r#"{{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":{}}}}}"#,
            serde_json::to_string(text).expect("quote text"),
        ),
    }
}

fn happy_script() -> Vec<Frame> {
    vec![
        message_start(),
        block_start(),
        text_delta("bdd 你好"),
        Frame::Sse {
            event: "content_block_stop",
            data: r#"{"type":"content_block_stop","index":0}"#.into(),
        },
        Frame::Sse {
            event: "message_delta",
            data: r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":7}}"#.into(),
        },
        Frame::Sse {
            event: "message_stop",
            data: r#"{"type":"message_stop"}"#.into(),
        },
    ]
}

fn stall_after_text_script() -> Vec<Frame> {
    vec![
        message_start(),
        block_start(),
        text_delta("半程"),
        Frame::Stall,
    ]
}

async fn spawn_mock(script: Vec<Frame>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock upstream");
    let addr = listener.local_addr().expect("mock addr");
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let Ok((mut sock, _)) = listener.accept().await else {
            return;
        };
        // 排掉请求头即可开写（POST body 未读净对 TCP 无害，雏形同法）。
        let mut buf = vec![0u8; 64 * 1024];
        let _ = tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await;
        let head =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
        if sock.write_all(head).await.is_err() {
            return;
        }
        for frame in script {
            match frame {
                Frame::Sse { event, data } => {
                    let payload = format!("event: {event}\ndata: {data}\n\n");
                    if sock.write_all(payload.as_bytes()).await.is_err() {
                        return;
                    }
                    let _ = sock.flush().await;
                }
                Frame::Stall => {
                    let () = futures::future::pending().await;
                    return;
                }
            }
        }
        // socket drop = EOF → 适配器 SSE 循环自然收尾（Done 已先至）。
    });
    addr
}

/// 场景内共享的 mock 上游地址与驱动状态。
pub struct MockUpstreamBdd {
    pub(crate) addr: RefCell<Option<SocketAddr>>,
    pub(crate) saw_text: std::cell::Cell<bool>,
}

#[fixture]
pub fn mock_upstream_bdd() -> MockUpstreamBdd {
    MockUpstreamBdd {
        addr: RefCell::new(None),
        saw_text: std::cell::Cell::new(false),
    }
}

fn build_model(addr: SocketAddr) -> AdapterXyModel {
    AdapterXyModel::new(Arc::new(AnthropicMessagesAdapter::new(
        "bdd-key".into(),
        BDD_MODEL.into(),
        Some(format!("http://{addr}")),
        None,
    )))
}

// ── given ─────────────────────────────────────────────────────

#[given("mock 上游按正常完成脚本回放 Anthropic SSE")]
pub(crate) async fn g_c2830_mock_happy(mock_upstream_bdd: &MockUpstreamBdd) {
    let addr = spawn_mock(happy_script()).await;
    *mock_upstream_bdd.addr.borrow_mut() = Some(addr);
}

#[given("mock 上游仅回放半程 SSE 后挂起")]
pub(crate) async fn g_c2830_mock_stall(mock_upstream_bdd: &MockUpstreamBdd) {
    let addr = spawn_mock(stall_after_text_script()).await;
    *mock_upstream_bdd.addr.borrow_mut() = Some(addr);
}

// ── when ──────────────────────────────────────────────────────

#[when("以 io=truncated 的观测闸对 mock 上游驱动一次流式生成并耗尽全部事件")]
pub(crate) async fn w_c2830_drive(otel_bdd: &OtelBdd, mock_upstream_bdd: &MockUpstreamBdd) {
    otel_bdd.mount_io(ObservationIoTier::Truncated);
    let addr = mock_upstream_bdd.addr.borrow().expect("mock started");
    let model = build_model(addr);
    let mut stream = XyModel::generate_stream(&model, vec![], &[], true, Default::default())
        .await
        .expect("mock stream established");
    while let Some(item) = stream.next().await {
        if let Err(e) = item {
            panic!("[c2830] mock stream error: {e}");
        }
    }
}

#[when("流式生成收到首个文本增量后放弃该流")]
pub(crate) async fn w_c2830_abort(otel_bdd: &OtelBdd, mock_upstream_bdd: &MockUpstreamBdd) {
    otel_bdd.mount_io(ObservationIoTier::Truncated);
    let addr = mock_upstream_bdd.addr.borrow().expect("mock started");
    let model = build_model(addr);
    let mut stream = XyModel::generate_stream(&model, vec![], &[], true, Default::default())
        .await
        .expect("mock stream established");
    while let Some(item) = stream.next().await {
        match item.expect("abort 前不得出错") {
            XyChunk::TextDelta(_) => {
                mock_upstream_bdd.saw_text.set(true);
                break;
            }
            _ => continue,
        }
    }
    assert!(
        mock_upstream_bdd.saw_text.get(),
        "c2830: 放弃前必须已收到文本增量"
    );
    drop(stream);
}

// ── then ──────────────────────────────────────────────────────

fn prop<'a>(record: &'a fastrace::collector::SpanRecord, key: &str) -> Option<&'a str> {
    record
        .properties
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_ref())
}

fn c2830_llm_span(records: &[fastrace::collector::SpanRecord]) -> &fastrace::collector::SpanRecord {
    records
        .iter()
        .rev()
        .find(|s| {
            s.name == "llm.request"
                && s.properties
                    .iter()
                    .any(|(k, v)| k == "langfuse.observation.model.name" && v.as_ref() == BDD_MODEL)
        })
        .expect("c2830: mock 模型的 llm.request span")
}

fn event_has(span: &fastrace::collector::SpanRecord, name: &str, key: &str, value: &str) -> bool {
    span.events.iter().any(|e| {
        e.name == name
            && e.properties
                .iter()
                .any(|(k, v)| k == key && v.as_ref() == value)
    })
}

/// r1462：provider tracing 激活时原始协议事件与映射后 XyChunk 变体
/// 在同一 fastrace 请求 span 上成对。
#[then("同一 llm.request span 上同时记录 raw 与 mapped 事件")]
pub(crate) fn t_c2830_raw_mapped(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let llm = c2830_llm_span(&records);
    assert!(
        event_has(llm, "raw", "kind", "raw"),
        "r1462: 应有 raw 事件，实际 {:?}",
        llm.events
    );
    assert!(
        event_has(llm, "mapped", "variant", "TextDelta"),
        "r1462: 应有 mapped TextDelta，实际 {:?}",
        llm.events
    );
    assert!(
        event_has(llm, "mapped", "variant", "Done"),
        "r1462: 应有 mapped Done，实际 {:?}",
        llm.events
    );
}

/// r1473：observation_input 优先来自适配器 HTTP 前捕获的请求体。
/// Anthropic 流的 raw 事件名（message_start 等）不在 legacy 请求体事件名
/// （response.json / chat.completion.json / message.json）之列，因此该
/// input 只可能来自 capture_request_input——断言即诚实。
#[then("llm.request 携带来自适配器 HTTP 前捕获的观测输入")]
pub(crate) fn t_c2830_input(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let llm = c2830_llm_span(&records);
    let input = prop(llm, "langfuse.observation.input")
        .expect("r1473: io=truncated 流式路径必须携带 observation.input");
    assert!(
        input.contains(BDD_MODEL) && input.contains("\"stream\":true"),
        "r1473: input 应为适配器组装的请求体：{input}"
    );
}

/// r1474：未见 Done 提前结束 → ERROR/aborted、按档 flush、不伪造 usage。
#[then("llm.request 以 ERROR/aborted 收口并按档 flush 输入输出且不伪造 usage")]
pub(crate) fn t_c2830_abort(otel_bdd: &OtelBdd) {
    let records = otel_bdd.records();
    let llm = c2830_llm_span(&records);
    assert_eq!(
        prop(llm, "langfuse.observation.level"),
        Some("ERROR"),
        "r1474: 提前结束必须标 ERROR"
    );
    assert_eq!(
        prop(llm, "langfuse.observation.status_message"),
        Some("aborted"),
        "r1474: status_message 必须为 aborted"
    );
    let input = prop(llm, "langfuse.observation.input")
        .expect("r1474: abort 也必须按档 flush 已缓冲 input");
    assert!(input.contains("\"stream\":true"), "{input}");
    let output = prop(llm, "langfuse.observation.output")
        .expect("r1474: abort 必须按档 flush 已缓冲 output");
    assert!(output.contains("半程"), "{output}");
    assert!(
        prop(llm, "langfuse.observation.usage_details").is_none(),
        "r1474: 不得伪造 usage_details"
    );
}
