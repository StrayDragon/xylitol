use std::cell::RefCell;
use std::sync::Arc;

use xylitol::XyDriverError;
use xylitol::agent::capabilities::AgentCapabilities;
use xylitol::agent::runtime::{AgentRuntime, RunPolicy};
use xylitol::agent::tools::ToolSet;
use xylitol::infra::hooks::{HookDispatcher, HookEvent, HookPhase};
use xylitol::infra::session::SessionManager;

use crate::fixtures::AgentState;

pub(crate) fn result_ok_str(r: &RefCell<Option<Result<String, XyDriverError>>>) -> String {
    let guard = r.borrow();
    guard.as_ref().unwrap().as_ref().unwrap().clone()
}

/// Strip surrounding double quotes from `{text}` placeholders.
/// Also unescape \\n to real newlines and \\\" to real quotes (rstest-bdd captures escaped).
pub(crate) fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    let s = if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    s.replace("\\n", "\n").replace("\\\"", "\"")
}

/// Split on ` 或 ` and check if any stripped clause is in haystack (case-insensitive).
pub(crate) fn check_or_contains(haystack: &str, or_clause: &str) -> bool {
    let lower = haystack.to_lowercase();
    or_clause
        .split(" 或 ")
        .any(|c| lower.contains(&strip_quotes(c).to_lowercase()))
}

/// Borrow result as &str — callers must keep the Ref alive
pub(crate) fn make_agent(agent: &AgentState) -> AgentRuntime {
    make_agent_with_store(agent).0
}

pub(crate) fn make_agent_with_store(
    agent: &AgentState,
) -> (
    AgentRuntime,
    Arc<dyn xylitol::protocol::ports::XySessionStore>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    use std::sync::Arc;
    let store: Arc<dyn xylitol::protocol::ports::XySessionStore> = Arc::new(mgr.clone());
    let sink: Arc<dyn xylitol::protocol::ports::XyEventSink> =
        Arc::new(xylitol::infra::event::EventBus::new());
    let hook_bus: Option<Arc<dyn xylitol::XyHookBus>> = agent
        .wiring_hook_log
        .borrow()
        .clone()
        .map(|log| log as Arc<dyn xylitol::XyHookBus>);
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store.clone(),
        sink,
        Some("you are helpful".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        xylitol::agent::capabilities::QueueMode::default(),
        xylitol::agent::capabilities::QueueMode::default(),
        hook_bus,
    );
    // Harness often registers models without select; pick the first so ReAct can build.
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    (AgentRuntime::new(session), store)
}

pub(crate) fn bind_session_or_panic(agent: &mut AgentRuntime, session_id: impl Into<String>) {
    agent.bind_session(session_id).expect("bind_session");
}

pub(crate) async fn agent_submit_root(
    agent: &mut AgentRuntime,
    prompt: &str,
) -> xylitol::agent::XyEventStream {
    if agent.session_id().is_none() {
        bind_session_or_panic(agent, uuid::Uuid::new_v4().to_string());
    }
    agent.submit_root(prompt, RunPolicy::Reject).await
}

pub(crate) async fn agent_submit_root_with_id(
    agent: &mut AgentRuntime,
    prompt: &str,
    session_id: &str,
) -> xylitol::agent::XyEventStream {
    bind_session_or_panic(agent, session_id.to_string());
    agent.submit_root(prompt, RunPolicy::Reject).await
}

/// Library-seam operation dictionary (c990+). Unknown names return a readable Err.
/// Must not call `HookDispatcher::dispatch` directly — only XyDriver/agent APIs.
pub(crate) async fn run_wiring_operation(
    agent: &AgentState,
    op: &str,
) -> Result<(), XyDriverError> {
    use xylitol::embed::{XyDriver, XyInProcessDriver};
    use xylitol::protocol::model::ThinkingLevel;
    use xylitol::protocol::session::SessionTreeKind;

    match op {
        "确保新会话" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let orphan = uuid::Uuid::new_v4().to_string();
            bind_session_or_panic(&mut runtime, orphan);
            let driver = XyInProcessDriver::new(runtime, store);
            driver.session_tree(SessionTreeKind::MessageHistory).await?;
            Ok(())
        }
        "选择模型 fake" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (runtime, store) = make_agent_with_store(agent);
            let mut driver = XyInProcessDriver::new(runtime, store);
            driver.select_model("fake").map(|_| ())
        }
        "设置思考级别 high" => {
            let _ = agent.ensure_wiring_hook_log();
            ensure_wiring_fake_model(agent, true);
            let (runtime, store) = make_agent_with_store(agent);
            // Select fake first so a model exists; then change thinking.
            let mut driver = XyInProcessDriver::new(runtime, store);
            let _ = driver.select_model("fake");
            // Clear recorder so only thinking_level_select remains for key asserts.
            if let Some(log) = agent.wiring_hook_log.borrow().as_ref() {
                log.calls.lock().unwrap_or_else(|e| e.into_inner()).clear();
            }
            driver.set_thinking_level(ThinkingLevel::High).unwrap();
            Ok(())
        }
        "打开会话树" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let orphan = uuid::Uuid::new_v4().to_string();
            bind_session_or_panic(&mut runtime, orphan);
            let driver = XyInProcessDriver::new(runtime, store);
            driver
                .session_tree(SessionTreeKind::MessageHistory)
                .await
                .map(|_| ())
        }
        "切换会话 target" => {
            let _ = agent.ensure_wiring_hook_log();
            let (mut runtime, store) = make_agent_with_store(agent);
            let current = uuid::Uuid::new_v4().to_string();
            let target = "target".to_string();
            store.create(&current, Some("."), None).await?;
            store.create(&target, Some("."), None).await?;
            bind_session_or_panic(&mut runtime, current);
            let mut driver = XyInProcessDriver::new(runtime, store);
            driver.switch_session(&target).await.map(|_| ())
        }
        "执行 bash" => {
            let _ = agent.ensure_wiring_hook_log();
            let (runtime, store) = make_agent_with_store(agent);
            let driver = XyInProcessDriver::new(runtime, store);
            driver.execute_bash("true", false, None).await.map(|_| ())
        }
        other => Err(format!("未知操作: {other}").into()),
    }
}

pub(crate) fn ensure_wiring_fake_model(agent: &AgentState, thinking: bool) {
    use xylitol::protocol::model::XyModelMeta;
    use xylitol::protocol::model::{XyModelConfig, XyModelKind};
    let mut reg = agent.registry.borrow_mut();
    if reg.find("fake").is_some() {
        return;
    }
    reg.register(XyModelMeta {
        id: "fake".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Fake".into(),
        thinking,
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
}

pub(crate) async fn dispatch_hook(agent: &AgentState, event: HookEvent, phase: HookPhase) {
    agent
        .last_hook_stdin
        .replace(Some(event.to_json_context(phase)));
    let dispatcher = HookDispatcher::new(&xylitol::infra::config::types::HooksConfig {
        global: agent.hook_entries.borrow().clone(),
        project: vec![],
        user: vec![],
    });
    agent
        .hook_result
        .replace(Some(dispatcher.dispatch(&event, phase).await));
}

#[macro_export]
macro_rules! tool_call {
    ($tool:expr, $ctx:expr, $json:expr, $ws:expr) => {
        match $tool.execute(&$ctx, $json).await {
            Ok(r) => $ws.last_result.replace(Some(Ok(r))),
            Err(e) => $ws
                .last_result
                .replace(Some(Err(XyDriverError::from(e.to_string())))),
        }
    };
}
