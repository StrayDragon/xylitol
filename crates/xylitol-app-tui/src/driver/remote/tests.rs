use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;
use xylitol_plugin_sdk::{BashChunk, BashOutputSink};
use xylitol_protocol::method;
use xylitol_protocol::{PROTOCOL_VERSION, RpcResult, WIRE_FORMAT_JSONRPC};

use super::*;
use crate::host_client::{HostClientError, MuxStream};

#[derive(Clone)]
struct ScriptedHost {
    describe: RpcResult,
    mux_calls: Arc<AtomicUsize>,
}

impl ScriptedHost {
    fn describing(describe: RpcResult) -> Self {
        Self {
            describe,
            mux_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl HostClient for ScriptedHost {
    async fn unary(&self, method: &str, _payload: Value) -> Result<RpcResult, HostClientError> {
        if method == METHOD_HOST_DESCRIBE {
            return Ok(self.describe.clone());
        }
        Ok(RpcResult::ok_value(json!({})))
    }

    async fn respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        self.mux_calls.fetch_add(1, Ordering::SeqCst);
        Err(HostClientError::WireFormatMismatch {
            formats: vec!["legacy-binary".into()],
            expected: WIRE_FORMAT_JSONRPC,
        })
    }
}

fn describe_ok() -> RpcResult {
    RpcResult::ok_value(json!({"protocol": PROTOCOL_VERSION, "formats": [WIRE_FORMAT_JSONRPC]}))
}

#[derive(Clone)]
struct MuxOkHost {
    describe: RpcResult,
    subscribe_fails: bool,
    mux_calls: Arc<AtomicUsize>,
}

impl MuxOkHost {
    fn new(subscribe_fails: bool) -> Self {
        Self {
            describe: describe_ok(),
            subscribe_fails,
            mux_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl HostClient for MuxOkHost {
    async fn unary(&self, method: &str, _payload: Value) -> Result<RpcResult, HostClientError> {
        if method == METHOD_HOST_DESCRIBE {
            return Ok(self.describe.clone());
        }
        if method == METHOD_SUBSCRIBE {
            return if self.subscribe_fails {
                Err(HostClientError::transport("subscribe failed"))
            } else {
                Ok(RpcResult::ok_value(
                    json!({"session_id": "s-test", "seq": 0, "inflight": false}),
                ))
            };
        }
        Ok(RpcResult::ok_value(json!({})))
    }

    async fn respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        self.mux_calls.fetch_add(1, Ordering::SeqCst);
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[derive(Clone)]
struct QueueHost {
    revoked: bool,
}

#[async_trait]
impl HostClient for QueueHost {
    async fn unary(&self, method: &str, _payload: Value) -> Result<RpcResult, HostClientError> {
        match method {
            METHOD_HOST_DESCRIBE => Ok(describe_ok()),
            METHOD_SUBSCRIBE => Ok(RpcResult::ok_value(
                json!({"session_id": "s-test", "seq": 0, "inflight": false}),
            )),
            "revoke_queue_item" => {
                if self.revoked {
                    Ok(RpcResult::ok_value(json!({"revoked": true, "items": []})))
                } else {
                    Ok(RpcResult::error("not_found", "queue item not found: q9"))
                }
            }
            "queue_stats" => Ok(RpcResult::ok_value(json!({"items": []}))),
            _ => Ok(RpcResult::ok_value(json!({}))),
        }
    }

    async fn respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

async fn run_downlink_briefly(host: MuxOkHost) -> (usize, bool) {
    let mux_calls = host.mux_calls.clone();
    let driver = XyRemoteDriver::with_host(host, "s-test").with_tunings(LinkTunings {
        backoff_base: Duration::from_millis(1),
        backoff_cap: Duration::from_millis(2),
        survive_threshold: Duration::from_secs(1),
        coalesce_window: Duration::ZERO,
    });
    let life = driver.session_life.clone();
    let phase = driver.phase.clone();
    let handle = tokio::spawn(downlink_loop(driver.downlink_ctx()));
    tokio::time::sleep(Duration::from_millis(50)).await;
    life.cancel();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("downlink stops after cancel")
        .expect("downlink task joins");
    let live = matches!(
        *phase.lock().unwrap_or_else(|e| e.into_inner()),
        super::super::LinkPhase::Live
    );
    (mux_calls.load(Ordering::SeqCst), live)
}

fn new_driver(host: ScriptedHost) -> XyRemoteDriver<ScriptedHost> {
    XyRemoteDriver::with_host(host, "s-test")
}

fn frame(name: &str, params: Value) -> Notification {
    Notification {
        method: name.into(),
        params,
    }
}

#[tokio::test]
async fn attach_stops_on_protocol_mismatch_without_opening_mux() {
    let host = ScriptedHost::describing(RpcResult::ok_value(
        json!({"protocol": 3, "formats": [WIRE_FORMAT_JSONRPC]}),
    ));
    let mux_calls = host.mux_calls.clone();
    let mut driver = new_driver(host);
    let err = driver
        .attach_session()
        .await
        .expect_err("protocol mismatch is fatal");
    assert!(err.to_string().contains("protocol mismatch"), "{err}");
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::FatalClosed { .. }
    ));
    assert_eq!(mux_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn attach_stops_on_wire_format_mismatch_without_opening_mux() {
    let host = ScriptedHost::describing(RpcResult::ok_value(
        json!({"protocol": PROTOCOL_VERSION, "formats": ["legacy-binary"]}),
    ));
    let mux_calls = host.mux_calls.clone();
    let mut driver = new_driver(host);
    let err = driver
        .attach_session()
        .await
        .expect_err("format mismatch is fatal");
    assert!(err.to_string().contains("wire format mismatch"), "{err}");
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::FatalClosed { .. }
    ));
    assert_eq!(mux_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn matching_describe_passes_the_handshake() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver
        .check_host_protocol()
        .await
        .expect("matching describe passes");
    assert!(!matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::FatalClosed { .. }
    ));
}

#[tokio::test]
async fn downlink_loop_stops_on_handshake_mismatch_without_retry() {
    let host = ScriptedHost::describing(describe_ok());
    let mux_calls = host.mux_calls.clone();
    let driver = new_driver(host);
    tokio::time::timeout(Duration::from_secs(5), downlink_loop(driver.downlink_ctx()))
        .await
        .expect("fatal mismatch ends the downlink loop instead of retrying");
    assert_eq!(mux_calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::FatalClosed { .. }
    ));
}

#[tokio::test]
async fn downlink_loop_retries_after_subscribe_failure() {
    let (mux_calls, subscribed_ok) = run_downlink_briefly(MuxOkHost::new(true)).await;
    assert!(
        mux_calls >= 2,
        "a failed subscribe must reconnect instead of stopping"
    );
    assert!(
        !subscribed_ok,
        "a failed subscribe must not mark the link ready"
    );
}

#[tokio::test]
async fn downlink_loop_reconnects_after_mux_closes() {
    let (mux_calls, subscribed_ok) = run_downlink_briefly(MuxOkHost::new(false)).await;
    assert!(
        mux_calls >= 2,
        "a closed mux stream must be reopened after the pause"
    );
    assert!(!subscribed_ok, "the link is not ready once the mux closes");
}

#[tokio::test]
async fn session_event_frame_advances_seq_and_pushes_agent_event() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 7,
                "event": {"type": "text_delta", "text": "hi"},
            }),
        ),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 7);
    let pushed = pushed.into_inner().expect("pushed");
    assert_eq!(pushed.len(), 1);
    assert!(matches!(&pushed[0], XyEvent::TextDelta(text) if text == "hi"));
}

