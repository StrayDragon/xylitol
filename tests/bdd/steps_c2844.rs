use std::cell::RefCell;
use std::sync::Arc;

use crate::bdd::fixtures::AgentState;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_kernel::provider::ScenarioStep;
use xylitol_kernel::provider::fake_xy_model;
use xylitol_official_plugin_session::SessionManager;
use xylitol_official_plugin_tools::default_tools;
use xylitol_plugin_sdk::XyModel;
use xylitol_plugin_sdk::model::XyModelConfig;
use xylitol_runtime::agent::capabilities::AgentCapabilities;
use xylitol_runtime::agent::runtime::RunPolicy;
use xylitol_runtime::agent::tools::ToolSet;

pub(crate) struct CompactionProbe {
    pub(crate) runtime: RefCell<Option<AgentRuntime>>,
    pub(crate) events: RefCell<Vec<String>>,
    pub(crate) store: RefCell<Option<(Arc<dyn xylitol_plugin_sdk::XySessionStore>, String)>>,
    pub(crate) summary: RefCell<Option<String>>,
}

#[fixture]
pub(crate) fn compaction_probe() -> CompactionProbe {
    CompactionProbe {
        runtime: RefCell::new(None),
        events: RefCell::new(Vec::new()),
        store: RefCell::new(None),
        summary: RefCell::new(None),
    }
}

fn overflow_error_builder() -> xylitol_plugin_sdk::XyModelBuilder {
    Arc::new(move |_cfg: &XyModelConfig| -> Arc<dyn XyModel> {
        fake_xy_model(
            "overflow-fake",
            vec![ScenarioStep::error(
                "This model's maximum context length is 4096 tokens",
                false,
            )],
        )
    })
}

fn small_window_meta(id: &str) -> xylitol_plugin_sdk::model::XyModelMeta {
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind, XyModelMeta};
    XyModelMeta {
        id: id.into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "OverflowFake".into(),
        thinking: false,
        context_window: 4096,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: vec!["off".into()],
        thinking_level_map: Default::default(),
    }
}

#[given("以 overflow 错误响应模型装配可压缩运行库并预置可压缩历史")]
async fn g_c2844_overflow_runtime(compaction_probe: &CompactionProbe, _agent: &AgentState) {
    use xylitol_plugin_sdk::XyEventSink;
    use xylitol_plugin_sdk::XySessionStore;

    let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let sid = "c2844-overflow";
    mgr.create(sid, Some("."), None).await.unwrap();
    for i in 0..60 {
        use xylitol_plugin_sdk::session::{EntryBase, MessageEntry, SessionEntry};
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("seed-{i}"),
                parent_id: None,
                timestamp: 1704067200000 + i as u64,
            },
            message: serde_json::to_value(format!("seed turn {i} {}", "x".repeat(400))).unwrap(),
        });
        mgr.append(sid, &e).await.unwrap();
    }

    let mut reg = xylitol_runtime::agent::model::registry::ModelRegistry::new();
    reg.register(small_window_meta("overflow-fake"));
    let store: Arc<dyn XySessionStore> = Arc::new(mgr.clone());
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol_kernel::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        reg,
        ToolSet::from_iter(default_tools()),
        store,
        sink,
        Some("you are helpful".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        overflow_error_builder(),
        xylitol_runtime::permission::allow_all_permission(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        None,
    );
    session.install_compaction(xylitol_kernel::composition::official_compaction_factory());
    session.select_model("overflow-fake").await.unwrap();
    let mut agent = AgentRuntime::new(session);
    agent.bind_session(sid).expect("bind");
    compaction_probe.runtime.replace(Some(agent));
}

#[when("提交一次回合并收齐事件名序列")]
async fn w_c2844_collect(compaction_probe: &CompactionProbe) {
    use futures::StreamExt;
    let mut agent = compaction_probe
        .runtime
        .borrow_mut()
        .take()
        .expect("runtime armed");
    let mut stream = agent
        .submit_root("continue the work", RunPolicy::Reject)
        .await;
    let mut names = Vec::new();
    while let Some(e) = stream.next().await {
        let short = match e {
            XyEvent::CompactionStart { .. } => "Start".to_string(),
            XyEvent::CompactionEnd { .. } => "End".to_string(),
            XyEvent::AgentEnd { .. } => "AgentEnd".to_string(),
            _ => ".".to_string(),
        };
        names.push(short);
    }
    compaction_probe.events.replace(names);
}

