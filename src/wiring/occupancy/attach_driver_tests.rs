//! Occupancy + attach-driver tests. Driver implementation: `xylitol-app-tui`.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use xylitol_app_tui::XyRemoteDriver;
use xylitol_app_tui::driver::{
    LoadedResourcesSnapshot as AppLoadedResourcesSnapshot, XyDriver, XyEvent,
};
use xylitol_app_tui::host_client::{HostClient, HostClientError, InProcessClient, MuxStream};
use xylitol_protocol::RpcMessage;
use xylitol_protocol::SessionTreeKind;
use xylitol_runtime::driver::LoadedResourcesSnapshot;
use xylitol_serve::occupancy::{
    HostState, handle_unary, materialize_writer, materialize_writer_at,
};

use super::host_state_for_test;
use super::host_state_for_test_with_default_model;
use super::host_state_for_test_with_mcp;
use super::in_process_client;
use crate::wiring::occupancy::runtime::{ServerConfig, serve};

#[test]
fn empty_session_arg_mints_uuid() {
    let driver = XyRemoteDriver::new("http://127.0.0.1:9", "");
    let sid = driver.session_id().expect("session");
    assert!(uuid::Uuid::parse_str(&sid).is_ok(), "{sid}");
    let kept = XyRemoteDriver::new("http://127.0.0.1:9", "keep-me");
    assert_eq!(kept.session_id().as_deref(), Some("keep-me"));
}

#[derive(Clone)]
struct SnapClient {
    calls: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl HostClient for SnapClient {
    async fn unary(
        &self,
        _method: &str,
        _payload: Value,
    ) -> Result<xylitol_protocol::RpcResult, HostClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        panic!("poll_mcp_bootstrap must not unary");
    }

    async fn respond(&self, _rpc_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[derive(Clone)]
struct FailPromptClient;

#[async_trait]
impl HostClient for FailPromptClient {
    async fn unary(
        &self,
        method: &str,
        _payload: Value,
    ) -> Result<xylitol_protocol::RpcResult, HostClientError> {
        if method == "prompt" {
            return Ok(xylitol_protocol::RpcResult::error(
                "session_write_held",
                "session s is being written by another process",
            ));
        }
        Ok(xylitol_protocol::RpcResult::ok_value(serde_json::json!({})))
    }

    async fn respond(&self, _rpc_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[tokio::test]
async fn prompt_business_error_ends_run_stream() {
    let mut driver = XyRemoteDriver::with_host(FailPromptClient, "s");
    driver.mark_subscribed_ok_for_test();
    let mut stream = driver.run("hi").await;
    let mut events = Vec::new();
    while let Some(ev) = stream.next().await {
        events.push(ev);
    }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, XyEvent::Error(err) if err.message.contains("prompt failed"))),
        "business error MUST yield Error: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(e, XyEvent::AgentEnd { .. })),
        "MUST NOT wait for AgentEnd after prompt failure: {events:?}"
    );
}

#[tokio::test]
async fn poll_mcp_bootstrap_is_cache_dirty_only() {
    let client = SnapClient {
        calls: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let mut driver = XyRemoteDriver::with_host(client.clone(), "s");
    assert!(!driver.poll_mcp_bootstrap().await);
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);
    driver.apply_resources_cache(AppLoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_bootstrap_complete: false,
        tools_table_frozen: false,
        mcp_connecting_label: Some("connecting 0/1".into()),
        ..AppLoadedResourcesSnapshot::default()
    });
    assert!(driver.poll_mcp_bootstrap().await);
    assert!(!driver.poll_mcp_bootstrap().await);
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);
}

/// ath37 / sr-abort1 (c2340): cancelling `reload_runtime` mid-flight MUST
/// ask the Host to cooperatively cancel the in-flight reload via the
/// existing `abort` unary and finish locally as cancelled.
#[derive(Clone)]
struct ReloadCancelClient {
    abort_called: Arc<std::sync::atomic::AtomicBool>,
    release: Arc<tokio::sync::Notify>,
}

#[async_trait]
impl HostClient for ReloadCancelClient {
    async fn unary(
        &self,
        method: &str,
        _payload: Value,
    ) -> Result<xylitol_protocol::RpcResult, HostClientError> {
        if method == "reload" {
            // Hold the reload open until the cooperative abort arrives.
            let release = self.release.clone();
            let _ = tokio::select! {
                _ = release.notified() => {}
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {}
            };
            return Ok(xylitol_protocol::RpcResult::ok_value(
                serde_json::json!({ "cancelled": true, "steps": [] }),
            ));
        }
        if method == "abort" {
            self.abort_called.store(true, Ordering::SeqCst);
            self.release.notify_waiters();
            return Ok(xylitol_protocol::RpcResult::ok_value(
                serde_json::json!({ "cancelled": true }),
            ));
        }
        Err(HostClientError::Transport(format!(
            "unexpected unary: {method}"
        )))
    }

    async fn respond(&self, _rpc_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

#[tokio::test]
async fn reload_cancel_requests_host_cooperative_abort() {
    use std::sync::atomic::AtomicBool;

    let client = ReloadCancelClient {
        abort_called: Arc::new(AtomicBool::new(false)),
        release: Arc::new(tokio::sync::Notify::new()),
    };
    let mut driver = XyRemoteDriver::with_host(client.clone(), "s");
    let token = CancellationToken::new();
    let cancel_token = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        cancel_token.cancel();
    });

    let report = driver.reload_runtime(&token).await.expect("reload report");
    assert!(
        report.cancelled,
        "report MUST be cancelled after token fires"
    );
    assert!(
        client.abort_called.load(Ordering::SeqCst),
        "cancel MUST reach the Host as a cooperative abort unary"
    );
}

#[derive(Clone, Default)]
struct CountingClient {
    unaries: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl HostClient for CountingClient {
    async fn unary(
        &self,
        _method: &str,
        _payload: Value,
    ) -> Result<xylitol_protocol::RpcResult, HostClientError> {
        self.unaries.fetch_add(1, Ordering::SeqCst);
        Ok(xylitol_protocol::RpcResult::error(
            "not_expected",
            "clipboard MUST NOT unary the host",
        ))
    }

    async fn respond(&self, _rpc_id: &str, _payload: Value) -> Result<(), HostClientError> {
        Ok(())
    }

    async fn mux(&self) -> Result<MuxStream, HostClientError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

/// Empty PATH so `plan_clipboard_copy` cannot spawn wl-copy/xclip/pbcopy.
/// Restored on drop. `env_global` covers the cargo-test in-process fallback
/// (nextest is already process-per-test).
struct StarveNativeClipboard {
    saved_path: Option<std::ffi::OsString>,
}

impl StarveNativeClipboard {
    fn enter() -> Self {
        let saved_path = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "");
        }
        Self { saved_path }
    }
}

