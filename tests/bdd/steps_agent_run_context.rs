use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use rstest_bdd_macros::{given, then, when};
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage, XyStopReason};
use xylitol_plugin_sdk::{XyChunk, XyGenerateOptions, XyModel, XyStream};
use xylitol_runtime::agent::capabilities::{AgentCapabilities, QueueMode};
use xylitol_runtime::agent::runtime::AgentRuntime;
use xylitol_runtime::agent::tools::ToolSet;

use crate::bdd::fixtures::{AgentState, Workspace};
use crate::bdd::helpers::{agent_submit_turn, bind_session_or_panic};
use crate::bdd::steps_agent_runtime::ar_register_fake;

const SESSION_ID: &str = "agent-run-context";

thread_local! {
    static CONTEXT: std::cell::RefCell<Vec<AgentMessage>> = const { std::cell::RefCell::new(Vec::new()) };
    static SYSTEM: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    static ENTRIES: std::cell::RefCell<Vec<xylitol_official_plugin_session::SessionEntry>> = const { std::cell::RefCell::new(Vec::new()) };
    static EVENTS: std::cell::RefCell<Vec<xylitol_plugin_sdk::XyEvent>> = const { std::cell::RefCell::new(Vec::new()) };
    static CAPTURED_SYSTEM: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

struct CaptureModel {
    captured: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl XyModel for CaptureModel {
    fn name(&self) -> &str {
        "bdd-run-context-capture"
    }

    async fn generate_stream(
        &self,
        _messages: Vec<XyLlmMessage>,
        _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
        _stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, xylitol_plugin_sdk::error::XyError> {
        *self.captured.lock().expect("capture lock") = options.system_prompt.clone();
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(XyChunk::TextDelta("answer".into())),
            Ok(XyChunk::Done {
                finish_reason: XyStopReason::Stop,
                usage: None,
            }),
        ])))
    }
}

fn reset_context() {
    CONTEXT.with(|c| c.borrow_mut().clear());
    SYSTEM.with(|s| *s.borrow_mut() = None);
    ENTRIES.with(|e| e.borrow_mut().clear());
    EVENTS.with(|e| e.borrow_mut().clear());
    CAPTURED_SYSTEM.with(|c| *c.borrow_mut() = None);
    xylitol_kernel::provider::factory::reset_fake_state();
}

fn prepare(agent: &AgentState, ws: &Workspace) {
    reset_context();
    ws.init();
    ar_register_fake(agent, "ar-run-context");
    xylitol_kernel::provider::factory::set_fake_text("answer");
}

fn capture_runner(
    agent: &AgentState,
) -> (
    AgentRuntime,
    Arc<dyn xylitol_plugin_sdk::XySessionStore>,
    Arc<Mutex<Option<String>>>,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mgr = xylitol_official_plugin_session::SessionManager::new(dir.keep());
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(mgr);
    let sink: Arc<dyn xylitol_plugin_sdk::XyEventSink> =
        Arc::new(xylitol_kernel::event::EventBus::new());
    let todo = xylitol_official_plugin_todo::SessionAgentTodoGateway::new(store.clone());
    let mut tools = xylitol_official_plugin_tools::core_tools();
    tools.extend(xylitol_official_plugin_todo::todo_tools(todo));
    let captured = Arc::new(Mutex::new(None));
    let captured_for_builder = captured.clone();
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(tools),
        store.clone(),
        sink,
        Some("boot system".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        Arc::new(move |_| {
            Arc::new(CaptureModel {
                captured: captured_for_builder.clone(),
            }) as Arc<dyn XyModel>
        }),
        xylitol_runtime::permission::allow_all_permission(),
        QueueMode::default(),
        QueueMode::default(),
        None,
    );
    session.install_compaction(xylitol_kernel::composition::official_compaction_factory());
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = futures::executor::block_on(session.select_model(&id));
    }
    (AgentRuntime::new(session), store, captured)
}

fn entry_texts(entries: &[xylitol_official_plugin_session::SessionEntry]) -> Vec<(String, String)> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            xylitol_official_plugin_session::SessionEntry::Message(message) => {
                let role = message.message.get("role")?.as_str()?.to_owned();
                let content = message.message.get("content")?.as_array()?;
                let text = content
                    .iter()
                    .filter_map(|part| part.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                Some((role, text))
            }
            _ => None,
        })
        .collect()
}

