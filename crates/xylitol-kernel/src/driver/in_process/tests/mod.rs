use std::sync::Arc;
use xylitol_official_plugin_provider_openai::provider::{ObsSessionContext, ObsSessionScope};

use super::*;
use crate::event::EventBus;
use xylitol_official_plugin_session::LogSessionStore;
use xylitol_plugin_sdk::session::{EntryBase, MessageEntry, SessionEntry};
use xylitol_plugin_sdk::{XyEventSink, XySessionStore};
use xylitol_runtime::XyDriver;
use xylitol_runtime::XySessionOps;
use xylitol_runtime::agent::AgentBuilder;
use xylitol_runtime::agent::tools::ToolSet;

type ModelBuilderFn = xylitol_plugin_sdk::XyModelBuilder;

fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: entry_ms(id),
        },
        message: xylitol_plugin_sdk::session::fixture_message_json(role, text),
    })
}

fn entry_ms(id: &str) -> u64 {
    1_781_827_200_000u64
        + id.as_bytes()
            .iter()
            .fold(0u64, |acc, b| acc * 31 + *b as u64)
            % 1_000_000
}

async fn build_test_driver(store: Arc<LogSessionStore>) -> (XyInProcessDriver, ObsSessionScope) {
    let scope = ObsSessionScope::enter(ObsSessionContext::default());
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut agent = AgentBuilder::new(
        xylitol_runtime::agent::model::registry::ModelRegistry::new(),
        Arc::new(crate::provider::factory::build_provider),
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
        xylitol_runtime::permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools(),
    ))
    .build_baseline()
    .materialize_runtime();
    let sid = uuid::Uuid::new_v4().to_string();
    store_trait
        .create(&sid, Some("."), None)
        .await
        .expect("create session");
    agent.bind_session(sid).expect("bind_session");
    (assemble_driver(agent, store), scope)
}

mod bash;
mod mcp;
mod session;
mod trust;

#[tokio::test]
async fn assemble_driver_full_with_custom_ops_builds_driver() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut agent = AgentBuilder::new(
        xylitol_runtime::agent::model::registry::ModelRegistry::new(),
        Arc::new(crate::provider::factory::build_provider),
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
        xylitol_runtime::permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools(),
    ))
    .build_baseline()
    .materialize_runtime();
    let sid = uuid::Uuid::new_v4().to_string();
    store_trait.create(&sid, Some("."), None).await.unwrap();
    agent.bind_session(sid).unwrap();

    let driver = assemble_driver_full(
        agent,
        store,
        BangExecHandler::new(None),
        SessionExporter::new(Some(Arc::new(crate::export::StdExportIo::new()))),
        ToolFeatures::default(),
        DriverContext::default(),
    );
    assert!(driver.session_id().is_some());
}

fn test_gateways(
    store: Arc<dyn XySessionStore>,
) -> (
    Arc<xylitol_official_plugin_todo::SessionAgentTodoGateway>,
    Arc<xylitol_official_plugin_workflow::SessionWorkflowGateway>,
    Arc<xylitol_official_plugin_tools::SessionAutomationGateway>,
) {
    (
        xylitol_official_plugin_todo::SessionAgentTodoGateway::new(store.clone()),
        xylitol_official_plugin_workflow::SessionWorkflowGateway::new(store.clone()),
        xylitol_official_plugin_tools::SessionAutomationGateway::new(store),
    )
}

#[cfg(feature = "fff")]
#[test]
fn compose_tools_prefers_fff_and_restores_builtin_when_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let store: Arc<dyn XySessionStore> =
        Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    let (todo, workflow, automation) = test_gateways(store);

    let mut static_tools = xylitol_official_plugin_tools::core_tools();
    static_tools.extend(xylitol_official_plugin_tools_fff::fff_tools());
    let context = DriverContext {
        project_root: dir.path().to_path_buf(),
        trusted: true,
        static_tools,
    };
    let features = ToolFeatures {
        todo: false,
        workflow: false,
        tool_selection: Vec::new(),
        ..Default::default()
    };

    let set = compose_tools(
        features.clone(),
        &context,
        todo.clone(),
        workflow.clone(),
        automation.clone(),
        None,
    );
    assert!(set.get("ffgrep").is_some(), "fff grep must be visible");
    assert!(set.get("grep").is_none(), "builtin grep must be replaced");

    let disabled = ToolFeatures {
        tool_selection: vec!["-ffgrep".into(), "-fffind".into()],
        ..features
    };
    let set = compose_tools(disabled, &context, todo, workflow, automation, None);
    assert!(set.get("grep").is_some(), "builtin grep must return");
    assert!(set.get("ffgrep").is_none());
}