impl Drop for StarveNativeClipboard {
    fn drop(&mut self) {
        unsafe {
            match self.saved_path.take() {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}

// env_global: this test clears PATH so native clipboard tools are not
// spawned (would clobber the developer clipboard). Elimination path:
// inject get_env into Driver clipboard helpers.
#[tokio::test]
#[serial_test::serial(env_global)]
async fn clipboard_ops_stay_client_local() {
    let _starve = StarveNativeClipboard::enter();
    let client = CountingClient::default();
    let mut driver = XyRemoteDriver::with_host(client.clone(), "s");

    let outcome = driver
        .copy_text_to_clipboard("copy-me")
        .await
        .expect("OSC52 fallback MUST make copy succeed without native tools");
    assert!(
        outcome.pending_osc52.is_some(),
        "empty PATH MUST take OSC52 fallback, not native copy"
    );
    let _ = driver.stage_clipboard_image().await;
    let _ = driver.read_clipboard_text().await;
    assert_eq!(
        client.unaries.load(Ordering::SeqCst),
        0,
        "clipboard ops MUST NOT unary the host (D1 client-local)"
    );
}

#[tokio::test]
async fn push_bash_output_is_not_journaled() {
    let host = host_state_for_test().expect("host");
    let slot = host.slot("bash-ev").await;
    let mut rx = host.in_process_downlink.subscribe();
    let seq_before = slot.journal.lock().await.max_seq();
    slot.push_bash_output(xylitol_plugin_sdk::BashChunk {
        bash_id: "b1".into(),
        seq: 1,
        data: "hello".into(),
    })
    .await;
    let msg = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("downlink")
        .expect("msg");
    match msg {
        RpcMessage::ServerRequest {
            method, payload, ..
        } => {
            assert_eq!(method, "session/bash_output");
            assert_eq!(payload["bash_id"], "b1");
            assert_eq!(payload["data"], "hello");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        slot.journal.lock().await.max_seq(),
        seq_before,
        "session/bash_output MUST NOT consume journal seq (c2760)"
    );
}

#[tokio::test]
async fn abort_cancels_running_bash_out_of_band() {
    use handle_unary;
    let host = host_state_for_test().expect("host");
    let slot = host.slot("abort-bash").await;
    let cancel = tokio_util::sync::CancellationToken::new();
    *slot.bash_run_cancel.lock().unwrap() = Some(cancel.clone());
    let result = handle_unary(
        &host,
        None,
        "abort",
        serde_json::json!({ "session_id": "abort-bash" }),
    )
    .await;
    assert_eq!(
        result.value.as_ref().and_then(|v| v["cancelled"].as_bool()),
        Some(true)
    );
    assert!(
        cancel.is_cancelled(),
        "abort must cancel the registered bash run"
    );
}

#[tokio::test]
async fn abort_without_running_bash_keeps_fallback_path() {
    use handle_unary;
    let host = host_state_for_test().expect("host");
    let slot = host.slot("idle-abort").await;
    let cancel = tokio_util::sync::CancellationToken::new();
    *slot.bash_run_cancel.lock().unwrap() = Some(cancel.clone());
    // A different session id must not reach this slot's registration.
    let result = handle_unary(
        &host,
        None,
        "abort",
        serde_json::json!({ "session_id": "other-session" }),
    )
    .await;
    assert!(
        !cancel.is_cancelled(),
        "foreign abort must not cancel our run"
    );
    // No registration on the target slot: falls through to the plain
    // `Command::Abort` dispatch (for_test materializes a writer), which
    // must still report a cancelled run.
    assert_eq!(
        result.value.as_ref().and_then(|v| v["cancelled"].as_bool()),
        Some(true)
    );
}

#[tokio::test]
async fn push_resources_is_not_journaled() {
    let host = host_state_for_test().expect("host");
    let slot = host.slot("res").await;
    let mut rx = host.in_process_downlink.subscribe();
    slot.push_resources(LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_connecting_label: Some("connecting 0/1".into()),
        ..LoadedResourcesSnapshot::default()
    })
    .await;
    let msg = tokio::time::timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("downlink")
        .expect("msg");
    match msg {
        RpcMessage::ServerRequest {
            method, payload, ..
        } => {
            assert_eq!(method, "session/resources");
            assert_eq!(payload["session_id"], "res");
            assert_eq!(payload["snapshot"]["mcp_configured"], 1);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        slot.journal.lock().await.max_seq(),
        0,
        "session/resources MUST NOT consume journal seq"
    );
}

/// c2850/r1926: attach 期 ModelSelect 同步不进 journal、不消耗 seq；
/// 重复调用仍向订阅者推送（语义幂等）。
#[tokio::test]
async fn sync_model_downlink_is_not_journaled() {
    use materialize_writer;

    let host = host_state_for_test_with_default_model("fake-model").expect("host");
    let slot = host.slot("model-sync").await;
    materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let seq_before = slot.journal.lock().await.max_seq();
    let mut rx = host.in_process_downlink.subscribe();
    slot.sync_model_downlink().await;
    slot.sync_model_downlink().await;

    let mut seen = 0u8;
    for _ in 0..16 {
        if seen >= 2 {
            break;
        }
        let msg = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("downlink")
            .expect("msg");
        let RpcMessage::ServerRequest {
            method, payload, ..
        } = msg
        else {
            continue;
        };
        if method != "session/event" {
            continue;
        }
        let ev = payload.get("event").cloned().unwrap_or_default();
        if ev.get("type").and_then(|v| v.as_str()) == Some("model_select") {
            assert_eq!(
                ev.get("model_id").and_then(|v| v.as_str()),
                Some("fake-model")
            );
            seen += 1;
        }
    }
    assert_eq!(seen, 2, "两次 sync MUST 各推一帧 ModelSelect");
    assert_eq!(
        slot.journal.lock().await.max_seq(),
        seq_before,
        "attach ModelSelect MUST NOT consume journal seq"
    );
}

#[tokio::test]
async fn minted_session_subscribe_does_not_require_cli_flag() {
    let host = host_state_for_test().expect("host");
    let (running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host,
    )
    .await
    .expect("bind");
    let driver = XyRemoteDriver::new(format!("http://127.0.0.1:{port}"), "");
    let result = driver
        .unary("subscribe", serde_json::json!({ "last_seq": 0 }))
        .await;
    running.shutdown();
    result.expect("subscribe must accept the minted session_id");
}

async fn exercise_session_and_host_capabilities<C>(mut driver: XyRemoteDriver<C>)
where
    C: HostClient + Clone + 'static,
{
    use xylitol_app_tui::dispatch::{DispatchOutcome, dispatch};
    use xylitol_protocol::Command;

    let session_id = match dispatch(&mut driver, Command::NewSession {})
        .await
        .expect("new session")
    {
        DispatchOutcome::NewSession(id) => id,
        other => panic!("new session outcome: {other:?}"),
    };
    let listed = match dispatch(&mut driver, Command::ListSessions {})
        .await
        .expect("list sessions")
    {
        DispatchOutcome::Sessions(list) => list,
        other => panic!("list sessions outcome: {other:?}"),
    };
    assert!(listed.iter().any(|entry| entry.id == session_id));

    let stored_name = match dispatch(
        &mut driver,
        Command::SetSessionName {
            name: " remote name\n".into(),
        },
    )
    .await
    .expect("set name")
    {
        DispatchOutcome::SessionName(Some(name)) => name,
        other => panic!("set name outcome: {other:?}"),
    };
    assert_eq!(stored_name, "remote name");
    let fetched = match dispatch(&mut driver, Command::GetSessionName {})
        .await
        .expect("get name")
    {
        DispatchOutcome::SessionName(name) => name,
        other => panic!("get name outcome: {other:?}"),
    };
    assert_eq!(fetched.as_deref(), Some("remote name"));

    let entries = match dispatch(
        &mut driver,
        Command::LoadSessionEntries {
            session_id: session_id.clone(),
        },
    )
    .await
    .expect("load entries")
    {
        DispatchOutcome::SessionEntries(entries) => entries,
        other => panic!("load entries outcome: {other:?}"),
    };
    assert_eq!(entries.len(), 2);

    let tree = match dispatch(
        &mut driver,
        Command::SessionTree {
            kind: SessionTreeKind::MessageHistory,
        },
    )
    .await
    .expect("session tree")
    {
        DispatchOutcome::SessionTree(tree) => tree,
        other => panic!("session tree outcome: {other:?}"),
    };
    driver
        .refresh_surface_caches()
        .await
        .expect("refresh fixed zone caches");
    let _ = driver.current_model();
    assert!(driver.leaf_entry_id().is_some());
    let target_id = tree
        .first()
        .and_then(|node| node.entry.entry_id())
        .map(str::to_string)
        .expect("session tree entry");
    dispatch(
        &mut driver,
        Command::AppendEntryLabel {
            target_id: target_id.clone(),
            label: Some("important".into()),
        },
    )
    .await
    .expect("append label");
    let labelled_tree = match dispatch(
        &mut driver,
        Command::SessionTree {
            kind: SessionTreeKind::MessageHistory,
        },
    )
    .await
    .expect("labelled tree")
    {
        DispatchOutcome::SessionTree(tree) => tree,
        other => panic!("labelled tree outcome: {other:?}"),
    };
    assert_eq!(
        labelled_tree.first().and_then(|node| node.label.as_deref()),
        Some("important")
    );
    let travel = match dispatch(
        &mut driver,
        Command::TravelSessionTree {
            kind: SessionTreeKind::MessageHistory,
            entry_id: target_id.clone(),
        },
    )
    .await
    .expect("travel")
    {
        DispatchOutcome::SessionTreeTravel(travel) => travel,
        other => panic!("travel outcome: {other:?}"),
    };
    assert_eq!(travel.selected_id, target_id);
    let named = match dispatch(
        &mut driver,
        Command::SetSessionNameFor {
            session_id: session_id.clone(),
            name: "named for session".into(),
        },
    )
    .await
    .expect("set name for")
    {
        DispatchOutcome::SessionName(Some(name)) => name,
        other => panic!("set name for outcome: {other:?}"),
    };
    assert_eq!(named, "named for session");
    let fetched = match dispatch(&mut driver, Command::GetSessionName {})
        .await
        .expect("get name")
    {
        DispatchOutcome::SessionName(name) => name,
        other => panic!("get name outcome: {other:?}"),
    };
    assert_eq!(fetched.as_deref(), Some("named for session"));

    let snapshot = driver.loaded_resources_snapshot().await;
    assert!(snapshot.mcp_diag_short.is_empty());
    driver.arm_tool_freeze_gate().await;
    assert!(
        driver.is_tools_frozen(),
        "arm_tool_freeze MUST freeze when no MCP is configured"
    );
    dispatch(
        &mut driver,
        Command::Steer {
            message: "nudge".into(),
        },
    )
    .await
    .expect("steer");
    assert_eq!(driver.queue_stats().steer_count, 1);
    let queued = driver
        .unary("queue_stats", serde_json::json!({}))
        .await
        .expect("queue_stats unary");
    assert_eq!(queued.get("steer_count").and_then(Value::as_u64), Some(1));
    dispatch(
        &mut driver,
        Command::ClearQueue {
            clear_steer: true,
            clear_follow_up: true,
        },
    )
    .await
    .expect("clear queue");
    let report = driver
        .reload_runtime(&CancellationToken::new())
        .await
        .expect("reload");
    assert!(!report.steps.is_empty());
    dispatch(
        &mut driver,
        Command::DeleteSession {
            session_id: session_id.clone(),
        },
    )
    .await
    .expect("delete session");
    let listed = match dispatch(&mut driver, Command::ListSessions {})
        .await
        .expect("list after delete")
    {
        DispatchOutcome::Sessions(list) => list,
        other => panic!("list after delete outcome: {other:?}"),
    };
    assert!(!listed.iter().any(|entry| entry.id == session_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_forwards_bash_output_frames_to_sink() {
    let host = host_state_for_test().expect("host");
    let (running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host.clone(),
    )
    .await
    .expect("bind");
    let mut driver = XyRemoteDriver::new(format!("http://127.0.0.1:{port}"), "sink-sess");
    driver.attach_session().await.expect("attach");
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    driver.set_bash_run_sink(Some(xylitol_plugin_sdk::BashOutputSink {
        tx,
        cancel: tokio_util::sync::CancellationToken::new(),
    }));
    host.slot("sink-sess")
        .await
        .push_bash_output(xylitol_plugin_sdk::BashChunk {
            bash_id: "b1".into(),
            seq: 1,
            data: "streamed".into(),
        })
        .await;
    let chunk = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("chunk delivery")
        .expect("chunk");
    assert_eq!(chunk.bash_id, "b1");
    assert_eq!(chunk.data, "streamed");
    running.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_and_host_capabilities_roundtrip_over_host() {
    let host = host_state_for_test().expect("host");
    let (running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host,
    )
    .await
    .expect("bind");
    exercise_session_and_host_capabilities(XyRemoteDriver::new(
        format!("http://127.0.0.1:{port}"),
        "remote-seed",
    ))
    .await;
    running.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_and_host_capabilities_match_in_process_carrier() {
    let host = host_state_for_test().expect("host");
    let client = in_process_client(host);
    exercise_session_and_host_capabilities(XyRemoteDriver::with_host(client, "in-process-seed"))
        .await;
}

fn fixture_mcp(name: &str) -> crate::wiring::mcp_spec::McpServerSpec {
    use crate::wiring::mcp_spec::{McpServerSpec, McpTransportSpec};
    use std::collections::HashMap;

    let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/mcp_fixture_server.py");
    let mut env = HashMap::new();
    env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), "ping".into());
    McpServerSpec {
        name: name.into(),
        transport: McpTransportSpec::Stdio,
        command: Some("python3".into()),
        args: Some(vec![script.display().to_string()]),
        url: None,
        env: Some(env),
        headers: None,
    }
}

#[tokio::test]
async fn loaded_resources_reads_writer_mcp_not_hollow_shell() {
    use materialize_writer;

    let host =
        host_state_for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
    let slot = host.slot("mcp-sess").await;
    materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    {
        let mut guard = slot.driver.lock().await;
        let driver = guard.as_mut().expect("writer");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            let _ = driver.poll_mcp_bootstrap().await;
            if !driver.mcp_blocks_agent() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
    let snap = host.loaded_resources_snapshot().await;
    assert_eq!(snap.mcp_configured, 2);
    assert!(
        !snap.mcp_connected.is_empty()
            || !snap.mcp_diag_short.is_empty()
            || snap.mcp_connecting_label.is_some(),
        "loaded_resources MUST NOT stay at configured-only hollow shell: {snap:?}"
    );
    if snap.mcp_bootstrap_complete {
        assert!(
            !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
            "settled snapshot MUST NOT be fake-complete 0 connected: {snap:?}"
        );
    }
}

#[tokio::test]
async fn loaded_resources_snapshot_applies_writer_mcp_without_direct_poll() {
    use materialize_writer;

    let host =
        host_state_for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
    let slot = host.slot("mcp-attach").await;
    materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    // Attach TUI only hits `loaded_resources` unary — never `XyDriver::poll_mcp`
    // on the writer. Snapshot MUST join/install so the header can leave
    // `2 configured · 0 connected`.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut snap = host.loaded_resources_snapshot().await;
    while std::time::Instant::now() < deadline {
        if !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        snap = host.loaded_resources_snapshot().await;
    }
    assert_eq!(snap.mcp_configured, 2);
    assert!(
        !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
        "attach-path snapshot MUST apply MCP without a direct writer poll: {snap:?}"
    );
    if snap.mcp_bootstrap_complete {
        assert!(
            !snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty(),
            "settled attach snapshot MUST NOT be 0 connected: {snap:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resources_watch_pushes_one_frame_on_snapshot_change() {
    use materialize_writer;

    // r1790：写者侧 poll MCP bootstrap，快照变化才向订阅连接广播
    // `session/resources` notification（bootstrap 过程可有多次变化、
    // 各推一帧）；payload 与 loaded_resources unary 同形；不消耗
    // journal seq；快照稳定后不重复推帧。
    let host = host_state_for_test_with_mcp(vec![fixture_mcp("a")]).expect("host");
    let slot = host.slot("res-watch").await;
    materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let mut rx = host.in_process_downlink.subscribe();
    slot.ensure_mcp_resources_watch();

    // 等 bootstrap 落定（与近邻 MCP 测试同一判据）。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut snap = host.loaded_resources_snapshot_for("res-watch").await;
    while std::time::Instant::now() < deadline {
        if snap.mcp_bootstrap_complete
            && (!snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty())
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        snap = host.loaded_resources_snapshot_for("res-watch").await;
    }
    assert!(
        snap.mcp_bootstrap_complete,
        "fixture MCP bootstrap must settle: {snap:?}"
    );
    // 非阻塞排空 + 轮询：等到出现「与已落定快照同形」那一帧（相对判据，
    // 不靠固定 sleep 猜 poll 间隔；收集到的帧仍逐帧可查）。
    let mut frames: Vec<Value> = Vec::new();
    // 帧与「unary 同形」比较须剥离 settle 探测字段：mcp_bootstrap_complete
    // 属 host 装配状态（测试 settle 判据），非 resources 帧语义——其翻转
    // 不一定伴随推帧（产品契约：仅资源面变化才推）。其余字段必须逐字一致。
    let strip = |v: &Value| -> Value {
        let mut m = v.as_object().cloned().unwrap_or_default();
        m.remove("mcp_bootstrap_complete");
        Value::Object(m)
    };
    // 计时只在「曾收到帧」后启用：冷启动首帧可能晚于 snap 落定，若在
    // 首帧前就按 5 秒收口会误判。收到帧后无进展才按窗口收口；
    // 另设 30 秒绝对保险，防产品 watch 异常导致测试挂死。
    let mut drain_deadline: Option<std::time::Instant> = None;
    let hard_cap = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let before = frames.len();
        while let Ok(msg) = rx.try_recv() {
            if let RpcMessage::ServerRequest {
                method, payload, ..
            } = msg
                && method == "session/resources"
            {
                frames.push(payload);
            }
        }
        if std::time::Instant::now() >= hard_cap {
            break;
        }
        // 静默收口：从「曾收到帧」起静默 5s 视为稳定（产品契约：资源面稳定后
        // 不重复推帧）。mcp_bootstrap_complete 为 settle 探测字段、非资源面，
        // 不要求其翻转伴随推帧——故后续「资源面一致帧」断言在其剔除后判断。
        if frames.len() != before {
            drain_deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
        } else if let Some(dl) = drain_deadline
            && std::time::Instant::now() >= dl
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let final_snap = serde_json::to_value(host.loaded_resources_snapshot_for("res-watch").await)
        .unwrap_or(Value::Null);
    let resource_stable_matched = frames
        .iter()
        .any(|f| strip(&f["snapshot"]) == strip(&final_snap));
    assert!(
        resource_stable_matched,
        "存在一帧的资源面与最终 loaded_resources unary 一致（剔除 mcp_bootstrap_complete 逐字同形）——帧={frames:?} final={final_snap:?}"
    );
    assert!(
        !frames.is_empty(),
        "watch loop must push frames on snapshot change"
    );
    let last = frames.last().expect("frames nonempty");
    assert_eq!(
        last["session_id"], "res-watch",
        "frame MUST carry the session id"
    );
    assert_eq!(
        slot.journal.lock().await.max_seq(),
        0,
        "resources frames MUST NOT consume journal seq"
    );

    // 快照稳定后不再重复广播（仅变化时推帧；其他下行帧不算）。
    let quiet = tokio::time::timeout(std::time::Duration::from_millis(700), rx.recv()).await;
    if let Ok(Ok(RpcMessage::ServerRequest { method, .. })) = quiet {
        assert_ne!(
            method, "session/resources",
            "stable snapshot MUST NOT repeat the resources frame"
        );
    }
}

#[tokio::test]
async fn arm_tool_freeze_unary_freezes_after_mcp_settle() {
    use materialize_writer;

    let host =
        host_state_for_test_with_mcp(vec![fixture_mcp("a"), fixture_mcp("b")]).expect("host");
    let slot = host.slot("mcp-freeze").await;
    materialize_writer(&host, &slot)
        .await
        .expect("materialize writer");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut snap = host.loaded_resources_snapshot().await;
    while std::time::Instant::now() < deadline {
        if snap.mcp_bootstrap_complete
            && (!snap.mcp_connected.is_empty() || !snap.mcp_diag_short.is_empty())
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        snap = host.loaded_resources_snapshot().await;
    }
    assert!(
        !snap.tools_table_frozen,
        "settle MUST NOT freeze until first-turn gate: {snap:?}"
    );
    let result = handle_unary(
        &host,
        None,
        "arm_tool_freeze",
        serde_json::json!({ "session_id": "mcp-freeze" }),
    )
    .await;
    assert!(result.ok, "arm_tool_freeze unary failed: {result:?}");
    let snap: LoadedResourcesSnapshot =
        serde_json::from_value(result.value.expect("snapshot")).expect("decode snapshot");
    assert!(
        snap.tools_table_frozen,
        "first-turn arm MUST freeze after MCP settle: {snap:?}"
    );
}

#[tokio::test]
async fn loaded_resources_before_writer_is_not_fake_complete() {
    let host = host_state_for_test_with_mcp(vec![fixture_mcp("pre")]).expect("host");
    let snap = host.loaded_resources_snapshot().await;
    assert_eq!(snap.mcp_configured, 1);
    assert!(
        !snap.mcp_bootstrap_complete
            || snap.mcp_connecting_label.is_some()
            || !snap.mcp_connected.is_empty()
            || !snap.mcp_diag_short.is_empty(),
        "pre-subscribe snapshot MUST NOT be Idle-complete 0 connected: {snap:?}"
    );
}

/// c2849 / c2893：`arm_tool_freeze` 不占写锁，已有写引擎时仍须返回冻结快照。
#[tokio::test]
async fn arm_tool_freeze_conflict_falls_back_lease_free_not_stuck() {
    use materialize_writer;

    let host = host_state_for_test_with_mcp(vec![fixture_mcp("a")]).expect("host");
    let slot = host.slot("conflict-sess").await;
    materialize_writer(&host, &slot).await.expect("materialize");
    // 由另一写者建立租约：先以一个 token 调用一次写 unary（set_model 走租约）。
    let first = handle_unary(
        &host,
        None,
        "set_model",
        serde_json::json!({ "provider": "", "model_id": "keep", "session_id": "conflict-sess" }),
    )
    .await;
    let _ = first;
    let result = handle_unary(
        &host,
        None,
        "arm_tool_freeze",
        serde_json::json!({ "session_id": "conflict-sess" }),
    )
    .await;
    assert!(
        result.ok,
        "arm_tool_freeze MUST succeed without taking occupancy: {result:?}"
    );
    let snap: LoadedResourcesSnapshot =
        serde_json::from_value(result.value.expect("snapshot")).expect("decode snapshot");
    assert!(
        snap.tools_table_frozen,
        "lease-free arm MUST still resolve frozen: {snap:?}"
    );
}

#[tokio::test]
async fn arm_tool_freeze_hangs_mcp_returns_frozen_within_gate_window() {
    // c2847 回归：配置了但永远连不上的 MCP，`arm_tool_freeze` unary MUST NOT
    // 无限等待——`ensure_tool_table_frozen` 在门时限内 detach 并冻结 armed 子集，
    // 返回 frozen+complete 快照；否则 TUI 的 Assembling 门死锁（旧行为：立即
    // 返回未冻结快照，客户端无限等待）。
    use crate::wiring::mcp_spec::{McpServerSpec, McpTransportSpec};
    use materialize_writer;

    let hang = McpServerSpec {
        name: "hang-forever".into(),
        transport: McpTransportSpec::Stdio,
        command: Some("sleep".into()),
        args: Some(vec!["300".into()]),
        url: None,
        env: None,
        headers: None,
    };
    let host = host_state_for_test_with_mcp(vec![hang]).expect("host");
    let slot = host.slot("hang-freeze").await;
    materialize_writer(&host, &slot).await.expect("materialize");

    let t0 = std::time::Instant::now();
    let result = handle_unary(
        &host,
        None,
        "arm_tool_freeze",
        serde_json::json!({ "session_id": "hang-freeze" }),
    )
    .await;
    let elapsed = t0.elapsed();
    assert_eq!(result.error.as_ref().map(|e| e.code.as_str()), None);
    assert!(
        result.ok,
        "arm_tool_freeze unary MUST resolve within gate window: {result:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "MUST be bounded by the first-turn gate window, took {elapsed:?}"
    );
    let snap: LoadedResourcesSnapshot =
        serde_json::from_value(result.value.expect("snapshot")).expect("decode snapshot");
    assert!(
        snap.tools_table_frozen,
        "timeout MUST freeze armed subset (deadlock guard): {snap:?}"
    );
    assert!(
        snap.mcp_bootstrap_complete,
        "timeout MUST settle bootstrap (no eternal connecting): {snap:?}"
    );
}

#[tokio::test]
async fn materialize_writer_does_not_wait_mcp_bootstrap() {
    use crate::wiring::mcp_spec::{McpServerSpec, McpTransportSpec};
    use materialize_writer;

    let hang = McpServerSpec {
        name: "hang".into(),
        transport: McpTransportSpec::Stdio,
        command: Some("sleep".into()),
        args: Some(vec!["30".into()]),
        url: None,
        env: None,
        headers: None,
    };
    let host = host_state_for_test_with_mcp(vec![hang]).expect("host");
    let slot = host.slot("hang-sess").await;
    let t0 = std::time::Instant::now();
    materialize_writer(&host, &slot).await.expect("materialize");
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(2),
        "writer unary MUST NOT wait MCP connect: {:?}",
        t0.elapsed()
    );
    let t1 = std::time::Instant::now();
    let result = handle_unary(
        &host,
        None,
        "set_model",
        serde_json::json!({
            "provider": "",
            "model_id": "missing",
            "session_id": "hang-sess",
        }),
    )
    .await;
    assert!(
        t1.elapsed() < std::time::Duration::from_secs(2),
        "set_model MUST NOT wait MCP: {:?} result={result:?}",
        t1.elapsed()
    );
}

#[tokio::test]
async fn attach_mux_survives_after_idle_and_receives_later_events() {
    use xylitol_protocol::Event;

    let host = host_state_for_test().expect("host");
    let client = in_process_client(host.clone());
    let mut driver = XyRemoteDriver::with_host(client, "mux-sess");
    driver.attach_session().await.expect("attach");
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let slot = host.slot("mux-sess").await;
    slot.append_and_push(Event::QueueUpdate {
        steer_count: 1,
        follow_up_count: 0,
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let first = driver.drain_idle_events();
    assert!(
        first
            .iter()
            .any(|e| matches!(e, XyEvent::QueueUpdate { steer_count: 1, .. })),
        "first downlink event missing: {first:?}"
    );
    slot.append_and_push(Event::AgentEnd).await;
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    let _ = driver.drain_idle_events();
    slot.append_and_push(Event::QueueUpdate {
        steer_count: 0,
        follow_up_count: 1,
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let later = driver.drain_idle_events();
    assert!(
        later.iter().any(|e| matches!(
            e,
            XyEvent::QueueUpdate {
                follow_up_count: 1,
                ..
            }
        )),
        "mux MUST still deliver after AgentEnd: {later:?}"
    );
}

#[test]
fn cold_replay_tape_drops_agent_keeps_queue() {
    assert!(XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
        &XyEvent::AgentStart {
            session_id: "s".into(),
            model: "m".into(),
        }
    ));
    assert!(XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
        &XyEvent::TextDelta("hi".into())
    ));
    assert!(!XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
        &XyEvent::QueueUpdate {
            steer_count: 1,
            follow_up_count: 0,
        }
    ));
    assert!(!XyRemoteDriver::<InProcessClient>::is_cold_replay_tape(
        &XyEvent::error_msg("boom")
    ));
}

#[tokio::test]
async fn cold_subscribe_drops_replay_tape_and_stays_live() {
    use xylitol_protocol::Event;

    let host = host_state_for_test().expect("host");
    {
        let slot = host.slot("s-cold").await;
        let mut j = slot.journal.lock().await;
        j.append(Event::TextDelta {
            text: "tape-1".into(),
        });
        j.append(Event::TextDelta {
            text: "tape-2".into(),
        });
    }
    let client = in_process_client(host.clone());
    let mut driver = XyRemoteDriver::with_host(client, "s-cold");
    driver.attach_session().await.expect("attach");
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let replay = driver.drain_idle_events();
    assert!(
        !replay
            .iter()
            .any(|e| matches!(e, XyEvent::TextDelta(t) if t.contains("tape"))),
        "cold-replay tape MUST NOT reach the transcript stream (ath36): {replay:?}"
    );
    let slot = host.slot("s-cold").await;
    slot.append_and_push(Event::QueueUpdate {
        steer_count: 0,
        follow_up_count: 2,
    })
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let live = driver.drain_idle_events();
    assert!(
        live.iter().any(|e| matches!(
            e,
            XyEvent::QueueUpdate {
                follow_up_count: 2,
                ..
            }
        )),
        "stream MUST stay live after the recovery window: {live:?}"
    );
}

#[tokio::test]
async fn switch_session_resubscribes_cold_and_drops_new_session_tape() {
    // resume-render 修复: SwitchSession 用旧 session 的 last_seq 重订阅新 session 时，
    // journal 从任意偏移重放，mid-message TextDelta 尾巴会涌进 transcript。
    // 重订阅必须 cold（last_seq=0 → skip_cold_replay 武装）。
    use xylitol_app_tui::dispatch::{DispatchOutcome, dispatch};
    use xylitol_protocol::{Command, Event};

    let host = host_state_for_test().expect("host");
    host.ports
        .store
        .create("s-target", None, None)
        .await
        .expect("seed target session");
    {
        let slot = host.slot("s-target").await;
        let mut j = slot.journal.lock().await;
        j.append(Event::TextDelta {
            text: "我是你的编码助手，".into(),
        });
        j.append(Event::TextDelta {
            text: "主要帮你在这个 `xylitol` 项目里干活。".into(),
        });
    }
    let client = in_process_client(host.clone());
    let mut driver = XyRemoteDriver::with_host(client, "s-other");
    driver.attach_session().await.expect("attach");
    // 在旧 session 上积累非零 last_seq（模拟看过一轮直播）。
    {
        let slot = host.slot("s-other").await;
        slot.append_and_push(Event::QueueUpdate {
            steer_count: 0,
            follow_up_count: 0,
        })
        .await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    let _ = driver.drain_idle_events();

    let outcome = dispatch(
        &mut driver,
        Command::SwitchSession {
            session_path: "s-target".into(),
        },
    )
    .await
    .expect("switch");
    assert!(matches!(outcome, DispatchOutcome::SwitchedSession(_)));
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let evs = driver.drain_idle_events();
    assert!(
        !evs.iter().any(|e| matches!(
            e,
            XyEvent::TextDelta(t) if t.contains("xylitol")
        )),
        "switch 后旧偏移的 journal 重放不得作为直播尾巴到达 transcript: {evs:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_after_fork_switch_delivers_live_text() {
    use futures::StreamExt;
    use xylitol_app_tui::dispatch::{DispatchOutcome, dispatch};
    use xylitol_protocol::Command;

    let host = host_state_for_test_with_default_model("fake-model").expect("host");
    let client = in_process_client(host.clone());
    let mut driver = XyRemoteDriver::with_host(client, "s-fork-live");
    driver.attach_session().await.expect("attach");

    let mut stream = driver.run("main branch").await;
    let mut saw_hello = false;
    while let Some(ev) = stream.next().await {
        if matches!(&ev, XyEvent::TextDelta(t) if t.contains("Hello from fake")) {
            saw_hello = true;
        }
        if matches!(ev, XyEvent::AgentEnd { .. }) {
            break;
        }
    }
    assert!(saw_hello, "trunk Fake reply MUST arrive");

    let leaf = driver.leaf_entry_id().expect("leaf after trunk");
    let child = match dispatch(
        &mut driver,
        Command::Fork {
            entry_id: leaf,
            position: Some("at".into()),
        },
    )
    .await
    .expect("fork")
    {
        DispatchOutcome::NewSession(id) => id,
        other => panic!("fork outcome: {other:?}"),
    };
    dispatch(
        &mut driver,
        Command::SwitchSession {
            session_path: child,
        },
    )
    .await
    .expect("switch");

    let mut stream = driver.run("alt branch").await;
    let mut saw_hello = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while let Ok(Some(ev)) = tokio::time::timeout(
        deadline.saturating_duration_since(tokio::time::Instant::now()),
        stream.next(),
    )
    .await
    {
        if matches!(&ev, XyEvent::TextDelta(t) if t.contains("Hello from fake")) {
            saw_hello = true;
        }
        if matches!(ev, XyEvent::AgentEnd { .. }) {
            break;
        }
    }
    assert!(
        saw_hello,
        "forked child prompt MUST deliver live Fake text, not hang"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prompt_after_fork_switch_delivers_live_text_http() {
    use futures::StreamExt;
    use xylitol_app_tui::dispatch::{DispatchOutcome, dispatch};
    use xylitol_protocol::Command;

    let host = host_state_for_test_with_default_model("fake-model").expect("host");
    let (running, port) = serve(
        ServerConfig {
            host: "127.0.0.1".into(),
            port: 0,
            sessions_dir: None,
            registration_path: None,
        },
        host,
    )
    .await
    .expect("bind");
    let mut driver = XyRemoteDriver::new(format!("http://127.0.0.1:{port}"), "s-fork-http");
    let attach = driver.attach_session().await;
    if let Err(e) = attach {
        running.shutdown();
        panic!("attach: {e}");
    }

    let mut stream = driver.run("main branch").await;
    let mut saw_hello = false;
    while let Some(ev) = stream.next().await {
        if matches!(&ev, XyEvent::TextDelta(t) if t.contains("Hello from fake")) {
            saw_hello = true;
        }
        if matches!(ev, XyEvent::AgentEnd { .. }) {
            break;
        }
    }
    if !saw_hello {
        running.shutdown();
        panic!("trunk Fake reply MUST arrive over HTTP");
    }

    let leaf = match driver.leaf_entry_id() {
        Some(leaf) => leaf,
        None => {
            running.shutdown();
            panic!("leaf after trunk");
        }
    };
    let child = match dispatch(
        &mut driver,
        Command::Fork {
            entry_id: leaf,
            position: Some("at".into()),
        },
    )
    .await
    {
        Ok(DispatchOutcome::NewSession(id)) => id,
        other => {
            running.shutdown();
            panic!("fork outcome: {other:?}");
        }
    };
    if let Err(e) = dispatch(
        &mut driver,
        Command::SwitchSession {
            session_path: child,
        },
    )
    .await
    {
        running.shutdown();
        panic!("switch: {e}");
    }

    let mut stream = driver.run("alt branch").await;
    let mut saw_hello = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while let Ok(Some(ev)) = tokio::time::timeout(
        deadline.saturating_duration_since(tokio::time::Instant::now()),
        stream.next(),
    )
    .await
    {
        if matches!(&ev, XyEvent::TextDelta(t) if t.contains("Hello from fake")) {
            saw_hello = true;
        }
        if matches!(ev, XyEvent::AgentEnd { .. }) {
            break;
        }
    }
    running.shutdown();
    assert!(
        saw_hello,
        "HTTP forked child prompt MUST deliver live Fake text, not hang"
    );
}

#[tokio::test]
async fn materialize_writer_binds_client_workspace_cwd() {
    use materialize_writer_at;

    let host = host_state_for_test().expect("host");
    let slot = host.slot("ws-cwd").await;
    let dir = tempfile::tempdir().expect("tmp");
    materialize_writer_at(&host, &slot, dir.path())
        .await
        .expect("materialize");
    let guard = slot.driver.lock().await;
    let cwd = guard.as_ref().expect("writer").agent_cwd_for_test();
    assert_eq!(
        std::path::Path::new(&cwd),
        dir.path(),
        "writer cwd MUST be the TUI workspace, not serve cwd"
    );
}

#[tokio::test]
async fn bash_unary_runs_in_client_workspace() {
    use xylitol_serve::occupancy::{handle_unary, materialize_writer_at};

    let host = host_state_for_test().expect("host");
    let slot = host.slot("ws-bang").await;
    let dir = tempfile::tempdir().expect("tmp");
    materialize_writer_at(&host, &slot, dir.path())
        .await
        .expect("materialize");
    let result = handle_unary(
        &host,
        None,
        "bash",
        serde_json::json!({ "session_id": "ws-bang", "command": "pwd" }),
    )
    .await;
    assert!(result.ok, "bash unary MUST succeed: {result:?}");
    let output = result
        .value
        .as_ref()
        .and_then(|v| v.get("output"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let got = std::path::PathBuf::from(output.trim());
    assert_eq!(
        got.canonicalize().unwrap(),
        dir.path().canonicalize().unwrap(),
        "attach bang MUST run in the session workspace, not serve cwd; got: {output}"
    );
}

/// ws1 (c2335): the model-side tool execution surface MUST run in the
/// session workspace bound at writer materialization — a scripted fake
/// model emits a `bash pwd` tool call and the result MUST be the
/// workspace, not the serve process cwd.
#[tokio::test]
async fn model_tool_runs_in_client_workspace() {
    use crate::XyModelMeta;
    use crate::XySessionStore;
    use crate::wiring::composition::build_baseline_with_store;
    use crate::wiring::infra::provider::factory::{reset_fake_state, set_fake_tool_call};
    use futures::StreamExt;
    use std::path::PathBuf;
    use std::sync::Arc;
    use xylitol_plugin_sdk::lifecycle::XyEvent;
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};
    use xylitol_serve::occupancy::{ReloadBaseline, materialize_writer_at};

    reset_fake_state();
    set_fake_tool_call("bash", r#"{"command":"pwd"}"#);

    let dir = tempfile::tempdir().expect("tmp");
    let store: Arc<dyn XySessionStore> =
        Arc::new(xylitol_official_plugin_session::SessionManager::new(
            std::env::temp_dir()
                .join(format!("xylitol-host-ws-{}", uuid::Uuid::new_v4()))
                .join("sessions"),
        ));
    let mut registry = xylitol_runtime::agent::ModelRegistry::new();
    registry.register(XyModelMeta {
        id: "fake-ws".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Fake WS".into(),
        thinking: false,
        context_window: 200_000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: Vec::new(),
        thinking_level_map: Default::default(),
    });
    let options = crate::wiring::composition::BuildAgentOptions {
        model_registry: registry,
        ..Default::default()
    };
    let ports = build_baseline_with_store(options, store).expect("ports");
    let host = HostState::new(
        ports.clone(),
        ReloadBaseline {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            agent_dir: crate::wiring::infra::resource::DefaultResourceLoader::default_agent_dir(),
            project_trusted: true,
            default_model_id: Some("fake-ws".into()),
        },
        Arc::new(crate::wiring::occupancy::ProductOccupancyFactory::new(
            ports.clone(),
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            crate::wiring::infra::resource::DefaultResourceLoader::default_agent_dir(),
            true,
            Vec::new(),
            Some("fake-ws".into()),
        )),
        "ws-model-tool-fallback".into(),
    );

    let slot = host.slot("ws-model-tool").await;
    materialize_writer_at(&host, &slot, dir.path())
        .await
        .expect("materialize");

    let mut tool_end_result: Option<String> = None;
    {
        let mut guard = slot.driver.lock().await;
        let driver = guard.as_mut().expect("writer driver");
        driver.select_model("fake-ws").await.expect("select fake");
        let mut stream = driver.run("run pwd via tool").await;
        while let Some(event) = stream.next().await {
            if let XyEvent::ToolExecutionEnd { ref result, .. } = event {
                tool_end_result = Some(result.clone());
            }
            if matches!(event, XyEvent::AgentEnd { .. }) {
                break;
            }
        }
    }

    let result = tool_end_result.expect("bash tool must execute during the run");
    assert!(
        !result.contains("error"),
        "model-side bash tool MUST succeed; got {result}"
    );
    assert!(
        result.contains(
            &dir.path()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .to_string()
        ),
        "model-side bash tool MUST run in the session workspace; got {result}"
    );
}

#[tokio::test]
async fn persist_trust_unary_routes_to_writer() {
    use xylitol_serve::occupancy::{handle_unary, materialize_writer_at};

    let host = host_state_for_test().expect("host");
    let slot = host.slot("ws-trust").await;
    let dir = tempfile::tempdir().expect("tmp");
    materialize_writer_at(&host, &slot, dir.path())
        .await
        .expect("materialize");
    let result = handle_unary(
        &host,
        None,
        "persist_trust",
        serde_json::json!({ "session_id": "ws-trust", "mode": "trust_cwd" }),
    )
    .await;
    assert!(
        result.ok,
        "persist_trust MUST route to the writer: {result:?}"
    );
    let value = result.value.as_ref().expect("value");
    assert!(
        value.get("trusted").is_some() && value.get("message").is_some(),
        "report shape MUST surface trusted/message: {value}"
    );
}

#[tokio::test]
async fn export_unary_returns_content_and_cleans_stage() {
    use xylitol_serve::occupancy::{handle_unary, materialize_writer_at};

    let host = host_state_for_test().expect("host");
    let slot = host.slot("ws-export").await;
    let dir = tempfile::tempdir().expect("tmp");
    materialize_writer_at(&host, &slot, dir.path())
        .await
        .expect("materialize");
    let minted = handle_unary(
        &host,
        None,
        "new_session",
        serde_json::json!({ "session_id": "ws-export" }),
    )
    .await;
    assert!(minted.ok, "new_session MUST succeed: {minted:?}");
    let result = handle_unary(
        &host,
        None,
        "export_jsonl",
        serde_json::json!({ "session_id": "ws-export" }),
    )
    .await;
    assert!(result.ok, "export unary MUST succeed: {result:?}");
    let value = result.value.as_ref().expect("value");
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .expect("export response MUST carry content bytes for the TUI to write locally");
    assert!(!content.is_empty(), "jsonl export should have header rows");
    let staged = value
        .get("path")
        .and_then(Value::as_str)
        .map(std::path::PathBuf::from)
        .expect("staged path");
    assert!(
        !staged.exists(),
        "Host-side stage file MUST be cleaned up: {}",
        staged.display()
    );
}

#[tokio::test]
async fn inflight_set_thinking_does_not_apply_until_flush() {
    use xylitol_serve::occupancy::{handle_unary, materialize_writer};

    let host = host_state_for_test().expect("host");
    let slot = host.slot("th").await;
    materialize_writer(&host, &slot).await.expect("writer");
    let before = {
        let g = slot.driver.lock().await;
        g.as_ref().expect("driver").thinking_level()
    };
    slot.mark_run_inflight_for_test(true);
    let result = handle_unary(
        &host,
        None,
        "set_thinking_level",
        serde_json::json!({ "session_id": "th", "level": "high" }),
    )
    .await;
    assert!(result.ok, "defer thinking MUST succeed: {result:?}");
    let during = {
        let g = slot.driver.lock().await;
        g.as_ref().expect("driver").thinking_level()
    };
    assert_eq!(
        during, before,
        "busy thinking MUST NOT retune the in-flight run"
    );
    slot.mark_run_inflight_for_test(false);
    slot.flush_pending_runtime_for_test().await;
}
