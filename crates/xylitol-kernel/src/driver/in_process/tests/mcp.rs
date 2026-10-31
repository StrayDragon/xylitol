use super::*;
use crate::mcp_spec::{McpServerSpec, McpTransportSpec};

fn attach_mcp(driver: &mut XyInProcessDriver, specs: Vec<McpServerSpec>) {
    let servers = McpServerSpec::to_infra_list(&specs);
    driver.set_prepare_hub(std::sync::Arc::new(
        xylitol_official_plugin_mcp::McpPrepareHub::new(servers),
    ));
}

fn bad_mcp(name: &str) -> McpServerSpec {
    McpServerSpec {
        name: name.into(),
        transport: McpTransportSpec::Stdio,
        command: None,
        args: None,
        url: None,
        env: None,
        headers: None,
    }
}

fn mcp_progress_needs_ui_refresh(
    last_ui_label: &mut Option<Option<String>>,
    current: Option<String>,
) -> bool {
    if last_ui_label.as_ref() == Some(&current) {
        return false;
    }
    *last_ui_label = Some(current);
    true
}

#[tokio::test]

async fn begin_session_prepare_empty_settles_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    assert!(!driver.mcp_blocks_agent());
    driver.begin_session_prepare().await;
    assert!(
        !driver.mcp_blocks_agent(),
        "empty mcp_servers MUST settle without gating"
    );
    let snap = driver.loaded_resources_snapshot().await;
    assert!(snap.mcp_connecting_label.is_none());
    assert_eq!(snap.mcp_configured, 0);
}

#[tokio::test]
async fn poll_complete_armed_gate_freezes_tools_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    driver.begin_session_prepare().await;
    driver.arm_tool_freeze_gate().await;
    if !driver.is_tools_frozen() {
        let _ = driver.poll_prepare_progress().await;
    }
    assert!(
        driver.is_tools_frozen(),
        "complete + armed first-turn gate MUST freeze so Assembling can release"
    );
}

#[tokio::test]

async fn mcp_settle_defers_system_prompt_off_tick() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    attach_mcp(&mut driver, vec![bad_mcp("bad")]);
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    let prompt_before = driver.system_prompt_for_test();
    assert!(prompt_before.is_some());

    driver.begin_session_prepare().await;
    assert!(
        driver.mcp_blocks_agent(),
        "connecting must gate wait_session_prepare"
    );

    let mut saw_ungate_with_refresh = false;
    let mut saw_tools_while_gated = false;
    let deadline = std::time::Instant::now()
        + xylitol_official_plugin_mcp::MCP_SERVER_CONNECT_TIMEOUT
        + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let before = driver.mcp_blocks_agent();
        let refresh = driver.poll_prepare_progress().await;
        let after = driver.mcp_blocks_agent();
        if before
            && driver.tool_names_for_test().iter().any(|n| n == "read")
            && !driver
                .loaded_resources_snapshot()
                .await
                .mcp_bootstrap_complete
        {
            saw_tools_while_gated = true;
        }
        if before && !after {
            assert!(
                refresh,
                "leaving mcp_blocks_agent MUST return true (connecting→complete sticky cue)"
            );
            saw_ungate_with_refresh = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        saw_tools_while_gated,
        "must observe tools applied while still gated (connecting)"
    );
    assert!(
        saw_ungate_with_refresh,
        "must observe gated→ungated poll with refresh=true"
    );
    assert!(!driver.mcp_blocks_agent());
    let snap = driver.loaded_resources_snapshot().await;
    assert!(
        snap.mcp_bootstrap_complete,
        "after ungate, bootstrap MUST be complete"
    );
    assert!(
        !snap.mcp_tools_pending(),
        "settled+complete MUST clear mcp_tools_pending"
    );
    assert!(driver.system_prompt_for_test().is_some());
}

#[tokio::test]

async fn leaving_mcp_gate_must_signal_ui_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    attach_mcp(&mut driver, vec![bad_mcp("bad")]);
    driver.begin_session_prepare().await;
    assert!(driver.mcp_blocks_agent());

    let mut ok = false;
    let deadline = std::time::Instant::now()
        + xylitol_official_plugin_mcp::MCP_SERVER_CONNECT_TIMEOUT
        + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let before = driver.mcp_blocks_agent();
        let refresh = driver.poll_prepare_progress().await;
        let after = driver.mcp_blocks_agent();
        if before && !after {
            assert!(
                refresh,
                "connecting→complete (including failed complete) MUST signal UI refresh"
            );
            ok = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(ok, "gated→ungated transition not observed");
    let snap = driver.loaded_resources_snapshot().await;
    assert!(snap.mcp_bootstrap_complete);
    assert!(!snap.mcp_tools_pending());
}

