//! Steps for the attach connection-resilience family (ath41/ath43/ath44, c2480).
//!
//! Seam: [`XyRemoteDriver`] over a scripted mock [`HostClient`] — the same
//! shape as the in-crate `driver/remote.rs` tests, with injected micro-second
//! tunings so backoff/coalesce timing assertions finish in milliseconds.

use crate::tests::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use std::collections::VecDeque;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use crate::protocol::RpcMessage;
use crate::protocol::wire::envelope::PROTOCOL_VERSION;
use crate::{HostClient, HostClientError, MuxStream, XyDriver};
use crate::{LinkTunings, XyRemoteDriver};

/// Tunings small enough for millisecond-scale timing assertions.
fn micro_tunings() -> LinkTunings {
    LinkTunings {
        backoff_base: Duration::from_millis(40),
        backoff_cap: Duration::from_millis(400),
        survive_threshold: Duration::from_millis(150),
        coalesce_window: Duration::from_millis(60),
    }
}

#[derive(Debug)]
enum MuxScript {
    FailTransport(String),
    FailProtocol {
        got: u32,
    },
    /// A live connection with an injectable frame channel; `life` ends it.
    Live {
        life: Option<Duration>,
    },
}

#[derive(Default)]
struct ScriptState {
    script: VecDeque<MuxScript>,
    mux_calls: Vec<Instant>,
    subscribes: Vec<u64>,
    senders: Vec<tokio::sync::mpsc::UnboundedSender<RpcMessage>>,
}

#[derive(Clone, Default)]
struct ScriptedMuxHost {
    state: Arc<StdMutex<ScriptState>>,
}

impl ScriptedMuxHost {
    fn script(&self, script: Vec<MuxScript>) {
        self.state.lock().unwrap().script = script.into();
    }

    /// Frame injection tolerant of an already-dead connection: a closed
    /// channel means the old generation's loop exited before consuming —
    /// which also satisfies the "late frame must not surface" property.
    fn send_frame(&self, conn: usize, frame: RpcMessage) {
        if let Some(tx) = self.state.lock().unwrap().senders.get(conn) {
            let _ = tx.send(frame);
        }
    }

    fn mux_call_count(&self) -> usize {
        self.state.lock().unwrap().mux_calls.len()
    }

    fn sender_count(&self) -> usize {
        self.state.lock().unwrap().senders.len()
    }
}

fn session_event(seq: u64, text: &str) -> RpcMessage {
    RpcMessage::ServerRequest {
        rpc_id: uuid::Uuid::new_v4().to_string(),
        method: "session/event".into(),
        payload: serde_json::json!({
            "seq": seq,
            "event": { "type": "text_delta", "text": text },
        }),
    }
}

fn session_subscribed() -> RpcMessage {
    RpcMessage::ServerRequest {
        rpc_id: uuid::Uuid::new_v4().to_string(),
        method: "session/subscribed".into(),
        payload: serde_json::json!({ "session_id": "s-res", "seq": 0 }),
    }
}

#[async_trait::async_trait]
impl HostClient for ScriptedMuxHost {
    async fn unary(
        &self,
        method: &str,
        payload: serde_json::Value,
    ) -> Result<crate::protocol::RpcResult, HostClientError> {
        if method == "subscribe" {
            let seq = payload
                .get("last_seq")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            self.state.lock().unwrap().subscribes.push(seq);
        }
        let value = match method {
            "get_state" => serde_json::json!({ "leaf_entry_id": null, "model": null }),
            "get_available_models" => serde_json::json!({ "models": [] }),
            "get_commands" => serde_json::json!({ "commands": [] }),
            "loaded_resources" => serde_json::json!({ "mcp_configured": 0 }),
            _ => serde_json::json!({}),
        };
        Ok(crate::protocol::RpcResult::ok_value(value))
    }