#[then("压缩开始与结束事件均到达且结束先于回合收尾")]
fn t_c2844_order(compaction_probe: &CompactionProbe) {
    let names = compaction_probe.events.borrow();
    assert!(
        names.iter().any(|n| n == "Start"),
        "turn-end overflow compaction MUST emit CompactionStart: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "End"),
        "CompactionEnd MUST be delivered (c2844 stream-tail starvation): {names:?}"
    );
    let end_pos = names.iter().position(|n| n == "End").expect("end");
    let agent_end_pos = names.iter().position(|n| n == "AgentEnd").unwrap();
    assert!(
        end_pos < agent_end_pos,
        "CompactionEnd MUST arrive BEFORE AgentEnd: {names:?}"
    );
}

pub(crate) struct ReasoningOnlyModel;

#[async_trait::async_trait]
impl XyModel for ReasoningOnlyModel {
    fn name(&self) -> &str {
        "reasoning-only"
    }
    async fn generate_stream(
        &self,
        _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
        _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
        _stream: bool,
        _options: xylitol_plugin_sdk::XyGenerateOptions,
    ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
        use futures::stream::iter;
        use xylitol_plugin_sdk::message::XyStopReason;
        Ok(Box::pin(iter(vec![
            Ok(xylitol_plugin_sdk::model::XyChunk::ThinkingDelta(
                "## Goal\nreasoning-only checkpoint content\n## Next Steps\n1. finish".into(),
            )),
            Ok(xylitol_plugin_sdk::model::XyChunk::ThinkingEnd {
                thinking: String::new(),
                thinking_signature: None,
            }),
            Ok(xylitol_plugin_sdk::model::XyChunk::Done {
                finish_reason: XyStopReason::Stop,
                usage: None,
            }),
        ])))
    }
}

#[given("以仅输出推理的摘要模型为压缩绑定并预置历史")]
async fn g_c2844_reasoning_store(
    sess: &crate::bdd::fixtures::XySessionStore,
    compaction_probe: &CompactionProbe,
) {
    use xylitol_official_plugin_session::SessionManager;
    let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let sid = "c2844-reasoning";
    mgr.create(sid, Some("."), None).await.unwrap();
    for i in 0..120 {
        use xylitol_plugin_sdk::session::{EntryBase, MessageEntry, SessionEntry};
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("seed-{i}"),
                parent_id: None,
                timestamp: 1704067200000 + i as u64,
            },
            message: serde_json::to_value(xylitol_plugin_sdk::message::AgentMessage::user(
                format!("turn {i} {}", "y".repeat(400)),
            ))
            .unwrap(),
        });
        mgr.append(sid, &e).await.unwrap();
    }
    let _ = sess;
    compaction_probe.store.replace(Some((
        Arc::new(mgr.clone()) as Arc<dyn xylitol_plugin_sdk::XySessionStore>,
        sid.to_string(),
    )));
}

#[when("触发一次压缩")]
async fn w_c2844_reasoning_compact(compaction_probe: &CompactionProbe) {
    use xylitol_kernel::event::EventBus;
    use xylitol_official_plugin_compaction::CompactionOrchestrator;
    use xylitol_official_plugin_compaction::CompactionSettings;
    use xylitol_runtime::agent::model::task_model::CompactionSummaryBinding;

    let (store, sid) = compaction_probe
        .store
        .borrow()
        .clone()
        .expect("store armed");
    let model: Arc<dyn XyModel> = Arc::new(ReasoningOnlyModel);
    let binding = CompactionSummaryBinding::for_test(model, "fake");
    let sink = Arc::new(EventBus::new());
    let mut notice = false;
    CompactionOrchestrator::new(CompactionSettings {
        enabled: true,
        reserve_tokens: 1024,
        keep_recent_tokens: 1000,
        ..Default::default()
    })
    .compact(
        store.as_ref(),
        &sid,
        &binding,
        sink.as_ref(),
        None,
        8192,
        None,
        &mut notice,
    )
    .await
    .expect("c2844: reasoning-only compaction must succeed");

    let entries = store.load_leaf_branch(&sid).await.expect("load leaf");
    let entry = entries
        .iter()
        .rev()
        .find_map(|e| match e {
            xylitol_plugin_sdk::session::SessionEntry::Compaction(c) => Some(c.clone()),
            _ => None,
        })
        .expect("compaction entry must exist");
    compaction_probe
        .summary
        .replace(Some(entry.summary.clone()));
}

#[then("摘要非空且采用推理内容而非 fallback 占位")]
fn t_c2844_reasoning_summary(compaction_probe: &CompactionProbe) {
    let summary = compaction_probe.summary.borrow().clone().expect("summary");
    assert!(
        summary.contains("reasoning-only checkpoint content"),
        "MUST use the reasoning as summary content, got: {summary:?}"
    );
    assert!(
        !summary.contains("[Turn prefix:") && !summary.contains("[Compacted:"),
        "MUST NOT fall back on reasoning-only: {summary:?}"
    );
}