async fn run(agent: &AgentState, capture_system: bool) {
    let (mut runner, store, captured) = if capture_system {
        capture_runner(agent)
    } else {
        let (runner, store) = crate::bdd::helpers::make_agent_with_store(agent);
        (runner, store, Arc::new(Mutex::new(None)))
    };
    bind_session_or_panic(&mut runner, SESSION_ID);
    let context = CONTEXT.with(|c| c.borrow().clone());
    let system = SYSTEM.with(|s| s.borrow().clone());
    let mut stream = agent_submit_turn(
        &mut runner,
        vec![AgentPart::text("new question")],
        context,
        system,
    )
    .await;
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        if let xylitol_plugin_sdk::XyEvent::Error(error) = &event {
            panic!("agent-run-context run failed: {error:?}");
        }
        events.push(event);
    }
    let entries = store.load_entries(SESSION_ID).await.unwrap_or_default();
    ENTRIES.with(|e| {
        let mut out = e.borrow_mut();
        out.clear();
        out.extend(entries);
    });
    EVENTS.with(|e| {
        let mut out = e.borrow_mut();
        out.clear();
        out.extend(events);
    });
    CAPTURED_SYSTEM.with(|c| *c.borrow_mut() = captured.lock().expect("capture lock").clone());
}

#[given("请求携带两条前序消息")]
pub(crate) fn given_two_context(agent: &AgentState, ws: &Workspace) {
    prepare(agent, ws);
    CONTEXT.with(|c| {
        *c.borrow_mut() = vec![
            AgentMessage::user("first"),
            AgentMessage::assistant("second"),
        ];
    });
}

#[given("请求携带一条前序 assistant 消息")]
pub(crate) fn given_assistant_context(agent: &AgentState, ws: &Workspace) {
    prepare(agent, ws);
    CONTEXT.with(|c| *c.borrow_mut() = vec![AgentMessage::assistant("prior")]);
}

#[given("请求携带 system prompt 覆盖")]
pub(crate) fn given_system_override(agent: &AgentState, ws: &Workspace) {
    prepare(agent, ws);
    SYSTEM.with(|s| *s.borrow_mut() = Some("override system".into()));
}

#[given("请求携带上下文与一条新 user 消息")]
pub(crate) fn given_context_and_user(agent: &AgentState, ws: &Workspace) {
    prepare(agent, ws);
    CONTEXT.with(|c| *c.borrow_mut() = vec![AgentMessage::user("prior")]);
}

#[when("运行一轮")]
pub(crate) async fn when_run_one_turn(agent: &AgentState) {
    run(agent, false).await;
}

#[when("运行一轮并检查会话条目")]
pub(crate) async fn when_run_and_check_entries(agent: &AgentState) {
    run(agent, false).await;
}

#[when("运行一轮并检查生成选项")]
pub(crate) async fn when_run_and_check_options(agent: &AgentState) {
    run(agent, true).await;
}

#[when("运行完成")]
pub(crate) async fn when_run_complete(agent: &AgentState) {
    run(agent, false).await;
}

#[then("模型收到的历史以这两条消息开头")]
pub(crate) fn then_history_starts_with_context() {
    let events = EVENTS.with(|e| e.borrow().clone());
    let messages = events
        .iter()
        .rev()
        .find_map(|event| match event {
            xylitol_plugin_sdk::XyEvent::AgentEnd { messages, .. } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd with history");
    let texts: Vec<String> = messages
        .iter()
        .filter(|message| message.role_name() != "system")
        .map(|message| message.text())
        .collect();
    assert!(texts.len() >= 2, "expected seeded history: {texts:?}");
    assert_eq!(texts[0], "first");
    assert_eq!(texts[1], "second");
}

#[then("会话不含该前序消息且含本轮 user 与 assistant")]
pub(crate) fn then_seed_not_persisted() {
    let entries = ENTRIES.with(|e| e.borrow().clone());
    let texts = entry_texts(&entries);
    assert!(
        texts.iter().all(|(_, text)| text != "prior"),
        "seed must not be persisted: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|(role, text)| role == "user" && text == "new question")
    );
    assert!(
        texts
            .iter()
            .any(|(role, text)| role == "assistant" && text == "answer")
    );
}

#[then("生成选项使用覆盖值且历史无 system 角色行")]
pub(crate) fn then_system_override() {
    let captured = CAPTURED_SYSTEM.with(|c| c.borrow().clone());
    assert_eq!(captured.as_deref(), Some("override system"));
    let entries = ENTRIES.with(|e| e.borrow().clone());
    let roles: Vec<String> = entries
        .iter()
        .filter_map(|entry| match entry {
            xylitol_official_plugin_session::SessionEntry::Message(message) => message
                .message
                .get("role")
                .and_then(|role| role.as_str())
                .map(str::to_owned),
            _ => None,
        })
        .collect();
    assert!(
        roles.iter().all(|role| role != "system"),
        "system override must not be persisted: {roles:?}"
    );
}

#[then("会话包含新 user 与本轮 assistant")]
pub(crate) fn then_current_turn_persisted() {
    let entries = ENTRIES.with(|e| e.borrow().clone());
    let texts = entry_texts(&entries);
    assert!(
        texts
            .iter()
            .any(|(role, text)| role == "user" && text == "new question")
    );
    assert!(
        texts
            .iter()
            .any(|(role, text)| role == "assistant" && text == "answer")
    );
}