#[tokio::test]
async fn session_event_with_unknown_variant_is_skipped_but_seq_advances() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({"session_id": "s-test", "seq": 8, "event": {"type": "future_kind"}}),
        ),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 8);
    assert!(pushed.lock().expect("pushed").is_empty());
}

#[tokio::test]
async fn malformed_session_event_is_dropped_without_touching_seq() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.last_seq.store(4, Ordering::SeqCst);
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(method::SESSION_EVENT, json!({"session_id": "s-test"})),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn bash_output_frame_reaches_active_sink() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let (tx, mut rx) = mpsc::channel(4);
    *driver.bash_run_sink.lock().expect("sink slot") = Some(BashOutputSink {
        tx,
        cancel: CancellationToken::new(),
    });
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::BASH_OUTPUT,
            json!({"session_id": "s-test", "bash_id": "b1", "seq": 2, "data": "out\n"}),
        ),
    )
    .await;
    assert_eq!(
        rx.try_recv().expect("chunk delivered"),
        BashChunk {
            bash_id: "b1".into(),
            seq: 2,
            data: "out\n".into(),
        }
    );
}

#[tokio::test]
async fn approval_frame_reaches_reverse_rpc_with_call_id_and_params() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    driver.set_reverse_rpc_notify(Arc::new(
        move |call_id: String, name: String, params: Value| {
            sink.lock().expect("seen").push((call_id, name, params));
        },
    ));
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::APPROVAL_REQUESTED,
            json!({"call_id": "c1", "tool": "bash"}),
        ),
    )
    .await;
    let seen = seen.lock().expect("seen");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "c1");
    assert_eq!(seen[0].1, method::APPROVAL_REQUESTED);
    assert_eq!(seen[0].2["tool"], "bash");
}

