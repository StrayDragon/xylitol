//! Steps for c2844 — turn-end overflow compaction event delivery contract.
//!
//! Drives a REAL AgentRuntime whose model replies with a context-overflow error
//! (overflow Case 1), so the run's own turn-end compaction emits CompactionStart
//! / CompactionEnd onto the run stream (c1730 tee). The scenario pins the order
//! contract: CompactionEnd MUST be delivered BEFORE AgentEnd — never silently
//! dropped by stream-tail starvation (the c2844 bug).

use std::cell::RefCell;
use std::sync::Arc;

use crate::bdd::fixtures::AgentState;
use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol::agent::capabilities::AgentCapabilities;
use xylitol::agent::runtime::RunPolicy;
use xylitol::infra::provider::fake_xy_model;
use xylitol::infra::provider::ScenarioStep;
use xylitol::infra::session::SessionManager;
use xylitol::infra::tools::default_tools;
use xylitol::protocol::model::XyModelConfig;
use xylitol::protocol::ports::XyModel;

pub(crate) struct CompactionProbe {
    pub(crate) runtime: RefCell<Option<AgentRuntime>>,
    pub(crate) events: RefCell<Vec<String>>,
}

#[fixture]
pub(crate) fn compaction_probe() -> CompactionProbe {
    CompactionProbe {
        runtime: RefCell::new(None),
        events: RefCell::new(Vec::new()),
    }
}

fn overflow_error_builder() -> xylitol::protocol::ports::XyModelBuilder {
    // c2844: the agent LLM replies with a context-overflow error so turn-end
    // overflow compaction (Case 1) fires through the real run stream.
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

fn small_window_meta(id: &str) -> xylitol::protocol::model::XyModelMeta {
    use xylitol::protocol::model::{XyModelConfig, XyModelKind, XyModelMeta};
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
async fn g_c2844_overflow_runtime(probe: &CompactionProbe, agent: &AgentState) {
    use xylitol::protocol::ports::XyEventSink;
    use xylitol::protocol::ports::XySessionStore;

    let mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let sid = "c2844-overflow";
    mgr.create(sid, Some("."), None).await.unwrap();
    // Summarizable history so the overflow turn-end compaction has content to cut.
    for i in 0..60 {
        use xylitol::protocol::session::{EntryBase, MessageEntry, SessionEntry};
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

    let mut reg = xylitol::agent::model::registry::ModelRegistry::new();
    reg.register(small_window_meta("overflow-fake"));
    let store: Arc<dyn XySessionStore> = Arc::new(mgr.clone());
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        reg,
        xylitol::infra::tools::ToolSet::from_iter(default_tools()),
        store,
        sink,
        Some("you are helpful".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        overflow_error_builder(),
        xylitol::infra::permission::allow_all_permission(),
        xylitol::agent::capabilities::QueueMode::default(),
        xylitol::agent::capabilities::QueueMode::default(),
        None,
    );
    session.select_model("overflow-fake").await.unwrap();
    let mut agent = AgentRuntime::new(session);
    agent.bind_session(sid).expect("bind");
    probe.runtime.replace(Some(agent));
    let _ = &agent;
}

#[when("提交一次回合并收齐事件名序列")]
async fn w_c2844_collect(probe: &CompactionProbe) {
    use futures::StreamExt;
    let mut agent = probe.runtime.borrow_mut().take().expect("runtime armed");
    let mut stream = agent.submit_root("continue the work", RunPolicy::Reject).await;
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
    probe.events.replace(names);
}

#[then("压缩开始与结束事件均到达且结束先于回合收尾")]
fn t_c2844_order(probe: &CompactionProbe) {
    let names = probe.events.borrow();
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