    async fn respond(
        &self,
        _rpc_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        let behavior = {
            let mut state = self.state.lock().unwrap();
            state.mux_calls.push(Instant::now());
            state
                .script
                .pop_front()
                .unwrap_or(MuxScript::FailTransport("script exhausted".into()))
        };
        match behavior {
            MuxScript::FailTransport(message) => Err(HostClientError::transport(message)),
            MuxScript::FailProtocol { got } => Err(HostClientError::ProtocolMismatch {
                got,
                expected: PROTOCOL_VERSION,
            }),
            MuxScript::Live { life } => {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<RpcMessage>();
                self.state.lock().unwrap().senders.push(tx);
                Ok(Box::pin(async_stream::stream! {
                    let deadline = life.map(|l| tokio::time::Instant::now() + l);
                    loop {
                        match deadline {
                            Some(d) => {
                                tokio::select! {
                                    _ = tokio::time::sleep_until(d) => break,
                                    item = rx.recv() => match item {
                                        Some(frame) => yield Ok(frame),
                                        None => break,
                                    },
                                }
                            }
                            None => match rx.recv().await {
                                Some(frame) => yield Ok(frame),
                                None => break,
                            },
                        }
                    }
                }) as MuxStream)
            }
        }
    }
}

struct ResilienceRig {
    host: ScriptedMuxHost,
    driver: XyRemoteDriver<ScriptedMuxHost>,
}

pub struct ResilienceBdd {
    rig: RefCell<Option<ResilienceRig>>,
    /// (receive instant, text) pairs collected from a `run` stream.
    pub received: RefCell<Vec<(Instant, String)>>,
    pub push_t0: RefCell<Option<Instant>>,
    pub attach_result: RefCell<Option<Result<(), XyDriverError>>>,
}

#[fixture]
pub fn resilience_bdd() -> ResilienceBdd {
    ResilienceBdd {
        rig: RefCell::new(None),
        received: RefCell::new(Vec::new()),
        push_t0: RefCell::new(None),
        attach_result: RefCell::new(None),
    }
}

fn take_rig(bdd: &ResilienceBdd) -> ResilienceRig {
    bdd.rig.borrow_mut().take().expect("resilience rig mounted")
}

fn put_rig(bdd: &ResilienceBdd, rig: ResilienceRig) {
    *bdd.rig.borrow_mut() = Some(rig);
}