#[tokio::test]
async fn resync_required_frame_resubscribes_from_zero() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.last_seq.store(9, Ordering::SeqCst);
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_RESYNC_REQUIRED,
            json!({"session_id": "s-test"}),
        ),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 0);
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::ResyncNeeded
    ));
}

#[tokio::test]
async fn subscribed_after_resync_returns_link_to_live_and_keeps_rebuild() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_RESYNC_REQUIRED,
            json!({"session_id": "s-test"}),
        ),
    )
    .await;
    assert_eq!(driver.link_health(), super::super::LinkHealth::Down);
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_SUBSCRIBED,
            json!({"session_id": "s-test", "seq": 0, "inflight": false}),
        ),
    )
    .await;
    assert_eq!(
        driver.link_health(),
        super::super::LinkHealth::Up,
        "session/subscribed MUST return a resynced link to Live"
    );
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::Live
    ));
    assert!(
        driver.take_resync_rebuild(),
        "resync rebuild MUST survive the Live transition"
    );
    assert_eq!(
        driver.link_health(),
        super::super::LinkHealth::Up,
        "consuming the rebuild MUST not downgrade Live"
    );
    assert!(!driver.take_resync_rebuild(), "rebuild is one-shot");
}

#[tokio::test]
async fn subscribed_frame_marks_connecting_phase_live() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_SUBSCRIBED,
            json!({"session_id": "s-test", "seq": 3, "inflight": false}),
        ),
    )
    .await;
    assert_eq!(driver.link_health(), super::super::LinkHealth::Up);
    assert!(!driver.take_resync_rebuild(), "no resync means no rebuild");
}

#[tokio::test]
async fn subscribed_frame_does_not_revive_fatal_closed() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    *driver.phase.lock().expect("phase slot") = super::super::LinkPhase::FatalClosed {
        reason: "protocol mismatch".into(),
    };
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_SUBSCRIBED,
            json!({"session_id": "s-test", "seq": 1, "inflight": false}),
        ),
    )
    .await;
    assert!(matches!(
        *driver.phase.lock().expect("phase slot"),
        super::super::LinkPhase::FatalClosed { .. }
    ));
    assert_eq!(driver.link_health(), super::super::LinkHealth::Down);
}

#[derive(Clone)]
struct ExportHost {
    result: Value,
}

#[async_trait]
impl HostClient for ExportHost {
    async fn unary(&self, _method: &str, _payload: Value) -> Result<RpcResult, HostClientError> {
        Ok(RpcResult::ok_value(self.result.clone()))
    }
    async fn respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }
    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Err(HostClientError::WireFormatMismatch {
            formats: vec!["legacy-binary".into()],
            expected: WIRE_FORMAT_JSONRPC,
        })
    }
}