#[test]
fn mcp_progress_needs_ui_refresh_only_on_label_change() {
    let mut last = None;
    assert!(mcp_progress_needs_ui_refresh(
        &mut last,
        Some("connecting 0/2".into())
    ));
    assert!(!mcp_progress_needs_ui_refresh(
        &mut last,
        Some("connecting 0/2".into())
    ));
    assert!(mcp_progress_needs_ui_refresh(
        &mut last,
        Some("connecting 1/2".into())
    ));
    assert!(!mcp_progress_needs_ui_refresh(
        &mut last,
        Some("connecting 1/2".into())
    ));
    assert!(mcp_progress_needs_ui_refresh(&mut last, None));
}

#[test]
fn mcp_tools_pending_ignores_failed_when_bootstrap_complete() {
    use crate::driver::{McpServerPhase, McpServerSnapshot};

    let snap = LoadedResourcesSnapshot {
        mcp_configured: 2,
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        mcp_servers: vec![
            McpServerSnapshot {
                id: "ok".into(),
                phase: McpServerPhase::Connected,
                tools_armed: true,
                tool_count: 1,
            },
            McpServerSnapshot {
                id: "bad".into(),
                phase: McpServerPhase::Failed,
                tools_armed: false,
                tool_count: 0,
            },
        ],
        ..Default::default()
    };
    assert!(!snap.mcp_tools_pending());
}

#[test]
fn mcp_tools_pending_while_connecting_label() {
    let snap = LoadedResourcesSnapshot {
        mcp_configured: 2,
        mcp_connecting_label: Some("connecting 1/2".into()),
        mcp_bootstrap_complete: false,
        ..Default::default()
    };
    assert!(snap.mcp_tools_pending());
}

#[test]
fn mcp_tools_pending_pre_freeze_while_incomplete_even_if_armed() {
    use crate::driver::{McpServerPhase, McpServerSnapshot};

    let snap = LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_bootstrap_complete: false,
        tools_table_frozen: false,
        mcp_servers: vec![McpServerSnapshot {
            id: "fs".into(),
            phase: McpServerPhase::Connected,
            tools_armed: true,
            tool_count: 3,
        }],
        ..Default::default()
    };
    assert!(
        snap.mcp_tools_pending(),
        "pre-freeze incomplete bootstrap MUST keep mcp pending (Assembling + cue)"
    );
}

#[test]
fn mcp_tools_pending_clears_after_freeze_when_complete() {
    use crate::driver::{McpServerPhase, McpServerSnapshot};

    let snap = LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        mcp_servers: vec![McpServerSnapshot {
            id: "fs".into(),
            phase: McpServerPhase::Connected,
            tools_armed: true,
            tool_count: 3,
        }],
        ..Default::default()
    };
    assert!(!snap.mcp_tools_pending());
}

#[tokio::test]

async fn ensure_freeze_on_empty_mcp_and_ignore_expand() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    assert!(!driver.is_tools_frozen());
    driver.begin_session_prepare().await;
    driver.ensure_tool_table_frozen().await;
    assert!(driver.is_tools_frozen());
    let names = driver.tool_names_for_test();
    assert!(names.iter().any(|n| n == "read"));
    let n = names.len();

    driver.set_tools(ToolSet::empty());
    assert_eq!(driver.tool_names_for_test().len(), n);
    assert!(driver.is_tools_frozen());
}

#[tokio::test]

async fn reload_re_freezes_tools() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    driver.ensure_tool_table_frozen().await;
    assert!(driver.is_tools_frozen());
    let report = driver
        .reload_runtime(&tokio_util::sync::CancellationToken::new())
        .await
        .expect("reload");
    assert!(
        !report.steps.is_empty(),
        "idle /reload must run registered participants"
    );
    assert!(
        !driver.is_tools_frozen(),
        "idle /reload fires prepare and must not wait/freeze"
    );
    driver.ensure_tool_table_frozen().await;
    assert!(driver.is_tools_frozen());
    assert!(driver.tool_names_for_test().iter().any(|n| n == "read"));
}

#[tokio::test]