#[test]
fn compose_tools_falls_back_to_core_when_registry_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store: Arc<dyn XySessionStore> =
        Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    let (todo, workflow, automation) = test_gateways(store);
    let context = DriverContext {
        project_root: dir.path().to_path_buf(),
        trusted: true,
        static_tools: Vec::new(),
    };
    let features = ToolFeatures {
        todo: false,
        workflow: false,
        ..Default::default()
    };
    let set = compose_tools(features, &context, todo, workflow, automation, None);
    assert!(set.get("grep").is_some());
    assert!(set.get("find").is_some());
    assert!(set.get("ffgrep").is_none());
}

#[tokio::test]
async fn capability_overlay_ends_frozen_with_consistent_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    let (mut driver, _scope) = build_test_driver(store).await;

    let tools = ToolSet::from_iter(xylitol_official_plugin_tools::default_tools());
    driver.install_base_tools(tools);
    assert!(driver.is_tools_frozen());

    let snapshot = driver.capability_snapshot();
    let id = snapshot
        .entries
        .first()
        .expect("capability entries")
        .id
        .clone();
    XyDriver::set_capability(&mut driver, &id, false)
        .await
        .expect("set capability");
    assert!(driver.capability_pending());

    driver.apply_pending_capability_overlay();
    assert!(!driver.capability_pending());
    assert!(driver.is_tools_frozen());

    let loaded = driver.loaded_resources_snapshot().await;
    assert!(
        loaded.tools_table_frozen,
        "loaded resources snapshot must agree with Frozen phase"
    );
}

#[tokio::test]
async fn attach_ctx_services_binds_prepare_hub_and_loop_registry() {
    struct ProbeLoop;

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::LoopMiddleware for ProbeLoop {
        fn id(&self) -> &str {
            "attach-ctx-probe"
        }

        fn stages(&self) -> &'static [xylitol_plugin_sdk::LoopStage] {
            &[xylitol_plugin_sdk::LoopStage::BeforeStep]
        }
    }

    struct ProbeHub {
        notice_taken: std::sync::atomic::AtomicBool,
    }

    impl ProbeHub {
        fn new() -> Self {
            Self {
                notice_taken: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }

    #[async_trait::async_trait]
    impl xylitol_plugin_sdk::SessionPrepareHub for ProbeHub {
        fn fire(&self) {}

        async fn reset_join(&self) {}

        async fn ensure(&self) -> xylitol_plugin_sdk::SessionPrepareResult {
            xylitol_plugin_sdk::SessionPrepareResult::default()
        }

        async fn wait(
            &self,
            _timeout: std::time::Duration,
        ) -> xylitol_plugin_sdk::SessionPrepareResult {
            xylitol_plugin_sdk::SessionPrepareResult::default()
        }

        fn view(&self) -> xylitol_plugin_sdk::SessionPrepareView {
            xylitol_plugin_sdk::SessionPrepareView::default()
        }

        fn take_notice(&self) -> Option<String> {
            if self
                .notice_taken
                .swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                None
            } else {
                Some("attach-ctx-hub-notice".into())
            }
        }

        fn tools(&self) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
            Vec::new()
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(LogSessionStore::new(dir.path().join("sessions")));
    let (mut driver, _scope) = build_test_driver(store).await;

    use xylitol_plugin_sdk::rt::CtxHost;
    let ctx = xylitol_plugin_sdk::rt::Ctx::root().expect("root ctx");
    let hub: Arc<dyn xylitol_plugin_sdk::SessionPrepareHub> = Arc::new(ProbeHub::new());
    ctx.provide_as::<dyn xylitol_plugin_sdk::SessionPrepareHub>(hub)
        .expect("provide hub");
    let registry = Arc::new(xylitol_plugin_sdk::LoopMiddlewareRegistry::new());
    registry
        .register(Arc::new(ProbeLoop))
        .expect("register probe");
    ctx.provide_as::<xylitol_plugin_sdk::LoopMiddlewareRegistry>(registry)
        .expect("provide registry");

    attach_ctx_services(&mut driver, &ctx);

    assert_eq!(
        driver.take_mcp_gate_notice().await.as_deref(),
        Some("attach-ctx-hub-notice")
    );
    assert_eq!(driver.take_mcp_gate_notice().await, None);
    assert_eq!(
        driver.agent.loop_middleware_ids(),
        vec!["attach-ctx-probe".to_string()]
    );

    drop(driver);
    let _ = ctx.shutdown().await;
}