async fn wait_for(mut cond: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let deadline = Instant::now() + timeout;
    while !cond() {
        assert!(Instant::now() < deadline, "timeout waiting for {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

fn mount_rig(host: ScriptedMuxHost) -> ResilienceRig {
    let driver = XyRemoteDriver::with_host(host.clone(), "s-res").with_tunings(micro_tunings());
    ResilienceRig { host, driver }
}

// ── ath44 hello mismatch (mock seam) ───────────────────────────────

#[given("mock HostClient 在 mux 首帧发送版本不符的 server_hello")]
fn g_hello_mismatch(resilience_bdd: &ResilienceBdd) {
    let host = ScriptedMuxHost::default();
    host.script(vec![MuxScript::FailProtocol { got: 99 }]);
    put_rig(resilience_bdd, mount_rig(host));
}

#[when("attach 客户端完成首帧校验")]
async fn w_hello_attach(resilience_bdd: &ResilienceBdd) {
    let mut rig = take_rig(resilience_bdd);
    let result = rig.driver.attach_session().await;
    resilience_bdd.attach_result.borrow_mut().replace(result);
    put_rig(resilience_bdd, rig);
}

#[then("driver MUST 报版本错误并终止该代循环且 MUST NOT 进入重试循环")]
fn t_hello_fatal(resilience_bdd: &ResilienceBdd) {
    let attach = resilience_bdd
        .attach_result
        .borrow_mut()
        .take()
        .expect("attach ran");
    assert!(
        attach.is_err(),
        "version mismatch MUST fail attach: {attach:?}"
    );
    let mut rig = take_rig(resilience_bdd);
    assert_eq!(
        rig.host.mux_call_count(),
        1,
        "fatal protocol mismatch MUST NOT retry-loop"
    );
    let drained = rig.driver.drain_idle_events();
    let fatal = drained.iter().any(|ev| {
        matches!(
            ev,
            crate::agent::runtime::XyEvent::Error(err)
                if err.message.contains("protocol mismatch") && err.message.contains("99")
        )
    });
    assert!(
        fatal,
        "driver MUST surface the version error, got {drained:?}"
    );
}

// ── ath41 backoff escalation (mock seam) ───────────────────────────

#[given("以注入短常量的 mock HostClient 驱动 attach 重连")]
async fn g_backoff_rig(resilience_bdd: &ResilienceBdd) {
    let host = ScriptedMuxHost::default();
    host.script(vec![
        MuxScript::Live {
            life: Some(Duration::from_millis(20)),
        },
        MuxScript::Live {
            life: Some(Duration::from_millis(20)),
        },
        MuxScript::Live {
            life: Some(Duration::from_millis(250)),
        },
        MuxScript::Live { life: None },
    ]);
    let mut rig = mount_rig(host);
    rig.driver.attach_session().await.expect("attach");
    put_rig(resilience_bdd, rig);
}

#[when("连续建立存活不足归零阈值即断开的连接")]
async fn w_backoff_flaps(resilience_bdd: &ResilienceBdd) {
    let host = resilience_bdd
        .rig
        .borrow()
        .as_ref()
        .expect("rig mounted")
        .host
        .clone();
    wait_for(
        || host.mux_call_count() >= 4,
        Duration::from_secs(5),
        "four mux attempts",
    )
    .await;
}

#[when("某次连接存活达到归零阈值后断开")]
async fn w_backoff_survives(resilience_bdd: &ResilienceBdd) {
    let host = resilience_bdd
        .rig
        .borrow()
        .as_ref()
        .expect("rig mounted")
        .host
        .clone();
    wait_for(
        || host.mux_call_count() >= 4,
        Duration::from_secs(5),
        "surviving connection attempt",
    )
    .await;
}

#[then("重试间隔 MUST 逐次翻倍升级")]
fn t_backoff_escalates(resilience_bdd: &ResilienceBdd) {
    let rig = resilience_bdd.rig.borrow();
    let rig = rig.as_ref().expect("rig mounted");
    let calls = rig.host.state.lock().unwrap().mux_calls.clone();
    assert!(calls.len() >= 4, "need four attempts, got {}", calls.len());
    let young = Duration::from_millis(20);
    // implied backoff = gap between attempts minus the scripted young life
    let implied = |gap: Duration| gap.saturating_sub(young);
    let b2 = implied(calls[1] - calls[0]);
    let b3 = implied(calls[2] - calls[1]);
    assert!(
        b2 >= Duration::from_millis(60) && b2 <= Duration::from_millis(160),
        "first retry ≈ 2×base(40ms), got {b2:?}"
    );
    assert!(
        b3 >= Duration::from_millis(128) && b3 <= Duration::from_millis(280),
        "second retry ≈ 4×base(40ms), got {b3:?}"
    );
    assert!(
        b3 > b2 + Duration::from_millis(40),
        "MUST escalate, got {b2:?} → {b3:?}"
    );
}

#[then("下次重试间隔 MUST 回落到起点")]
fn t_backoff_resets(resilience_bdd: &ResilienceBdd) {
    let rig = resilience_bdd.rig.borrow();
    let rig = rig.as_ref().expect("rig mounted");
    let calls = rig.host.state.lock().unwrap().mux_calls.clone();
    assert!(calls.len() >= 4, "need four attempts, got {}", calls.len());
    // conn3 lived 250ms ≥ survive threshold(150ms) → next backoff resets
    let long = Duration::from_millis(250);
    let b4 = (calls[3] - calls[2]).saturating_sub(long);
    assert!(
        b4 <= Duration::from_millis(120),
        "survived connection MUST reset backoff to base(40ms), got {b4:?}"
    );
}

// ── ath41 stale generation (mock seam) ─────────────────────────────

#[given("已有一条订阅中的 mock mux 连接")]
async fn g_stale_conn(resilience_bdd: &ResilienceBdd) {
    let host = ScriptedMuxHost::default();
    host.script(vec![
        MuxScript::Live { life: None },
        MuxScript::Live { life: None },
    ]);
    let mut rig = mount_rig(host);
    rig.driver.attach_session().await.expect("attach");
    wait_for(
        || rig.host.sender_count() >= 1,
        Duration::from_secs(2),
        "first connection",
    )
    .await;
    // clear the cold-replay filter so scripted tape passes as live
    rig.host.send_frame(0, session_subscribed());
    tokio::time::sleep(Duration::from_millis(30)).await;
    put_rig(resilience_bdd, rig);
}

#[when("触发重订或换会话产生新代循环后旧代连接迟到推入事件")]
async fn w_stale_late_frame(resilience_bdd: &ResilienceBdd) {
    let mut rig = take_rig(resilience_bdd);
    crate::app::core::dispatch::dispatch(
        &mut rig.driver,
        crate::protocol::Command::SwitchSession {
            session_path: "s-res-2".into(),
        },
    )
    .await
    .expect("switch");
    wait_for(
        || rig.host.sender_count() >= 2,
        Duration::from_secs(2),
        "second connection",
    )
    .await;
    // clear cold-replay on the new generation, then inject frames
    rig.host.send_frame(1, session_subscribed());
    rig.host.send_frame(0, session_event(1, "stale-payload"));
    rig.host.send_frame(1, session_event(2, "fresh-payload"));
    tokio::time::sleep(Duration::from_millis(120)).await;
    put_rig(resilience_bdd, rig);
}

#[then("该迟到事件 MUST NOT 出现在新代的 drain 结果中")]
fn t_stale_dropped(resilience_bdd: &ResilienceBdd) {
    let mut rig = take_rig(resilience_bdd);
    let drained = rig.driver.drain_idle_events();
    let texts: Vec<String> = drained
        .iter()
        .filter_map(|ev| match ev {
            crate::agent::runtime::XyEvent::TextDelta(t) => Some(t.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts.contains(&"fresh-payload".into()),
        "new generation MUST see its own events, got {texts:?}"
    );
    assert!(
        !texts.contains(&"stale-payload".into()),
        "old generation late frame MUST be dropped, got {texts:?}"
    );
}

// ── ath43 coalescing burst (mock seam) ─────────────────────────────

#[given("合帧窗口开启")]
async fn g_coalesce_window(resilience_bdd: &ResilienceBdd) {
    let host = ScriptedMuxHost::default();
    host.script(vec![MuxScript::Live { life: None }]);
    let mut rig = mount_rig(host);
    rig.driver.attach_session().await.expect("attach");
    wait_for(
        || rig.host.sender_count() >= 1,
        Duration::from_secs(2),
        "first connection",
    )
    .await;
    rig.host.send_frame(0, session_subscribed());
    tokio::time::sleep(Duration::from_millis(30)).await;
    put_rig(resilience_bdd, rig);
}

#[when("窗口内连续到达多条下行事件")]
async fn w_coalesce_burst(resilience_bdd: &ResilienceBdd) {
    let mut rig = take_rig(resilience_bdd);
    let mut stream = rig.driver.run("hi").await;
    let received: Arc<StdMutex<Vec<(Instant, String)>>> = Arc::new(StdMutex::new(Vec::new()));
    let sink = received.clone();
    let collector = tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                item = stream.next() => match item {
                    Some(crate::agent::runtime::XyEvent::TextDelta(t)) => {
                        sink.lock().unwrap().push((Instant::now(), t));
                    }
                    Some(_) => {}
                    None => break,
                },
            }
        }
    });
    // a tight burst inside one coalesce window
    *resilience_bdd.push_t0.borrow_mut() = Some(Instant::now());
    for seq in 1..=6u64 {
        rig.host
            .send_frame(0, session_event(seq, &format!("burst-{seq}")));
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    collector.abort();
    *resilience_bdd.received.borrow_mut() = received.lock().unwrap().clone();
    put_rig(resilience_bdd, rig);
}

#[then("客户端 MUST 以一次投影批消费且消费批数等于合并后批数而非事件条数")]
fn t_coalesce_single_batch(resilience_bdd: &ResilienceBdd) {
    let received = resilience_bdd.received.borrow();
    let t0 = resilience_bdd.push_t0.borrow().expect("burst pushed");
    assert_eq!(
        received.len(),
        6,
        "all burst events MUST arrive, got {:?}",
        received
    );
    let first_delay = received[0].0.duration_since(t0);
    assert!(
        first_delay >= Duration::from_millis(45),
        "first delivery MUST wait the coalesce window, got {first_delay:?}"
    );
    let spread = received[5].0 - received[0].0;
    assert!(
        spread <= Duration::from_millis(25),
        "one window ⇒ one projection batch: spread {spread:?}"
    );
}