#[tokio::test]
async fn export_html_writes_inline_content() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("export.html");
    let host = ExportHost {
        result: json!({ "content": "<html>hi</html>" }),
    };
    let mut driver = XyRemoteDriver::with_host(host, "s-test");
    let outcome = driver
        .exec_export_html(Some(path.to_string_lossy().into_owned()))
        .await
        .expect("export");
    match outcome {
        DispatchOutcome::ExportedPath(written) => {
            assert_eq!(written, path.to_string_lossy().into_owned())
        }
        other => panic!("unexpected export outcome: {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(&path).expect("written"),
        "<html>hi</html>"
    );
}

#[tokio::test]
async fn export_jsonl_falls_back_to_host_path() {
    let host = ExportHost {
        result: json!({ "path": "/tmp/server-export.jsonl" }),
    };
    let mut driver = XyRemoteDriver::with_host(host, "s-test");
    let outcome = driver
        .exec_export_jsonl(Some("client.jsonl".into()))
        .await
        .expect("export");
    match outcome {
        DispatchOutcome::ExportedPath(written) => assert_eq!(written, "/tmp/server-export.jsonl"),
        other => panic!("unexpected export outcome: {other:?}"),
    }
}

#[tokio::test]
async fn export_jsonl_writes_inline_content() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("export.jsonl");
    let host = ExportHost {
        result: json!({ "content": "{\"type\":\"message\"}\n" }),
    };
    let mut driver = XyRemoteDriver::with_host(host, "s-test");
    let outcome = driver
        .exec_export_jsonl(Some(path.to_string_lossy().into_owned()))
        .await
        .expect("export");
    match outcome {
        DispatchOutcome::ExportedPath(written) => {
            assert_eq!(written, path.to_string_lossy().into_owned())
        }
        other => panic!("unexpected export outcome: {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(&path).expect("written"),
        "{\"type\":\"message\"}\n"
    );
}

#[tokio::test]
async fn remote_tui_oauth_is_unsupported() {
    let mut driver = XyRemoteDriver::with_host(MuxOkHost::new(false), "s-test");
    assert!(driver.mcp_oauth_start("server").await.is_err());
    assert!(driver.mcp_oauth_cancel("flow").await.is_err());
    assert!(driver.mcp_oauth_status("flow").await.is_err());
}
#[derive(Clone, Default)]
struct DetachHost {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
}

#[async_trait]
impl HostClient for DetachHost {
    async fn unary(&self, method: &str, payload: Value) -> Result<RpcResult, HostClientError> {
        self.calls
            .lock()
            .expect("calls lock")
            .push((method.to_string(), payload));
        Ok(RpcResult::ok_value(json!({})))
    }

    async fn respond(&self, _call_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Err(HostClientError::transport("detach test does not need mux"))
    }
}

#[tokio::test]
async fn detach_session_sends_session_detach_unary() {
    use xylitol_protocol::registry::METHOD_SESSION_DETACH;

    let host = DetachHost::default();
    let calls = host.calls.clone();
    let mut driver = XyRemoteDriver::with_host(host, "s-detach");
    driver.detach_session().await.expect("detach");
    let calls = calls.lock().expect("calls lock");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, METHOD_SESSION_DETACH);
    assert_eq!(
        calls[0].1.get("session_id").and_then(Value::as_str),
        Some("s-detach")
    );
}

#[tokio::test]
async fn fatal_closed_reports_down_and_reason() {
    let host = ScriptedHost::describing(RpcResult::ok_value(
        json!({"protocol": 3, "formats": [WIRE_FORMAT_JSONRPC]}),
    ));
    let mut driver = new_driver(host);
    let err = driver
        .attach_session()
        .await
        .expect_err("protocol mismatch is fatal");
    assert!(err.to_string().contains("protocol mismatch"), "{err}");
    assert_eq!(driver.link_health(), super::super::LinkHealth::Down);
    let wait_err = driver
        .wait_subscribed()
        .await
        .expect_err("fatal blocks subscribe");
    assert!(
        wait_err.to_string().contains("protocol mismatch"),
        "{wait_err}"
    );
}

