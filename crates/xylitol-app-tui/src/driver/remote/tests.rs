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