async fn arm_tool_freeze_gate_empty_mcp_freezes_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let (mut driver, _obs) = build_test_driver(store).await;
    driver.enable_reload_state(dir.path().to_path_buf(), dir.path().join(".xylitol"), true);
    assert!(!driver.is_tools_frozen());
    driver.arm_tool_freeze_gate().await;
    assert!(driver.is_tools_frozen());
    let snap = driver.loaded_resources_snapshot().await;
    assert!(snap.mcp_connecting_label.is_none());
    assert!(snap.mcp_bootstrap_complete);
}

#[tokio::test]
async fn lab_provider_tools_field_follows_reload_freeze() {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::StreamExt;

    use xylitol_plugin_sdk::error::{XyError, XyToolError};
    use xylitol_plugin_sdk::message::XyStopReason;
    use xylitol_plugin_sdk::model::{XyChunk, XyModelConfig, XyModelMeta, XyToolSchema};
    use xylitol_plugin_sdk::{XyGenerateOptions, XyModel, XyStream, XyTool, XyToolCtx};

    struct StubMcpTool(&'static str);
    #[async_trait]
    impl XyTool for StubMcpTool {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            "stub"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        async fn execute(
            &self,
            _ctx: &XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, XyToolError> {
            Ok("ok".into())
        }
    }

    struct RecordingModel {
        seen: Arc<Mutex<Vec<Vec<String>>>>,
    }
    #[async_trait]
    impl XyModel for RecordingModel {
        fn name(&self) -> &str {
            "recording-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
            tools: &[XyToolSchema],
            _stream: bool,
            _options: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
            self.seen.lock().expect("seen").push(names);
            Ok(Box::pin(async_stream::stream! {
                yield Ok(XyChunk::TextDelta("hi".into()));
                yield Ok(XyChunk::Done {
                    finish_reason: XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_b = seen.clone();
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(SessionManager::new(dir.path().join("sessions")));
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut reg = xylitol_runtime::agent::model::registry::ModelRegistry::new();
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: XyModelConfig {
            kind: xylitol_plugin_sdk::model::XyModelKind::Fake,
            api_key: String::new(),
            model: "mock".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
        },
        display_name: "Mock".into(),
        thinking: false,
        context_window: 128000,
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
    let builder: ModelBuilderFn = Arc::new(move |_| {
        Arc::new(RecordingModel {
            seen: seen_b.clone(),
        }) as Arc<dyn XyModel>
    });
    let mut agent = AgentBuilder::new(
        reg,
        builder,
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn xylitol_plugin_sdk::XyEventSink>,
        xylitol_runtime::permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::empty())
    .build_baseline()
    .materialize_runtime();
    agent.select_model("mock").await.expect("select mock");
    let sid = uuid::Uuid::new_v4().to_string();
    agent.bind_session(sid).expect("bind_session");
    let mut driver = assemble_driver(agent, store_trait);

    let builtins = driver.builtins_for_reload();
    driver.freeze_tools(ToolSet::rebuild_agent_tools(
        builtins,
        vec![Arc::new(StubMcpTool("mcp__demo__alpha")) as Arc<dyn XyTool>],
    ));
    let mut s1 = driver.run("ping1").await;
    while s1.next().await.is_some() {}
    let first = seen.lock().expect("seen")[0].clone();
    assert!(
        first
            .iter()
            .any(|n| n.contains("alpha") || n == "mcp__demo__alpha"),
        "first provider tools MUST include alpha: {first:?}"
    );

    driver.reopen_tools_for_regate();
    let builtins = driver.builtins_for_reload();
    driver.freeze_tools(ToolSet::from_iter(builtins));
    let mut s2 = driver.run("ping2").await;
    while s2.next().await.is_some() {}
    let second = seen.lock().expect("seen")[1].clone();
    assert!(
        !second
            .iter()
            .any(|n| n.contains("alpha") || n == "mcp__demo__alpha"),
        "after remove re-freeze, provider tools MUST NOT include alpha: {second:?}"
    );

    driver.reopen_tools_for_regate();
    let builtins = driver.builtins_for_reload();
    driver.freeze_tools(ToolSet::rebuild_agent_tools(
        builtins,
        vec![Arc::new(StubMcpTool("mcp__demo__beta")) as Arc<dyn XyTool>],
    ));
    let mut s3 = driver.run("ping3").await;
    while s3.next().await.is_some() {}
    let third = seen.lock().expect("seen")[2].clone();
    assert!(
        third
            .iter()
            .any(|n| n.contains("beta") || n == "mcp__demo__beta"),
        "after add re-freeze, provider tools MUST include beta: {third:?}"
    );
    assert!(
        !third.iter().any(|n| n.contains("alpha")),
        "beta epoch MUST NOT keep alpha: {third:?}"
    );
}