#[tokio::test]
async fn resync_take_rebuild_transitions_to_connecting() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    *driver.phase.lock().expect("phase") = super::super::LinkPhase::ResyncNeeded;
    assert_eq!(driver.link_health(), super::super::LinkHealth::Down);
    assert!(
        driver.take_resync_rebuild(),
        "ResyncNeeded MUST be consumable"
    );
    assert!(matches!(
        *driver.phase.lock().expect("phase"),
        super::super::LinkPhase::Connecting
    ));
    assert!(!driver.take_resync_rebuild(), "only one rebuild per resync");
}

#[tokio::test]
async fn agent_end_resets_turn_cancel() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.turn_cancel.lock().expect("turn cancel").cancel();
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 9,
                "event": {"type": "agent_end"},
            }),
        ),
    )
    .await;
    assert!(
        !driver
            .turn_cancel
            .lock()
            .expect("turn cancel")
            .is_cancelled(),
        "AgentEnd MUST reset the turn cancel token"
    );
}

#[tokio::test]
async fn explicit_resubscribe_clears_ended_observe() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    *driver.attach_observe.lock().expect("observe") = AttachObserve::Ended;
    driver.restart_downlink().await;
    assert_eq!(driver.attach_observe(), AttachObserve::Live);
}

fn models_host() -> ExportHost {
    ExportHost {
        result: json!({"models": [
            {"id": "m-fast", "display_name": "Fast", "context_window": 123}
        ]}),
    }
}

#[tokio::test]
async fn unregistered_downlink_method_frame_is_ignored() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.last_seq.store(5, Ordering::SeqCst);
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame("lsp/publishDiagnostics", json!({"session_id": "s-test"})),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 5);
    assert!(pushed.into_inner().expect("pushed").is_empty());
}

#[tokio::test]
async fn session_event_with_undecodable_workflow_payload_is_skipped() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 31,
                "event": {
                    "type": "workflow_updated",
                    "workflow": {"goal": "g", "tasks": [{"id": 7, "description": "d"}]}
                },
            }),
        ),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 31);
    assert!(pushed.into_inner().expect("pushed").is_empty());
}

#[tokio::test]
async fn session_event_queue_update_refreshes_cached_queue_stats() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.skip_cold_replay.store(true, Ordering::SeqCst);
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 32,
                "event": {"type": "queue_update", "items": [
                    {"id": "qs0", "kind": "steer", "preview": "s0"},
                    {"id": "qs1", "kind": "steer", "preview": "s1"},
                    {"id": "qf0", "kind": "follow_up", "preview": "f0"},
                    {"id": "qf1", "kind": "follow_up", "preview": "f1"},
                    {"id": "qf2", "kind": "follow_up", "preview": "f2"}
                ]},
            }),
        ),
    )
    .await;
    let (steer, follow) = {
        let stats = driver.queue_stats();
        (stats.steer_count, stats.follow_up_count)
    };
    assert_eq!((steer, follow), (2, 3));
    assert_eq!(pushed.into_inner().expect("pushed").len(), 1);
}

#[tokio::test]
async fn cold_replay_tape_frame_is_dropped_while_skip_flag_set() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver.skip_cold_replay.store(true, Ordering::SeqCst);
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 34,
                "event": {"type": "text_delta", "text": "hidden"},
            }),
        ),
    )
    .await;
    assert_eq!(driver.last_seq.load(Ordering::SeqCst), 34);
    assert!(pushed.into_inner().expect("pushed").is_empty());
}

#[tokio::test]
async fn session_event_model_select_resolves_and_caches_model() {
    let driver = XyRemoteDriver::with_host(models_host(), "s-test");
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 33,
                "event": {"type": "model_select", "provider": "p", "model_id": "m-fast"},
            }),
        ),
    )
    .await;
    let cached_id = driver
        .cached_model
        .lock()
        .expect("cached model")
        .as_ref()
        .map(|m| m.id.clone());
    assert_eq!(cached_id, Some("m-fast".to_string()));
    let pushed = pushed.into_inner().expect("pushed");
    assert!(matches!(
        &pushed[0],
        XyEvent::ModelSelect { model_id, .. } if model_id == "m-fast"
    ));
}

