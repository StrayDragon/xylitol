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

async fn run_downlink_briefly(host: MuxOkHost) -> (usize, bool) {
    let mux_calls = host.mux_calls.clone();
    let driver = XyRemoteDriver::with_host(host, "s-test").with_tunings(LinkTunings {
        backoff_base: Duration::from_millis(1),
        backoff_cap: Duration::from_millis(2),
        survive_threshold: Duration::from_secs(1),
        coalesce_window: Duration::ZERO,
    });
    let life = driver.session_life.clone();
    let subscribed_ok = driver.subscribed_ok.clone();
    let handle = tokio::spawn(downlink_loop(driver.downlink_ctx()));
    tokio::time::sleep(Duration::from_millis(50)).await;
    life.cancel();
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("downlink stops after cancel")
        .expect("downlink task joins");
    (
        mux_calls.load(Ordering::SeqCst),
        subscribed_ok.load(Ordering::SeqCst),
    )
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
    assert!(driver.fatal.lock().expect("fatal slot").is_some());
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
    assert!(driver.fatal.lock().expect("fatal slot").is_some());
    assert_eq!(mux_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn matching_describe_passes_the_handshake() {
    let driver = new_driver(ScriptedHost::describing(describe_ok()));
    driver
        .check_host_protocol()
        .await
        .expect("matching describe passes");
    assert!(driver.fatal.lock().expect("fatal slot").is_none());
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
    assert!(driver.fatal.lock().expect("fatal slot").is_some());
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
    assert!(driver.resync_needed.load(Ordering::SeqCst));
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