#[tokio::test]
async fn session_model_frame_pushes_model_select_and_caches_model() {
    let driver = XyRemoteDriver::with_host(models_host(), "s-test");
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::SESSION_MODEL,
            json!({"session_id": "s-test", "provider": "p", "model_id": "m-fast"}),
        ),
    )
    .await;
    let cached_id = driver
        .cached_model
        .lock()
        .expect("cached model")
        .as_ref()
        .map(|m| m.id.clone());
    assert_eq!(cached_id, Some("m-fast".to_string()));
    let pushed = pushed.into_inner().expect("pushed");
    assert!(matches!(
        &pushed[0],
        XyEvent::ModelSelect { provider, model_id, .. }
            if provider == "p" && model_id == "m-fast"
    ));
}

#[tokio::test]
async fn agent_end_frame_moves_inflight_attach_to_ended() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    *driver.attach_observe.lock().expect("observe") = AttachObserve::Inflight;
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_EVENT,
            json!({
                "session_id": "s-test",
                "seq": 21,
                "event": {"type": "agent_end"},
            }),
        ),
    )
    .await;
    assert_eq!(driver.attach_observe(), AttachObserve::Ended);
}

#[tokio::test]
async fn session_resources_frame_populates_caches_and_gate_notice() {
    let mut driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    handle_frame(
        &ctx,
        &|_: XyEvent| {},
        frame(
            method::SESSION_RESOURCES,
            json!({
                "session_id": "s-test",
                "snapshot": {
                    "skill_names": ["lab_probe"],
                    "mcp_connected": [["srv", 2]],
                    "mcp_configured": 1,
                    "mcp_diag_short": [],
                    "mcp_connecting_label": null,
                    "mcp_servers": [
                        {"id": "srv", "phase": "connected", "tools_armed": true, "tool_count": 2}
                    ],
                    "mcp_bootstrap_complete": true,
                    "tools_table_frozen": false,
                    "mcp_gate_notice": "mcp gate held",
                },
            }),
        ),
    )
    .await;
    assert_eq!(
        driver.dollar_skill_catalog(),
        vec![("lab_probe".to_string(), String::new())]
    );
    assert_eq!(
        driver.take_mcp_gate_notice().await,
        Some("mcp gate held".to_string())
    );
    assert_eq!(driver.take_mcp_gate_notice().await, None);
    assert!(driver.loaded_resources_cached().is_some());
}

#[tokio::test]
async fn mcp_oauth_started_frame_pushes_session_info_changed() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            "mcp/oauth_started",
            json!({
                "kind": "started",
                "flow": {
                    "flow_id": "f1",
                    "server": "srv",
                    "auth_url": "https://auth.example/start",
                    "redirect_uri": "",
                    "scopes": [],
                    "expires_at_ms": 99,
                },
            }),
        ),
    )
    .await;
    let pushed = pushed.into_inner().expect("pushed");
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        XyEvent::SessionInfoChanged { key, .. } if key == "mcp_oauth"
    ));
}

#[tokio::test]
async fn approval_frame_without_reverse_rpc_notify_is_dropped() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    let ctx = driver.downlink_ctx();
    let pushed = Mutex::new(Vec::new());
    let push = |ev: XyEvent| pushed.lock().expect("pushed").push(ev);
    handle_frame(
        &ctx,
        &push,
        frame(
            method::APPROVAL_REQUESTED,
            json!({"call_id": "c9", "tool": "bash"}),
        ),
    )
    .await;
    assert!(pushed.into_inner().expect("pushed").is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_queue_item_reports_success_from_host_result() {
    let driver = XyRemoteDriver::with_host(QueueHost { revoked: true }, "s-test");
    let outcome = XyDriver::revoke_queue_item(&driver, "q1");
    assert!(
        outcome.revoked,
        "a host result without an explicit revoked flag must count as revoked"
    );
    assert!(outcome.items.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoke_queue_item_not_found_is_not_revoked() {
    let driver = XyRemoteDriver::with_host(QueueHost { revoked: false }, "s-test");
    let outcome = XyDriver::revoke_queue_item(&driver, "q9");
    assert!(
        !outcome.revoked,
        "a NotFound host answer must surface as not revoked"
    );
}
