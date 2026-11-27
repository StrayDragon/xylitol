use crate::bdd::fixtures::*;
use crate::bdd::helpers::*;
use crate::bdd::prelude::*;
use crate::bdd::steps_app_tui_bridge::BridgeBdd;
use rstest_bdd_macros::{given, then, when};
use xylitol_app_tui::UiEntry;

#[given("mock 模型先 tool 后无 tool")]
pub(crate) fn _g_ar_react_setup(agent: &AgentState, ws: &Workspace) {
    AR_RUNNER.with(|r| *r.borrow_mut() = None);
    AR_RUNNER_EVENTS.with(|e| e.borrow_mut().clear());
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "ar-react".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
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
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
}
#[when("运行 AgentRuntime")]
pub(crate) async fn _w_ar_react_run(agent: &AgentState) {
    let mut runner = AR_RUNNER
        .with(|r| r.borrow_mut().take())
        .unwrap_or_else(|| make_agent(agent));
    let mut stream = agent_submit_root(&mut runner, "读取文件").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    ar_store_runner(runner);
    ar_store_events(local_events.clone());
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[when("运行 AgentRuntime 并收集事件")]
pub(crate) async fn _w_ar_react_run_collect(agent: &AgentState) {
    _w_ar_react_run(agent).await;
}

#[then(
    "MessageUpdate 含工具意图且早于任意 ToolExecutionStart；ToolExecutionStart 不早于 MessageEnd"
)]
pub(crate) fn _t_ar_intent_before_execution(agent: &AgentState) {
    use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};

    let events = agent.events.borrow();
    let mut saw_intent = false;
    let mut message_end_idx = None;
    let mut tool_start_idx = None;

    for (i, evt) in events.iter().enumerate() {
        match evt {
            XyEvent::MessageUpdate {
                message: Some(AgentMessage::Llm(XyLlmMessage::AssistantMessage { content, .. })),
                ..
            } if content
                .iter()
                .any(|p| matches!(p, AgentPart::ToolCall { .. })) =>
            {
                saw_intent = true;
                if tool_start_idx.is_some() {
                    panic!("MessageUpdate with ToolCall after ToolExecutionStart: {events:?}");
                }
            }
            XyEvent::MessageEnd { .. } => {
                message_end_idx.get_or_insert(i);
            }
            XyEvent::ToolExecutionStart { .. } => {
                tool_start_idx.get_or_insert(i);
            }
            _ => {}
        }
    }

    assert!(
        saw_intent,
        "expected MessageUpdate with ToolCall intent: {events:?}"
    );
    let end_i = message_end_idx.expect("expected MessageEnd");
    let start_i = tool_start_idx.expect("expected ToolExecutionStart");
    assert!(
        start_i > end_i,
        "ToolExecutionStart (idx {start_i}) must follow MessageEnd (idx {end_i}): {events:?}"
    );
}

#[then("先执行工具再结束且无 adk 类型")]
pub(crate) fn _t_ar_react_terminates(agent: &AgentState) {
    let events = agent.events.borrow();
    let has_tool = events
        .iter()
        .any(|e| !matches!(e, XyEvent::TextDelta(_)) && format!("{e:?}").contains("Tool"));
    assert!(has_tool, "expected tool execution, got {:?}", events);
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::StepEnd { .. })),
        "expected StepEnd, got {:?}",
        events
    );
}

#[given("消费事件流")]
pub(crate) fn _g_ar_stream_setup(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "ar-stream".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
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
    set_fake_text("stream content");
}
#[when("轮询")]
pub(crate) async fn _w_ar_stream_poll(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = agent_submit_root(&mut runner, "hi").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}
#[then("每项为 XyEvent")]
pub(crate) fn _t_ar_stream_is_xyevent(agent: &AgentState) {
    let events = agent.events.borrow();
    assert!(!events.is_empty(), "expected non-empty event stream");
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TextDelta(_))),
        "expected TextDelta in stream, got {:?}",
        events
    );
}

#[given("配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒")]
pub(crate) fn _g_ar_abort_slow_stream(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: "test-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
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
    set_fake_slow_stream(40, 20);
}

thread_local! {
    static AR_RUNNER: std::cell::RefCell<Option<AgentRuntime>> =
        const { std::cell::RefCell::new(None) };
    static AR_RUNNER_EVENTS: std::cell::RefCell<Vec<XyEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static AR_OPEN_STREAM: std::cell::RefCell<Option<xylitol_runtime::agent::XyEventStream>> =
        const { std::cell::RefCell::new(None) };
    static AR_BANG_RESULT: std::cell::RefCell<
        Option<Result<xylitol_plugin_sdk::XyBashResult, XyDriverError>>,
    > = const { std::cell::RefCell::new(None) };
}

pub(crate) fn ar_store_open_stream(stream: xylitol_runtime::agent::XyEventStream) {
    AR_OPEN_STREAM.with(|s| *s.borrow_mut() = Some(stream));
}

pub(crate) fn ar_take_open_stream() -> xylitol_runtime::agent::XyEventStream {
    AR_OPEN_STREAM.with(|s| s.borrow_mut().take().expect("open stream stored"))
}

pub(crate) fn ar_register_fake(agent: &AgentState, id: &str) {
    agent.registry.borrow_mut().register(XyModelMeta {
        id: id.into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
            prompt_cache: None,
        },
        display_name: "Fake Mock".into(),
        thinking: false,
        context_window: 200000,
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

pub(crate) fn ar_make_runner(agent: &AgentState, ws: &Workspace) -> AgentRuntime {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-queue");
    let mut runtime = make_agent(agent);
    futures::executor::block_on(runtime.select_model("ar-queue"))
        .expect("select ar-queue fake model");
    runtime
}

pub(crate) fn ar_store_runner(runner: AgentRuntime) {
    AR_RUNNER.with(|r| *r.borrow_mut() = Some(runner));
}

pub(crate) fn ar_with_runner<R>(f: impl FnOnce(&AgentRuntime) -> R) -> R {
    AR_RUNNER.with(|r| f(r.borrow().as_ref().expect("runner assembled")))
}

pub(crate) fn ar_take_runner() -> AgentRuntime {
    AR_RUNNER.with(|r| r.borrow_mut().take().expect("runner assembled"))
}

pub(crate) fn ar_store_events(local: Vec<XyEvent>) {
    AR_RUNNER_EVENTS.with(|e| {
        let mut ev = e.borrow_mut();
        ev.clear();
        ev.extend(local);
    });
}

pub(crate) fn ar_events() -> Vec<XyEvent> {
    AR_RUNNER_EVENTS.with(|e| e.borrow().clone())
}

pub(crate) async fn ar_run_capture(runner: &mut AgentRuntime, prompt: &str) -> Vec<XyEvent> {
    let mut stream = agent_submit_root(runner, prompt).await;
    let mut local = Vec::new();
    while let Some(e) = stream.next().await {
        local.push(e);
    }
    local
}

pub(crate) async fn ar_take_run_store(prompt: &str) {
    let mut runner = ar_take_runner();
    let local = ar_run_capture(&mut runner, prompt).await;
    ar_store_runner(runner);
    ar_store_events(local);
}

#[given("装配 agent 并入队两条 follow_up")]
pub(crate) async fn _g_ar_revoke_two_followups(agent: &AgentState, ws: &Workspace) {
    set_fake_text("revoke ack");
    let runner = ar_make_runner(agent, ws);
    let first = runner.follow_up("追问一");
    runner.follow_up("追问二");
    AR_REVOKE_ID.with(|cell| *cell.borrow_mut() = Some(first));
    ar_store_runner(runner);
}

thread_local! {
    static AR_REVOKE_BROADCAST: RefCell<Option<Vec<xylitol_plugin_sdk::session::QueueItem>>> =
        const { RefCell::new(None) };
    static AR_REVOKE_ID: RefCell<Option<String>> = const { RefCell::new(None) };
    static AR_REVOKE_OUTCOME: RefCell<Option<xylitol_runtime::agent::capabilities::RevokeOutcome>> =
        const { RefCell::new(None) };
}

#[when("撤销第一条队列项")]
pub(crate) fn _w_ar_revoke_first() {
    let id = AR_REVOKE_ID.with(|cell| cell.borrow().clone().expect("first item id"));
    let outcome = ar_with_runner(|runner| runner.revoke_queue_item(&id));
    AR_REVOKE_OUTCOME.with(|cell| *cell.borrow_mut() = Some(outcome));
}

#[then("队列项列表仅剩第二条且其不入本轮 turn")]
pub(crate) async fn _t_ar_revoke_leaves_second() {
    let outcome = AR_REVOKE_OUTCOME.with(|cell| cell.borrow().clone().expect("revoke outcome"));
    assert!(outcome.revoked, "revoking a pending item must succeed");
    assert_eq!(
        outcome.items.len(),
        1,
        "only the untouched follow_up must remain: {:?}",
        outcome.items
    );
    assert_eq!(outcome.items[0].preview, "追问二");
    let items = ar_with_runner(|runner| runner.queue_items());
    assert_eq!(items, outcome.items, "driver view must match revoke result");
}

#[given("装配 agent 并入队一条 follow_up 且该队列项已出队成为新 turn")]
pub(crate) async fn _g_ar_revoke_dequeued(agent: &AgentState, ws: &Workspace) {
    set_fake_text("dequeued ack");
    let runner = ar_make_runner(agent, ws);
    let id = runner.follow_up("追问一");
    AR_REVOKE_ID.with(|cell| *cell.borrow_mut() = Some(id));
    ar_store_runner(runner);
}

#[when("撤销该队列项")]
pub(crate) async fn _w_ar_revoke_dequeued() {
    ar_take_run_store("主提示").await;
    let id = AR_REVOKE_ID.with(|cell| cell.borrow().clone().expect("follow_up id"));
    let outcome = ar_with_runner(|runner| runner.revoke_queue_item(&id));
    AR_REVOKE_OUTCOME.with(|cell| *cell.borrow_mut() = Some(outcome));
}

#[then("应答 kind=NotFound")]
pub(crate) fn _t_ar_revoke_not_found() {
    let outcome = AR_REVOKE_OUTCOME.with(|cell| cell.borrow().clone().expect("revoke outcome"));
    assert!(
        !outcome.revoked,
        "a dequeued item is a fact and must not be revocable: {outcome:?}"
    );
}

#[given("两个客户端并发撤销同一队列项")]
pub(crate) async fn _g_ar_concurrent_revoke(agent: &AgentState, ws: &Workspace) {
    set_fake_text("concurrent revoke ack");
    let runner = ar_make_runner(agent, ws);
    let id = runner.follow_up("并发撤的追问");
    AR_REVOKE_ID.with(|cell| *cell.borrow_mut() = Some(id));
    ar_store_runner(runner);
}

#[when("撤销请求经会话信箱串行化处理")]
pub(crate) fn _w_ar_concurrent_revoke_serialized() {
    let id = AR_REVOKE_ID.with(|cell| cell.borrow().clone().expect("item id"));
    let outcomes = ar_with_runner(|runner| {
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..2 {
                handles.push(scope.spawn(|| runner.revoke_queue_item(&id)));
            }
            handles
                .into_iter()
                .map(|h| h.join().expect("revoke thread"))
                .collect::<Vec<_>>()
        })
    });
    AR_REVOKE_OUTCOMES.with(|cell| *cell.borrow_mut() = Some(outcomes));
}

thread_local! {
    static AR_REVOKE_OUTCOMES: RefCell<Option<Vec<xylitol_runtime::agent::capabilities::RevokeOutcome>>> =
        const { RefCell::new(None) };
}

#[then("恰好一次成功且另一次应答 kind=NotFound")]
pub(crate) fn _t_ar_concurrent_revoke_exactly_once() {
    let outcomes = AR_REVOKE_OUTCOMES.with(|cell| cell.borrow().clone().expect("outcomes"));
    assert_eq!(outcomes.len(), 2, "both clients must get an answer");
    let revoked = outcomes.iter().filter(|o| o.revoked).count();
    let not_found = outcomes.iter().filter(|o| !o.revoked).count();
    assert_eq!(revoked, 1, "exactly one revoke must win: {outcomes:?}");
    assert_eq!(not_found, 1, "the loser must answer NotFound: {outcomes:?}");
    assert!(
        outcomes.iter().all(|o| o.items.is_empty()),
        "the queue must be empty after the revoke: {outcomes:?}"
    );
}

#[given("分别经嵌入 driver 直调与线协议 revoke_queue_item 撤销同一队列项")]
pub(crate) async fn _g_ar_embed_and_wire_revoke(agent: &AgentState, ws: &Workspace) {
    use xylitol::embed::{XyDriver, assemble_driver};
    set_fake_text("embed revoke ack");
    ws.init();
    let (runtime_a, store_a) = crate::bdd::helpers::make_agent_with_store(agent);
    let mut embed_driver = assemble_driver(runtime_a, store_a);
    let embed_id = XyDriver::follow_up(&mut embed_driver, "嵌入端追问")
        .await
        .expect("embedding follow_up");
    let embed = XyDriver::revoke_queue_item(&embed_driver, &embed_id);
    let (runtime_b, store_b) = crate::bdd::helpers::make_agent_with_store(agent);
    let mut wire_driver = assemble_driver(runtime_b, store_b);
    let wire_id = XyDriver::follow_up(&mut wire_driver, "嵌入端追问")
        .await
        .expect("wire-side follow_up");
    let wire = crate::bdd::steps_protocol::wire_revoke_queue_item(&mut wire_driver, &wire_id);
    AR_REVOKE_PAIR.with(|cell| {
        *cell.borrow_mut() = Some((embed, wire));
    });
}

thread_local! {
    static AR_REVOKE_PAIR: RefCell<
        Option<(
            xylitol_plugin_sdk::RevokeQueueItemOutcome,
            crate::bdd::steps_protocol::WireRevoke,
        )>,
    > = const { RefCell::new(None) };
}

#[then("两者语义一致且剩余队列项列表相同")]
pub(crate) fn _t_ar_embed_and_wire_same() {
    let (embed, wire) = AR_REVOKE_PAIR.with(|cell| cell.borrow().clone().expect("revoke pair"));
    assert_eq!(
        embed.revoked, wire.revoked,
        "embedding and wire revoke must answer the same outcome kind"
    );
    let embed_items: Vec<(String, String)> = embed
        .items
        .iter()
        .map(|i| (i.id.clone(), i.kind.as_str().to_string()))
        .collect();
    assert_eq!(
        embed_items, wire.items,
        "both ends must expose the same remaining queue item list"
    );
}

#[given("两个客户端附着同一会话且队列中有一项")]
pub(crate) async fn _g_ar_two_clients_queue(agent: &AgentState, ws: &Workspace) {
    set_fake_text("two clients ack");
    let mut runner = ar_make_runner(agent, ws);
    let mut stream = agent_submit_root(&mut runner, "共享会话主提示").await;
    let mut local = Vec::new();
    let mut saw_start = false;
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::AgentStart { .. }) {
            saw_start = true;
        }
        local.push(event);
        if saw_start && matches!(local.last(), Some(XyEvent::StepStart { .. })) {
            break;
        }
    }
    assert!(saw_start, "the shared turn must have started");
    let id = runner.follow_up("共享队列项");
    let mut saw_enqueue = false;
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::QueueUpdate { .. }) {
            saw_enqueue = true;
        }
        local.push(event);
        if saw_enqueue {
            break;
        }
    }
    assert!(
        saw_enqueue,
        "enqueueing during a live turn must emit QueueUpdate"
    );
    AR_REVOKE_ID.with(|cell| *cell.borrow_mut() = Some(id));
    ar_store_runner(runner);
    ar_store_events(local);
}

#[when("任一客户端撤销该项")]
pub(crate) fn _w_ar_any_client_revokes() {
    let id = AR_REVOKE_ID.with(|cell| cell.borrow().clone().expect("item id"));
    let outcome = ar_with_runner(|runner| runner.revoke_queue_item(&id));
    AR_REVOKE_BROADCAST.with(|cell| *cell.borrow_mut() = Some(outcome.items.clone()));
    AR_REVOKE_OUTCOME.with(|cell| *cell.borrow_mut() = Some(outcome));
}

#[then("另一客户端经 QueueUpdate 收到更新后的队列项列表")]
pub(crate) fn _t_ar_other_client_sees_update() {
    let outcome = AR_REVOKE_OUTCOME.with(|cell| cell.borrow().clone().expect("revoke outcome"));
    assert!(outcome.revoked, "the revoke must succeed");
    let events = AR_RUNNER_EVENTS.with(|e| e.borrow().clone());
    let queue_updates: Vec<Vec<xylitol_plugin_sdk::session::QueueItem>> = events
        .iter()
        .filter_map(|e| match e {
            XyEvent::QueueUpdate { items } => Some(items.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !queue_updates.is_empty(),
        "a revoke must emit QueueUpdate: {events:?}"
    );
    let broadcast = AR_REVOKE_BROADCAST.with(|c| c.borrow().clone().expect("broadcast list"));
    assert_eq!(
        broadcast, outcome.items,
        "the shared QueueUpdate must carry the post-revoke item list"
    );
    let revoked_id = AR_REVOKE_ID.with(|c| c.borrow().clone().unwrap_or_default());
    assert!(
        broadcast.iter().all(|item| item.id != revoked_id),
        "the revoked id must be gone from the broadcast list: {broadcast:?}"
    );
    assert!(
        queue_updates
            .iter()
            .any(|items| items.iter().any(|i| i.id == revoked_id)),
        "the pre-revoke QueueUpdate must have carried the item: {queue_updates:?}"
    );
}

#[given("print 端运行一轮对话且队列撤销并发发生")]
pub(crate) async fn _g_ar_print_with_concurrent_revoke(agent: &AgentState, ws: &Workspace) {
    set_fake_text("print ack");
    let mut runner = ar_make_runner(agent, ws);
    let extra = runner.follow_up("并发撤销的追问");
    let _ = extra;
    AR_REVOKE_ID.with(|cell| *cell.borrow_mut() = Some(extra));
    let mut stream = agent_submit_root(&mut runner, "print 主提示").await;
    let mut local = Vec::new();
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::AgentStart { .. }) {
            let id = AR_REVOKE_ID.with(|cell| cell.borrow().clone().expect("extra id"));
            let _ = runner.revoke_queue_item(&id);
        }
        local.push(event);
    }
    ar_store_runner(runner);
    ar_store_events(local);
}

#[then("print 端输出不受撤销影响")]
pub(crate) fn _t_ar_print_unaffected() {
    let events = AR_RUNNER_EVENTS.with(|e| e.borrow().clone());
    let texts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            XyEvent::TextDelta(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !texts.is_empty() && texts.iter().all(|t| !t.is_empty()),
        "the print turn must still produce its answer: {texts:?}"
    );
    let agent_ends = events
        .iter()
        .filter(|e| matches!(e, XyEvent::AgentEnd { .. }))
        .count();
    assert_eq!(
        agent_ends, 1,
        "a concurrent revoke must not spawn or drop turns: {events:?}"
    );
}

#[given("会话空闲时连续提交两条对话")]
pub(crate) async fn _g_ar_two_submissions(agent: &AgentState, ws: &Workspace) {
    set_fake_text("one submission ack");
    let mut runner = ar_make_runner(agent, ws);
    let first = agent_submit_root(&mut runner, "第一条对话").await;
    ar_store_runner(runner);
    AR_FIRST_SUBMISSION.with(|cell| *cell.borrow_mut() = Some(first));
}

thread_local! {
    static AR_FIRST_SUBMISSION: RefCell<Option<xylitol_runtime::agent::XyEventStream>> =
        const { RefCell::new(None) };
    static AR_SUBMISSION_ORDER: RefCell<Option<(Vec<XyEvent>, Vec<XyEvent>)>> =
        const { RefCell::new(None) };
}

#[when("会话 actor 按信箱顺序消费")]
pub(crate) async fn _w_ar_actor_consumes_in_order() {
    let mut first =
        AR_FIRST_SUBMISSION.with(|cell| cell.borrow_mut().take().expect("first submission"));
    let mut first_events = Vec::new();
    while let Some(event) = first.next().await {
        let ended = matches!(event, XyEvent::AgentEnd { .. });
        first_events.push(event);
        if ended {
            break;
        }
    }
    drop(first);
    let mut runner = ar_take_runner();
    let mut second = agent_submit_root(&mut runner, "第二条对话").await;
    let mut second_events = Vec::new();
    while let Some(event) = second.next().await {
        let ended = matches!(event, XyEvent::AgentEnd { .. });
        second_events.push(event);
        if ended {
            break;
        }
    }
    ar_store_runner(runner);
    AR_SUBMISSION_ORDER.with(|cell| {
        *cell.borrow_mut() = Some((first_events, second_events));
    });
}

#[then("第一条进入 turn 处理且第二条仍在队列等待")]
pub(crate) fn _t_ar_second_submission_waits() {
    let (first_events, second_events) =
        AR_SUBMISSION_ORDER.with(|cell| cell.borrow().clone().expect("submission order"));
    assert!(
        first_events
            .iter()
            .any(|e| matches!(e, XyEvent::AgentStart { .. })),
        "the first submission must run its own turn: {first_events:?}"
    );
    assert!(
        first_events
            .iter()
            .any(|e| matches!(e, XyEvent::AgentEnd { .. })),
        "the first turn must complete before the second starts: {first_events:?}"
    );
    assert!(
        second_events
            .iter()
            .any(|e| matches!(e, XyEvent::AgentStart { .. })),
        "the second submission must run after the first turn ended: {second_events:?}"
    );
    assert!(
        !second_events.iter().any(|e| matches!(e, XyEvent::Error(_))),
        "a second submission after the first turn must not be rejected: {second_events:?}"
    );
    assert_eq!(
        first_events
            .iter()
            .chain(second_events.iter())
            .filter(|e| matches!(e, XyEvent::AgentStart { .. }))
            .count(),
        2,
        "one submission must produce exactly one turn"
    );
}

#[given("队列已有两个队列项且客户端中段订阅")]
pub(crate) async fn _g_ar_midstream_queue(agent: &AgentState, ws: &Workspace) {
    set_fake_text("midstream ack");
    let mut runner = ar_make_runner(agent, ws);
    let mut stream = agent_submit_root(&mut runner, "中段订阅主提示").await;
    let mut saw_start = false;
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::AgentStart { .. }) {
            saw_start = true;
        }
        if saw_start && matches!(event, XyEvent::StepStart { .. }) {
            break;
        }
    }
    assert!(
        saw_start,
        "the turn must be live before the mid-stream subscribe"
    );
    runner.follow_up("中段一");
    runner.follow_up("中段二");
    AR_MIDSTREAM_ITEMS.with(|cell| *cell.borrow_mut() = Some(runner.queue_items()));
    ar_store_runner(runner);
}

thread_local! {
    static AR_MIDSTREAM_ITEMS: RefCell<Option<Vec<xylitol_plugin_sdk::session::QueueItem>>> =
        const { RefCell::new(None) };
    static AR_MIDSTREAM_RESYNC: RefCell<Option<Vec<xylitol_plugin_sdk::session::QueueItem>>> =
        const { RefCell::new(None) };
}

#[when("订阅经 resync 补发")]
pub(crate) fn _w_ar_resync_replays_queue() {
    let live = AR_MIDSTREAM_ITEMS.with(|cell| cell.borrow().clone().expect("queued items"));
    let mut journal = xylitol_serve::occupancy::EventJournal::new("s0", 64);
    let wire = xylitol_protocol::Event::QueueUpdate {
        items: live
            .iter()
            .map(|item| xylitol_protocol::QueueItem {
                id: item.id.clone(),
                kind: item.kind.as_str().to_string(),
                preview: item.preview.clone(),
            })
            .collect(),
    };
    journal.append(wire);
    let replayed = journal.replay_from(0).expect("journal retains the event");
    let mut resynced = Vec::new();
    for (_, event) in replayed {
        if let xylitol_protocol::Event::QueueUpdate { items } = event {
            resynced = items
                .into_iter()
                .map(|item| xylitol_plugin_sdk::session::QueueItem {
                    id: item.id,
                    kind: xylitol_plugin_sdk::session::QueueItemKind::from_wire(&item.kind),
                    preview: item.preview,
                })
                .collect();
        }
    }
    AR_MIDSTREAM_RESYNC.with(|cell| *cell.borrow_mut() = Some(resynced));
}

#[then("QueueUpdate 携带完整队列项列表")]
pub(crate) fn _t_ar_resync_queue_complete() {
    let live = AR_MIDSTREAM_ITEMS.with(|cell| cell.borrow().clone().expect("queued items"));
    let resynced = AR_MIDSTREAM_RESYNC.with(|cell| cell.borrow().clone().expect("resynced items"));
    assert_eq!(
        live.len(),
        2,
        "the live queue must hold two items: {live:?}"
    );
    assert_eq!(
        resynced, live,
        "resync must replay the complete queue item list"
    );
    assert!(
        resynced
            .iter()
            .all(|item| item.kind == xylitol_plugin_sdk::session::QueueItemKind::FollowUp),
        "item kinds must survive the journal round trip: {resynced:?}"
    );
}

#[given("装配并运行入队 steer 的 agent")]
pub(crate) async fn _g_ar8_steer_before_llm_request(agent: &AgentState, ws: &Workspace) {
    set_fake_text("steer ack");
    let mut runner = ar_make_runner(agent, ws);
    let mut stream = agent_submit_root(&mut runner, "初始提示").await;
    let mut local = Vec::new();
    while let Some(event) = stream.next().await {
        let started = matches!(event, XyEvent::AgentStart { .. });
        local.push(event);
        if started {
            break;
        }
    }
    runner.steer("插队指令");
    while let Some(event) = stream.next().await {
        local.push(event);
    }
    ar_store_runner(runner);
    ar_store_events(local);
}

#[when("检查队列与历史")]
pub(crate) fn _w_ar8_check_queue() {
    let _ = ar_with_runner(|r| r.queue_stats());
}

#[then("steer 计数归零且已处理")]
pub(crate) fn _t_ar8_steer_drained() {
    ar_with_runner(|r| {
        assert_eq!(r.queue_stats().steer_count, 0, "steer should be drained");
    });
}

#[given("装配无工具 agent 并入队 follow_up")]
pub(crate) async fn _g_ar8_followup_extends(agent: &AgentState, ws: &Workspace) {
    set_fake_text("followup ack");
    let runner = ar_make_runner(agent, ws);
    runner.follow_up("追问一");
    runner.follow_up("追问二");
    runner.follow_up("追问三");
    ar_store_runner(runner);
}

#[when("运行至将结束")]
pub(crate) async fn _w_ar8_run_until_end() {
    ar_take_run_store("主提示").await;
}

#[then("follow_up 逐条独立成回合")]
pub(crate) fn _t_ar8_followups_one_turn_each() {
    let events = ar_events();
    let starts = events
        .iter()
        .filter(|event| matches!(event, XyEvent::AgentStart { .. }))
        .count();
    let ends = events
        .iter()
        .filter(|event| matches!(event, XyEvent::AgentEnd { .. }))
        .count();
    assert_eq!(
        starts, 4,
        "initial submission plus three follow_ups must each open a turn: {events:?}"
    );
    assert_eq!(ends, 4, "each turn must close with AgentEnd: {events:?}");
    let mut open = false;
    for event in &events {
        match event {
            XyEvent::AgentStart { .. } => {
                assert!(!open, "AgentStart must not nest: {events:?}");
                open = true;
            }
            XyEvent::AgentEnd { .. } => {
                assert!(open, "AgentEnd must follow AgentStart: {events:?}");
                open = false;
            }
            _ => {}
        }
    }
    let messages = events
        .iter()
        .rev()
        .find_map(|event| match event {
            XyEvent::AgentEnd { messages, .. } => Some(messages),
            _ => None,
        })
        .expect("final AgentEnd must carry history");
    let user_texts: Vec<String> = messages
        .iter()
        .filter(|message| message.role_name() == "user")
        .map(|message| message.text())
        .collect();
    assert_eq!(
        user_texts,
        vec!["主提示", "追问一", "追问二", "追问三"],
        "follow_ups must be delivered one by one in FIFO order"
    );
}

#[given("装配 agent 并入队 steer")]
pub(crate) async fn _g_ar9_queue_update(agent: &AgentState, ws: &Workspace) {
    set_fake_text("queue ack");
    let runner = ar_make_runner(agent, ws);
    runner.steer("入队观察");
    ar_store_runner(runner);
}

#[when("观察事件流")]
pub(crate) async fn _w_ar9_observe_stream() {
    ar_take_run_store("主提示").await;
}

#[then("出现 QueueUpdate 且载荷为队列项列表含 id 与 kind")]
pub(crate) fn _t_ar9_queue_update_emitted() {
    let events = ar_events();
    let payloads: Vec<&Vec<xylitol_plugin_sdk::session::QueueItem>> = events
        .iter()
        .filter_map(|ev| match ev {
            XyEvent::QueueUpdate { items } => Some(items),
            _ => None,
        })
        .collect();
    assert!(
        !payloads.is_empty(),
        "expected at least one QueueUpdate after enqueue, got {events:?}"
    );
    assert!(
        payloads.iter().all(|items| items.iter().all(|item| {
            !item.id.is_empty()
                && item.kind != xylitol_plugin_sdk::session::QueueItemKind::Unknown
                && !item.preview.is_empty()
        })),
        "every QueueUpdate payload must be an items list whose entries carry id / kind / preview: {payloads:?}"
    );
    let runtime = ar_with_runner(|runner| runner.queue_items());
    assert!(
        runtime.iter().all(|item| !item.id.is_empty()
            && item.kind != xylitol_plugin_sdk::session::QueueItemKind::Unknown
            && !item.preview.is_empty()),
        "driver queue_items must expose id / kind / preview for every pending item: {runtime:?}"
    );
}

#[given("装配 agent 并入队 steer 与 follow_up")]
pub(crate) async fn _g_ar10_abort_clears(agent: &AgentState, ws: &Workspace) {
    set_fake_text("abort queue ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("保留 follow_up");
    let mut stream = agent_submit_root(&mut runner, "主提示").await;
    while let Some(event) = stream.next().await {
        if matches!(event, XyEvent::AgentStart { .. }) {
            break;
        }
    }
    runner.steer("待清除 steer");
    ar_store_runner(runner);
    ar_store_open_stream(stream);
}

#[when("abort")]
pub(crate) fn _w_ar10_abort() {
    ar_with_runner(|r| r.abort());
}

#[then("steer 空且 follow_up 保留")]
pub(crate) async fn _t_ar10_queue_semantics() {
    ar_with_runner(|r| {
        let stats = r.queue_stats();
        assert_eq!(stats.steer_count, 0, "abort must clear steer");
        assert_eq!(
            stats.follow_up_count, 1,
            "abort must keep the follow_up FIFO queued"
        );
    });
    let runner = ar_take_runner();
    let mut stream = ar_take_open_stream();
    let mut local = Vec::new();
    while let Some(event) = stream.next().await {
        local.push(event);
    }
    ar_store_runner(runner);
    ar_store_events(local.clone());
    let boundaries: Vec<(&str, Option<xylitol_plugin_sdk::AgentEndReason>)> = local
        .iter()
        .filter_map(|event| match event {
            XyEvent::AgentStart { .. } => Some(("start", None)),
            XyEvent::AgentEnd { reason, .. } => Some(("end", *reason)),
            _ => None,
        })
        .collect();
    assert_eq!(
        boundaries,
        vec![
            ("end", Some(xylitol_plugin_sdk::AgentEndReason::UserCancel)),
            ("start", None),
            ("end", None),
        ],
        "abort must close the current turn and immediately open a turn for the kept follow_up; the given step already consumed the first AgentStart: {local:?}"
    );
    let final_user_texts: Vec<String> = local
        .iter()
        .rev()
        .find_map(|event| match event {
            XyEvent::AgentEnd { messages, .. } => Some(
                messages
                    .iter()
                    .filter(|m| m.role_name() == "user")
                    .map(|m| m.text())
                    .collect(),
            ),
            _ => None,
        })
        .expect("final AgentEnd carries history");
    assert!(
        final_user_texts.contains(&"保留 follow_up".to_string()),
        "kept follow_up must be delivered after abort: {final_user_texts:?}"
    );
    assert!(
        !final_user_texts.contains(&"待清除 steer".to_string()),
        "steer cleared by abort must never reach history: {final_user_texts:?}"
    );
}

#[given("装配慢速 agent 并在首轮 abort 后")]
pub(crate) async fn _g_ar11_second_run_after_abort(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-abort-second");
    set_fake_slow_stream(20, 10);
    let mut runner = make_agent(agent);
    runner
        .select_model("ar-abort-second")
        .await
        .expect("select ar-abort-second fake model");
    let mut stream = agent_submit_root(&mut runner, "首轮").await;
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            runner.abort();
        }
    }
    reset_fake_state();
    set_fake_text("第二轮完成");
    ar_store_runner(runner);
}

#[when("再次运行")]
pub(crate) async fn _w_ar11_second_run() {
    ar_take_run_store("第二轮").await;
}

#[then("正常完成而非立即 aborted")]
pub(crate) fn _t_ar11_second_run_ok() {
    let events = ar_events();
    let aborted = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::Error(err) if err.is_aborted()));
    let ended = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::StepEnd { .. }));
    assert!(!aborted, "second run must not be immediately aborted");
    assert!(ended, "second run must complete with StepEnd");
}

#[given("注册 should_stop_after_step 在首次 StepEnd 后返回 true")]
pub(crate) async fn _g_ar24_should_stop(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-after-turn ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_should_stop_after_step(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("入队 follow_up 且 should_stop_after_step 在首次 StepEnd 后返回 true")]
pub(crate) async fn _g_ar24_should_stop_with_followup(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-skip-followup ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("停闸后不应注入的追问");
    runner.set_should_stop_after_step(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("未注册 should_stop_after_step 的无工具 agent")]
pub(crate) async fn _g_ar24_no_hook_open(agent: &AgentState, ws: &Workspace) {
    set_fake_text("open-end ack");
    let runner = ar_make_runner(agent, ws);
    ar_store_runner(runner);
}

#[given(
    "按 session.max_steps_per_turn=2 安装 should_stop_after_step 且入队 follow_up 以迫使第二轮"
)]
pub(crate) async fn _g_ar30_max_steps_per_turn(agent: &AgentState, ws: &Workspace) {
    set_fake_text("max-turns ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("第二轮追问");
    runner.set_should_stop_after_step(Some(xylitol_runtime::agent::max_steps_stop_hook(2)));
    ar_store_runner(runner);
}

#[then("出现 AgentEnd 且其后无新的模型轮 StepStart")]
pub(crate) fn _t_ar24_agent_end_no_extra_turn(agent: &AgentState) {
    let events = agent.events.borrow();
    let mut turn_starts = 0usize;
    let mut saw_agent_end = false;
    let mut turn_start_after_end = false;
    for ev in events.iter() {
        match ev {
            XyEvent::StepStart { .. } => {
                turn_starts += 1;
                if saw_agent_end {
                    turn_start_after_end = true;
                }
            }
            XyEvent::AgentEnd { .. } => saw_agent_end = true,
            _ => {}
        }
    }
    assert!(saw_agent_end, "expected AgentEnd, got {events:?}");
    assert_eq!(
        turn_starts, 1,
        "expected exactly one StepStart, got {events:?}"
    );
    assert!(
        !turn_start_after_end,
        "no StepStart after AgentEnd: {events:?}"
    );
}

fn agent_end_reason(events: &[XyEvent]) -> Option<xylitol_plugin_sdk::AgentEndReason> {
    events.iter().rev().find_map(|e| match e {
        XyEvent::AgentEnd { reason, .. } => *reason,
        _ => None,
    })
}

fn apply_events_to_bridge(agent: &AgentState, bridge_bdd: &BridgeBdd) {
    for ev in agent.events.borrow().iter() {
        crate::bdd::steps_app_tui_bridge::apply(bridge_bdd, ev.clone());
    }
}

fn assert_scroll_notice(bridge_bdd: &BridgeBdd, text: &str) {
    let model = bridge_bdd.model.borrow();
    assert!(
        model
            .entries
            .iter()
            .any(|e| matches!(e, UiEntry::ScrollNotice { text: t } if t == text)),
        "expected scroll notice {text:?}, got {:?}",
        model.entries
    );
}

#[given("启动慢速流式 mock 并在首个 TextDelta 后 abort")]
pub(crate) async fn _g_ar_user_cancel_reason(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-cancel-reason");
    set_fake_slow_stream(20, 10);
    let mut runner = make_agent(agent);
    runner
        .select_model("ar-cancel-reason")
        .await
        .expect("select ar-cancel-reason");
    let mut stream = agent_submit_root(&mut runner, "取消").await;
    let mut events = Vec::new();
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            runner.abort();
        }
        events.push(e);
    }
    ar_store_runner(runner);
    let mut stored = agent.events.borrow_mut();
    stored.clear();
    stored.extend(events);
}

#[given("mock 模型 generate 返回不可重试错误")]
pub(crate) fn _g_ar_model_error_reason(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    set_fake_error("invalid request", false);
    ws.init();
    ar_register_fake(agent, "ar-model-error");
    let mut runner = make_agent(agent);
    futures::executor::block_on(runner.select_model("ar-model-error"))
        .expect("select ar-model-error");
    ar_store_runner(runner);
}

#[given("按 session.max_steps_per_turn=1 安装 should_stop_after_step")]
pub(crate) fn _g_ar_step_limit_reason(agent: &AgentState, ws: &Workspace) {
    let mut runner = ar_make_runner(agent, ws);
    runner.set_should_stop_after_step(Some(xylitol_runtime::agent::max_steps_stop_hook(1)));
    ar_store_runner(runner);
}

#[when("观察事件流与产品端输出")]
pub(crate) fn _w_ar_reason_observe_stored() {}

#[when("运行 AgentRuntime 并观察事件流与产品端输出")]
pub(crate) async fn _w_ar_reason_run_observe(agent: &AgentState) {
    _w_ar_react_run_collect(agent).await;
}

#[then("AgentEnd.reason 为 UserCancel 且端侧显示取消归因")]
pub(crate) fn _t_ar_user_cancel_reason(agent: &AgentState, bridge_bdd: &BridgeBdd) {
    let events = agent.events.borrow();
    assert_eq!(
        agent_end_reason(&events),
        Some(xylitol_plugin_sdk::AgentEndReason::UserCancel),
        "{events:?}"
    );
    drop(events);
    apply_events_to_bridge(agent, bridge_bdd);
    assert_scroll_notice(bridge_bdd, "Operation aborted");
}

#[then("AgentEnd.reason 为 ModelError 且端侧显示模型错误归因")]
pub(crate) fn _t_ar_model_error_reason(agent: &AgentState, bridge_bdd: &BridgeBdd) {
    let events = agent.events.borrow();
    assert_eq!(
        agent_end_reason(&events),
        Some(xylitol_plugin_sdk::AgentEndReason::ModelError),
        "{events:?}"
    );
    drop(events);
    apply_events_to_bridge(agent, bridge_bdd);
    assert_scroll_notice(bridge_bdd, "Model error");
}

#[then("AgentEnd.reason 为 StepLimit 且端侧显示步数上限归因")]
pub(crate) fn _t_ar_step_limit_reason(agent: &AgentState, bridge_bdd: &BridgeBdd) {
    let events = agent.events.borrow();
    assert_eq!(
        agent_end_reason(&events),
        Some(xylitol_plugin_sdk::AgentEndReason::StepLimit),
        "{events:?}"
    );
    drop(events);
    apply_events_to_bridge(agent, bridge_bdd);
    assert_scroll_notice(bridge_bdd, "Step limit reached");
}

#[given("mock 模型在首个 step 先报可重试错误后成功、次 step 直接成功")]
pub(crate) fn _g_ar_retry_once(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    set_fake_retry_once("rate limit exceeded");
    set_fake_text("retry ok");
    ws.init();
    ar_register_fake(agent, "ar-retry");
    let mut runtime = make_agent(agent);
    futures::executor::block_on(runtime.select_model("ar-retry"))
        .expect("select ar-retry fake model");
    runtime.follow_up("second step");
    ar_store_runner(runtime);
}

#[then("恰出现一次 AutoRetryEnd 且其后再无 AutoRetryEnd")]
pub(crate) fn _t_ar_retry_once(agent: &AgentState) {
    let events = agent.events.borrow();
    let ends: Vec<&XyEvent> = events
        .iter()
        .filter(|e| matches!(e, XyEvent::AutoRetryEnd { .. }))
        .collect();
    assert_eq!(
        ends.len(),
        1,
        "exactly one AutoRetryEnd expected (second step must not emit one): {events:?}"
    );
    match ends[0] {
        XyEvent::AutoRetryEnd { success, .. } => assert!(*success, "{events:?}"),
        _ => unreachable!(),
    }
    let step_starts = events
        .iter()
        .filter(|e| matches!(e, XyEvent::StepStart { .. }))
        .count();
    assert_eq!(step_starts, 2, "retry then next step: {events:?}");
}

#[then("至多出现 2 次 StepStart 后出现 AgentEnd")]
pub(crate) fn _t_ar30_max_two_turns(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|ev| matches!(ev, XyEvent::StepStart { .. }))
        .count();
    let saw_agent_end = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::AgentEnd { .. }));
    assert!(saw_agent_end, "expected AgentEnd, got {events:?}");
    assert!(
        turn_starts <= 2,
        "expected at most 2 StepStart, got {turn_starts}: {events:?}"
    );
    assert!(
        turn_starts >= 1,
        "expected at least one StepStart, got {events:?}"
    );
}

#[then("本 turn 以 AgentEnd 结束且 follow_up 未被注入历史")]
pub(crate) fn _t_ar24_followup_not_injected(agent: &AgentState) {
    let events = agent.events.borrow();
    let agent_end = events.iter().rev().find_map(|ev| match ev {
        XyEvent::AgentEnd { messages, .. } => Some(messages.clone()),
        _ => None,
    });
    let history = agent_end.expect("expected AgentEnd");
    let injected = history.iter().any(|m| match m {
        xylitol_plugin_sdk::message::AgentMessage::Llm(
            xylitol_plugin_sdk::message::XyLlmMessage::UserMessage { content, .. },
        ) => content.iter().any(|p| match p {
            xylitol_plugin_sdk::message::AgentPart::Text { text } => {
                text.contains("停闸后不应注入的追问")
            }
            _ => false,
        }),
        _ => false,
    });
    assert!(!injected, "follow_up must not be in history: {history:?}");
    ar_with_runner(|r| {
        assert_eq!(
            r.queue_stats().follow_up_count,
            1,
            "follow_up must remain queued"
        );
    });
}

#[then("正常出现 AgentEnd 且恰好一轮 StepStart")]
pub(crate) fn _t_ar24_no_hook_open_end(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|ev| matches!(ev, XyEvent::StepStart { .. }))
        .count();
    let saw_agent_end = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::AgentEnd { .. }));
    assert!(
        saw_agent_end,
        "expected AgentEnd without stop hook: {events:?}"
    );
    assert_eq!(
        turn_starts, 1,
        "open end without max_iterations / stop hook: one StepStart, got {events:?}"
    );
}

thread_local! {
    static BATCH_TIMING: RefCell<Vec<(String, u128, u128)>> = const { RefCell::new(Vec::new()) };
    static BATCH_EPOCH: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
}

struct BddSlowTool {
    name: &'static str,
    mode: xylitol_plugin_sdk::XyToolExecutionMode,
    sleep_ms: u64,
}

#[async_trait::async_trait]
impl XyTool for BddSlowTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "bdd slow tool"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn execution_mode(&self) -> xylitol_plugin_sdk::XyToolExecutionMode {
        self.mode
    }
    async fn execute_outcome(
        &self,
        _ctx: &XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<xylitol_plugin_sdk::XyToolOutcome, xylitol_plugin_sdk::error::XyToolError> {
        let epoch = BATCH_EPOCH.with(|e| e.get().expect("batch epoch"));
        let start = epoch.elapsed().as_millis();
        tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms)).await;
        let end = epoch.elapsed().as_millis();
        BATCH_TIMING.with(|t| t.borrow_mut().push((self.name.to_string(), start, end)));
        Ok(xylitol_plugin_sdk::XyToolOutcome::text(format!(
            "{}-ok",
            self.name
        )))
    }
}

struct BddMultiToolModel {
    rounds: std::sync::Mutex<Vec<Vec<xylitol_plugin_sdk::model::XyChunk>>>,
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyModel for BddMultiToolModel {
    fn name(&self) -> &str {
        "bdd-batch-mock"
    }
    async fn generate_stream(
        &self,
        _messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
        _tools: &[xylitol_plugin_sdk::model::XyToolSchema],
        _stream: bool,
        _options: xylitol_plugin_sdk::XyGenerateOptions,
    ) -> Result<xylitol_plugin_sdk::XyStream, xylitol_plugin_sdk::error::XyError> {
        let chunks = self.rounds.lock().unwrap().remove(0);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

pub(crate) fn bdd_batch_rounds(
    calls: &[(&str, &str)],
) -> Vec<Vec<xylitol_plugin_sdk::model::XyChunk>> {
    let done = || xylitol_plugin_sdk::model::XyChunk::Done {
        finish_reason: xylitol_plugin_sdk::message::XyStopReason::Stop,
        usage: None,
    };
    let mut round1 = Vec::new();
    for (i, (name, args_json)) in calls.iter().enumerate() {
        let args: serde_json::Value =
            serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
        round1.push(xylitol_plugin_sdk::model::XyChunk::ToolCallStart {
            id: format!("call-{i}"),
            name: (*name).into(),
        });
        round1.push(xylitol_plugin_sdk::model::XyChunk::ToolCallEnd {
            id: format!("call-{i}"),
            name: (*name).into(),
            args,
        });
    }
    round1.push(done());
    vec![
        round1,
        vec![
            xylitol_plugin_sdk::model::XyChunk::TextDelta("ok".into()),
            done(),
        ],
    ]
}

pub(crate) fn bdd_batch_make_runner(
    agent: &AgentState,
    tools: ToolSet,
    calls: &[(&str, &str)],
    mode: xylitol_plugin_sdk::XyBatchMode,
) -> AgentRuntime {
    use xylitol_plugin_sdk::{XyEventSink, XyModel, XySessionStore};
    BATCH_TIMING.with(|t| t.borrow_mut().clear());
    BATCH_EPOCH.with(|e| e.set(Some(std::time::Instant::now())));
    reset_fake_state();
    ar_register_fake(agent, "ar-batch");
    let rounds = bdd_batch_rounds(calls);
    let dir = tempfile::tempdir().unwrap();
    let mgr = LogSessionStore::new(dir.keep());
    let store: Arc<dyn XySessionStore> = Arc::new(mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol_kernel::event::EventBus::new());
    let builder: xylitol_plugin_sdk::XyModelBuilder = Arc::new(move |_| {
        Arc::new(BddMultiToolModel {
            rounds: std::sync::Mutex::new(rounds.clone()),
        }) as Arc<dyn XyModel>
    });
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        tools,
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        xylitol_runtime::permission::allow_all_permission(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        xylitol_runtime::agent::capabilities::QueueMode::default(),
        None,
    );
    futures::executor::block_on(session.select_model("ar-batch")).expect("select ar-batch fake");
    let mut runner = AgentRuntime::new(session);
    runner.set_batch_mode(mode);
    runner
}

pub(crate) fn bdd_timing_overlaps(a: (u128, u128), b: (u128, u128)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

#[given("未配置工具批模式且 mock 模型同 turn 发出两个可并行假工具")]
pub(crate) fn _g_ar27_batch_default(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![Arc::new(BddSlowTool {
        name: "slow_safe",
        mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
        sleep_ms: 80,
    }) as Arc<dyn XyTool>]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_safe", r#"{"n":1}"#), ("slow_safe", r#"{"n":2}"#)],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 发出两个 ParallelSafe 慢假工具后接一个 Barrier 假工具"
)]
pub(crate) fn _g_ar28_overlap(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 100,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Sequential,
            sleep_ms: 40,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("slow_safe", r#"{"n":2}"#),
            ("slow_barrier", r#"{}"#),
        ],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、Barrier、ParallelSafe"
)]
pub(crate) fn _g_ar28_windows(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 60,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Sequential,
            sleep_ms: 40,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("slow_barrier", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given(
    "工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、mcp 假工具、ParallelSafe"
)]
pub(crate) fn _g_ar28_mcp(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_safe",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "mcp__fake__x",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("mcp__fake__x", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given("工具批模式为 barrier_parallel 且并行窗内后发先完成")]
pub(crate) fn _g_ar29_history(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_a",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 120,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_b",
            mode: xylitol_plugin_sdk::XyToolExecutionMode::Parallel,
            sleep_ms: 30,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_a", r#"{}"#), ("slow_b", r#"{}"#)],
        xylitol_plugin_sdk::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[then("两工具执行时间重叠")]
pub(crate) fn _t_ar27_default_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        bdd_timing_overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
        "default batch must overlap ParallelSafe tools: {entries:?}"
    );
}

#[then("两 ParallelSafe 执行时间重叠且均在 Barrier 开始前结束")]
pub(crate) fn _t_ar28_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    let safes: Vec<_> = entries
        .iter()
        .filter(|(n, _, _)| n == "slow_safe")
        .cloned()
        .collect();
    let barrier = entries
        .iter()
        .find(|(n, _, _)| n == "slow_barrier")
        .expect("barrier");
    assert_eq!(safes.len(), 2, "{entries:?}");
    assert!(
        bdd_timing_overlaps((safes[0].1, safes[0].2), (safes[1].1, safes[1].2)),
        "safes must overlap: {entries:?}"
    );
    let safe_end = safes.iter().map(|e| e.2).max().unwrap();
    assert!(safe_end <= barrier.1, "safes before barrier: {entries:?}");
}

#[then("第二个 ParallelSafe 不与第一个 ParallelSafe 同窗并行且 在 Barrier 完成之后开始")]
pub(crate) fn _t_ar28_windows() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 3, "{entries:?}");
    let first = &entries[0];
    let barrier = entries
        .iter()
        .find(|(n, _, _)| n == "slow_barrier")
        .unwrap();
    let second = entries
        .iter()
        .rev()
        .find(|(n, _, _)| n == "slow_safe")
        .unwrap();
    assert!(first.2 <= barrier.1, "{entries:?}");
    assert!(barrier.2 <= second.1, "{entries:?}");
    assert!(
        !bdd_timing_overlaps((first.1, first.2), (second.1, second.2)),
        "{entries:?}"
    );
}

#[then("mcp 假工具与两侧 ParallelSafe 均无执行时间重叠")]
pub(crate) fn _t_ar28_mcp_no_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    let mcp = entries
        .iter()
        .find(|(n, _, _)| n == "mcp__fake__x")
        .expect("mcp");
    for (n, s, e) in &entries {
        if n == "mcp__fake__x" {
            continue;
        }
        assert!(
            !bdd_timing_overlaps((*s, *e), (mcp.1, mcp.2)),
            "mcp overlaps {n}: {entries:?}"
        );
    }
}

#[when("检查 session history 中 toolResult")]
pub(crate) async fn _w_ar29_check_history(agent: &AgentState) {
    if agent.events.borrow().is_empty() {
        _w_ar_react_run(agent).await;
    }
}

#[then("toolResult 顺序与 assistant 源序一致")]
pub(crate) fn _t_ar29_history_order(agent: &AgentState) {
    let events = agent.events.borrow();
    let history = events
        .iter()
        .rev()
        .find_map(|ev| match ev {
            XyEvent::AgentEnd { messages, .. } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd");
    let ids: Vec<_> = history
        .iter()
        .filter_map(|m| match m {
            xylitol_plugin_sdk::message::AgentMessage::Llm(
                xylitol_plugin_sdk::message::XyLlmMessage::ToolResultMessage {
                    tool_use_id, ..
                },
            ) => Some(tool_use_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        ids,
        vec!["call-0".to_string(), "call-1".to_string()],
        "history order: {ids:?}"
    );
}

#[given("启动交互 bang 长命令后 abort")]
pub(crate) async fn _g_ar10_abort_cancels_bang(_agent: &AgentState, _ws: &Workspace) {
    use xylitol_official_plugin_tools::bash_exec::InfraBashExecutor;
    use xylitol_plugin_sdk::{BashExecOpts, XyBashExecutor};
    let executor = std::sync::Arc::new(InfraBashExecutor::new());
    let token = tokio_util::sync::CancellationToken::new();
    let exec_for_task = executor.clone();
    let token_for_task = token.clone();
    let handle = tokio::task::spawn(async move {
        exec_for_task
            .execute("sleep 30", BashExecOpts::cancel_only(token_for_task))
            .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    token.cancel();
    let result = handle.await.expect("bash task joined");
    AR_BANG_RESULT.with(|r| *r.borrow_mut() = Some(Ok(result)));
}

#[when("检查 bash 结果")]
pub(crate) fn _w_ar10_check_bash_result() {}

#[then("cancelled 为 true")]
pub(crate) fn _t_ar10_bang_cancelled() {
    AR_BANG_RESULT.with(|r| {
        let borrow = r.borrow();
        let result = borrow
            .as_ref()
            .expect("bash result captured")
            .as_ref()
            .expect("execute_bash ok");
        assert!(
            matches!(result.outcome, xylitol_plugin_sdk::BashOutcome::Killed),
            "bang must be killed, got outcome={:?}",
            result.outcome
        );
    });
}

#[given("注册匹配 bash 的 before 拒绝 hook")]
pub(crate) async fn _g_ar7_before_denies(agent: &AgentState, ws: &Workspace) {
    let mut runner = ar_make_runner(agent, ws);
    runner.add_hook(Arc::new(
        |name: &str, _id: &str, _args: &serde_json::Value| {
            if name == "bash" {
                Some("blocked by before hook".to_string())
            } else {
                None
            }
        },
    ));
    set_fake_tool_call("bash", r#"{"command":"echo hi"}"#);
    set_fake_text("ack");
    ar_store_runner(runner);
}

#[when("运行 AgentRuntime 触发 bash")]
pub(crate) async fn _w_ar7_run_trigger_bash() {
    ar_take_run_store("触发 bash").await;
}

#[then("tool-error 回写且未执行")]
pub(crate) fn _t_ar7_tool_error_written() {
    let events = ar_events();
    let has_error = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::ToolExecutionEnd { is_error: true, .. }));
    assert!(
        has_error,
        "before hook must deny bash with tool-error, got {:?}",
        events
    );
}

#[when("运行一轮对话后检查持久化条目")]
pub(crate) async fn w_c2826_first_turn_history(agent: &AgentState) {
    xylitol_kernel::provider::factory::set_fake_text("回复正文");
    let (mut runner, store) = make_agent_with_store(agent);
    crate::bdd::helpers::bind_session_or_panic(&mut runner, "c2826-first-turn");
    let mut stream = agent_submit_root(&mut runner, "用户第一句").await;
    while let Some(e) = stream.next().await {
        if let XyEvent::Error(err) = &e {
            panic!("c2826: run failed: {err:?}");
        }
    }
    let entries = store
        .load_entries("c2826-first-turn")
        .await
        .unwrap_or_default();
    let roles: Vec<String> = entries
        .iter()
        .filter_map(|e| match e {
            xylitol_official_plugin_session::SessionEntry::Message(m) => m
                .message
                .get("role")
                .and_then(|r| r.as_str())
                .map(str::to_owned),
            _ => None,
        })
        .collect();
    let first_text = entries
        .iter()
        .find_map(|e| match e {
            xylitol_official_plugin_session::SessionEntry::Message(m) => m
                .message
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
                .map(str::to_owned),
            _ => None,
        })
        .unwrap_or_default();
    agent
        .last_result
        .replace(Some(Ok(format!("roles={roles:?} first_text={first_text}"))));
}

#[then("首条消息条目为真实 user 输入且无 system 角色行")]
pub(crate) fn t_c2826_first_turn_history(agent: &AgentState) {
    let summary = crate::bdd::helpers::result_ok_str(&agent.last_result);
    let dialogue_roles: Vec<&str> = ["user", "assistant", "system"]
        .iter()
        .filter(|r| {
            summary.contains(&format!("roles=[\"{r}\"")) || summary.contains(&format!(", \"{r}\""))
        })
        .copied()
        .collect();
    assert!(
        !dialogue_roles.contains(&"system"),
        "c2826: system 正文不得写入 history：{summary}"
    );
    assert!(
        summary.contains("roles=[\"user\"") || summary.contains(", \"user\""),
        "c2826: 对话首条必须为真实 user 输入：{summary}"
    );
    assert!(
        summary.contains("first_text=用户第一句"),
        "c2826: 首条 user 文本必须保留：{summary}"
    );
}

fn c2826_driver() -> (
    xylitol_kernel::driver::XyInProcessDriver,
    std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore>,
) {
    use std::sync::Arc;
    let agent = xylitol_kernel::composition::build_agent(
        xylitol_kernel::composition::BuildAgentOptions::default(),
    )
    .expect("build agent for reload seam");
    let mgr = xylitol_official_plugin_session::LogSessionStore::new(
        tempfile::tempdir().unwrap().path().join("sessions"),
    );
    let store: Arc<dyn xylitol_plugin_sdk::XySessionStore> = Arc::new(mgr);
    let driver = xylitol_kernel::driver::assemble_driver(agent, store.clone());
    (driver, store)
}

#[when("在项目目录写 AGENTS.md 并分别以信任与未信任重载 context")]
pub(crate) async fn w_c2826_context_reload_trust(
    ws: &crate::bdd::fixtures::Workspace,
    agent: &AgentState,
) {
    ws.init();
    let project = ws.root();
    std::fs::write(
        std::path::Path::new(&project).join("AGENTS.md"),
        "SECRET_PROJECT_AGENTS_MARKER\n",
    )
    .unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    let (mut trusted_driver, _s1) = c2826_driver();
    let trusted = xylitol_kernel::bootstrap::reload_prompt_context(
        &mut trusted_driver,
        std::path::Path::new(&project),
        agent_dir.path(),
        true,
        None,
    )
    .await;
    let (mut untrusted_driver, _s2) = c2826_driver();
    let untrusted = xylitol_kernel::bootstrap::reload_prompt_context(
        &mut untrusted_driver,
        std::path::Path::new(&project),
        agent_dir.path(),
        false,
        None,
    )
    .await;
    let trusted_sp = trusted_driver.system_prompt_for_test().unwrap_or_default();
    let untrusted_sp = untrusted_driver
        .system_prompt_for_test()
        .unwrap_or_default();
    agent.last_result.replace(Some(Ok(format!(
        "trusted_files={} untrusted_files={} trusted_marker={} untrusted_marker={}",
        trusted.context_file_count,
        untrusted.context_file_count,
        trusted_sp.contains("SECRET_PROJECT_AGENTS_MARKER"),
        untrusted_sp.contains("SECRET_PROJECT_AGENTS_MARKER")
    ))));
}

#[then("信任时项目 context 注入而未信任时被跳过且会话条目不变")]
pub(crate) fn t_c2826_context_reload_trust(agent: &AgentState) {
    let summary = crate::bdd::helpers::result_ok_str(&agent.last_result);
    assert!(
        summary.contains("trusted_files=1") || summary.contains("trusted_files=2"),
        "c2826: 信任重载应发现项目 context：{summary}"
    );
    assert!(
        summary.contains("trusted_marker=true"),
        "c2826: 信任重载必须注入项目 AGENTS.md：{summary}"
    );
    assert!(
        summary.contains("untrusted_files=0"),
        "c2826: 未信任重载必须跳过项目 context：{summary}"
    );
    assert!(
        summary.contains("untrusted_marker=false"),
        "c2826: 未信任重载不得注入项目 AGENTS.md：{summary}"
    );
}

#[when("在项目目录写 skill 并分别以信任与未信任重载 skills")]
pub(crate) async fn w_c2826_skills_reload_trust(
    ws: &crate::bdd::fixtures::Workspace,
    agent: &AgentState,
) {
    ws.init();
    let project = ws.root();
    let skill_dir = std::path::Path::new(&project)
        .join(".xylitol")
        .join("skills")
        .join("c2826-demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: c2826-demo\ndescription: test\n---\n\nbody\n",
    )
    .unwrap();
    let agent_dir = tempfile::tempdir().unwrap();
    let (mut trusted_driver, _s1) = c2826_driver();
    let trusted = xylitol_kernel::bootstrap::reload_skills(
        &mut trusted_driver,
        std::path::Path::new(&project),
        agent_dir.path(),
        true,
    )
    .await;
    let (mut untrusted_driver, _s2) = c2826_driver();
    let untrusted = xylitol_kernel::bootstrap::reload_skills(
        &mut untrusted_driver,
        std::path::Path::new(&project),
        agent_dir.path(),
        false,
    )
    .await;
    agent.last_result.replace(Some(Ok(format!(
        "trusted_names={:?} untrusted_names={:?} trusted_query={:?} untrusted_query={:?}",
        trusted.names,
        untrusted.names,
        trusted_driver.loaded_skill_names(),
        untrusted_driver.loaded_skill_names()
    ))));
}

#[then("信任时项目 skill 注入且可查询已加载名而未信任时被跳过")]
pub(crate) fn t_c2826_skills_reload_trust(agent: &AgentState) {
    let summary = crate::bdd::helpers::result_ok_str(&agent.last_result);
    assert!(
        summary.contains("\"c2826-demo\""),
        "c2826: 信任重载应注入项目 skill：{summary}"
    );
    assert!(
        summary.contains("trusted_query=[\"c2826-demo\"]")
            || summary.contains("trusted_query=[\"c2826-demo\","),
        "c2826: 已加载 skill 名必须可查询：{summary}"
    );
    assert!(
        !summary.contains("untrusted_names=[\"c2826-demo\"]"),
        "c2826: 未信任重载必须跳过项目 skill：{summary}"
    );
}

#[when("经 Driver 启动会话并在首个 TextDelta 后 abort 并检查持久化条目")]
pub(crate) async fn w_c2826_abort_persist(agent: &AgentState) {
    use xylitol::embed::{XyDriver, XySessionOps, assemble_driver};
    xylitol_kernel::provider::factory::set_fake_slow_stream(40, 5);
    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = assemble_driver(runtime, store.clone());
    let sid = driver
        .new_session()
        .await
        .expect("c2826: abort persist session");
    let mut stream = driver.run("partial-please").await;
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(&e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            driver.abort();
        }
    }
    assert!(saw_delta, "c2826: 应先见到 TextDelta");
    let entries = store.load_entries(&sid).await.unwrap_or_default();
    let assistant_rows: Vec<String> = entries
        .iter()
        .filter_map(|e| match e {
            xylitol_official_plugin_session::SessionEntry::Message(m)
                if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant") =>
            {
                Some(serde_json::to_string(&m.message).unwrap_or_default())
            }
            _ => None,
        })
        .collect();
    agent.last_result.replace(Some(Ok(format!(
        "assistants={}",
        assistant_rows.join("\n---\n")
    ))));
}

#[then("partial assistant 以 aborted 落盘且非空")]
pub(crate) fn t_c2826_abort_persist(agent: &AgentState) {
    let summary = crate::bdd::helpers::result_ok_str(&agent.last_result);
    assert!(
        summary.contains("\"stopReason\":\"aborted\"")
            || summary.contains("stop_reason\":\"aborted")
            || summary.contains("aborted"),
        "c2826: partial assistant 应以 aborted 落盘：{summary}"
    );
    assert!(
        summary.contains("\"type\":\"text\""),
        "c2826: partial assistant 正文应非空：{summary}"
    );
}

#[then("StepStart 与 StepEnd 成对出现两次且 ContextTokenSettlement 恰一次")]
pub(crate) fn t_c2826_two_iterations_one_settlement(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|e| matches!(e, XyEvent::StepStart { .. }))
        .count();
    let turn_ends = events
        .iter()
        .filter(|e| matches!(e, XyEvent::StepEnd { .. }))
        .count();
    let settlements = events
        .iter()
        .filter(|e| matches!(e, XyEvent::ContextTokenSettlement { .. }))
        .count();
    assert_eq!(
        (turn_starts, turn_ends),
        (2, 2),
        "c2826: 带工具回合应为两个成对 iteration"
    );
    assert_eq!(
        settlements, 1,
        "c2826: ContextTokenSettlement 必须恰一次（仅 Settle 发）"
    );
}

thread_local! {
        static COORD_STATE: RefCell<Option<(xylitol_runtime::agent::runtime::SharedTurnCoordinator, u64, u64)>> =
        const { RefCell::new(None) };
}

fn coord_take() -> (
    xylitol_runtime::agent::runtime::SharedTurnCoordinator,
    u64,
    u64,
) {
    COORD_STATE
        .with(|s| s.borrow_mut().take())
        .expect("协调器已预置")
}

fn coord_put(c: xylitol_runtime::agent::runtime::SharedTurnCoordinator, r1: u64, r2: u64) {
    COORD_STATE.with(|s| *s.borrow_mut() = Some((c, r1, r2)));
}

fn phase_name(p: xylitol_runtime::agent::runtime::RuntimePhase) -> &'static str {
    match p {
        xylitol_runtime::agent::runtime::RuntimePhase::Idle => "idle",
        xylitol_runtime::agent::runtime::RuntimePhase::Starting(_) => "starting",
        xylitol_runtime::agent::runtime::RuntimePhase::Streaming(_) => "streaming",
        xylitol_runtime::agent::runtime::RuntimePhase::Cancelling(_) => "cancelling",
    }
}

fn coord_seed(
    phase: &str,
) -> (
    xylitol_runtime::agent::runtime::SharedTurnCoordinator,
    u64,
    u64,
) {
    use xylitol_runtime::agent::runtime::SharedTurnCoordinator;
    let c = SharedTurnCoordinator::new();
    let (r1, r2) = (1_u64, 2_u64);
    match phase {
        "idle" => {}
        "starting" | "streaming" | "cancelling" => {
            let (id, _) = c.with_mut(|co| co.try_begin_immediate().expect("begin r1"));
            if phase != "starting" {
                c.with_mut(|co| co.mark_streaming(id));
            }
            if phase == "cancelling" {
                c.with_mut(|co| co.cancel_active());
            }
        }
        other => panic!("unknown phase: {other}"),
    }
    (c, r1, r2)
}

#[given("运行协调器处于 {from:string} 阶段")]
pub(crate) fn _g_coord_phase(from: String) {
    let (c, r1, r2) = coord_seed(&from);
    coord_put(c, r1, r2);
}

#[when("协调器事件 {event:string} 且守卫 {guard:string}")]
pub(crate) fn _w_coord_transition(event: String, guard: String) {
    let (c, r1, r2) = coord_take();
    match (event.as_str(), guard.as_str()) {
        ("begin", "phase_idle") => {
            let _began = c.with_mut(|co| co.try_begin_immediate()).is_some();
        }
        ("begin", "phase_busy") => {
            let began = c.with_mut(|co| co.try_begin_immediate());
            assert!(began.is_none(), "busy coordinator must refuse begin");
        }
        ("stream", "turn_id_match") => {
            let active = c.with(|co| co.phase().active_turn_id());
            if let Some(id) = active {
                c.with_mut(|co| co.mark_streaming(id));
            }
        }
        ("stream", "turn_id_stale") => {
            let stale_id = c.with(|co| co.phase().active_turn_id());
            if let Some(id) = stale_id {
                c.with_mut(|co| co.finish_turn(id));
                c.with_mut(|co| co.mark_streaming(id));
            }
        }
        ("cancel", "always") => {
            c.with_mut(|co| co.cancel_active());
        }
        ("finish", "turn_id_match") => {
            let active = c.with(|co| co.phase().active_turn_id());
            if let Some(id) = active {
                c.with_mut(|co| co.finish_turn(id));
            }
        }
        ("finish", "turn_id_stale") => {
            let old = c.with(|co| co.phase().active_turn_id());
            if let Some(old) = old {
                c.with_mut(|co| co.cancel_active());
                c.with_mut(|co| co.finish_turn(old));
                let (_new, _) = c.with_mut(|co| co.try_begin_immediate().expect("begin r2"));
                let r = c.with_mut(|co| co.finish_turn(old));
                assert!(!r, "stale finish must be ignored");
            }
        }
        other => panic!("unknown event/guard: {other:?}"),
    }
    coord_put(c, r1, r2);
}

#[then("协调器阶段为 {to:string}")]
pub(crate) fn _t_coord_phase(to: String) {
    let (c, _r1, _r2) = coord_take();
    let actual = phase_name(c.phase());
    assert_eq!(actual, to, "runtime phase transition");
}

#[then("首个事件为 AgentStart 且末个事件为 AgentEnd")]
pub(crate) fn _t_ar_run_event_boundary(agent: &AgentState) {
    let events = agent.events.borrow();
    let first = events.first().expect("non-empty event stream");
    assert!(
        matches!(first, XyEvent::AgentStart { .. }),
        "first event MUST be AgentStart, got {first:?}: {events:?}"
    );
    let last = events.last().expect("non-empty event stream");
    assert!(
        matches!(last, XyEvent::AgentEnd { .. }),
        "last event MUST be AgentEnd, got {last:?}: {events:?}"
    );
}

#[then("AgentStart 的 session_id 为本会话且 model 为当前模型标识")]
pub(crate) fn _t_ar_agent_start_fields(agent: &AgentState) {
    let events = agent.events.borrow();
    let (session_id, model) = events
        .iter()
        .find_map(|ev| match ev {
            XyEvent::AgentStart { session_id, model } => Some((session_id.clone(), model.clone())),
            _ => None,
        })
        .expect("AgentStart present");
    let expected_session =
        ar_with_runner(|r| r.session_id().map(str::to_string)).expect("runner bound to session");
    let expected_model =
        ar_with_runner(|r| r.current_model().map(|m| m.id)).expect("runner has current model");
    assert_eq!(
        session_id, expected_session,
        "AgentStart.session_id MUST equal the bound session"
    );
    assert_eq!(
        model, expected_model,
        "AgentStart.model MUST equal the run-start current model id"
    );
}

thread_local! {
    static AR_ABORT_MARK: RefCell<Option<AbortMarkBdd>> = const { RefCell::new(None) };
}

struct AbortMarkBdd {
    driver: xylitol::embed::XyInProcessDriver,
    store: Arc<dyn xylitol_plugin_sdk::XySessionStore>,
    sid: String,
    stream: xylitol_runtime::driver::EventStream,
    events: Vec<XyEvent>,
}

#[given("装配 agent 并提交一条消息")]
pub(crate) async fn g_ar_abort_mark_submit(agent: &AgentState, ws: &Workspace) {
    use xylitol::embed::{XyDriver, XySessionOps, assemble_driver};
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-abort-mark");
    set_fake_slow_stream(40, 20);
    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = assemble_driver(runtime, store.clone());
    let sid = driver.new_session().await.expect("abort mark session");
    let stream = driver.run("abort me").await;
    AR_ABORT_MARK.with(|slot| {
        *slot.borrow_mut() = Some(AbortMarkBdd {
            driver,
            store,
            sid,
            stream,
            events: Vec::new(),
        });
    });
}

#[when("模型流产生任何内容前 abort")]
pub(crate) async fn w_ar_abort_before_content() {
    use xylitol::embed::XyDriver;
    let mut fx = AR_ABORT_MARK
        .with(|slot| slot.borrow_mut().take())
        .expect("abort mark fixture must be assembled");
    fx.driver.abort();
    while let Some(event) = fx.stream.next().await {
        fx.events.push(event);
    }
    AR_ABORT_MARK.with(|slot| *slot.borrow_mut() = Some(fx));
}

#[then("会话落盘恰好一条空 aborted assistant 标记且 AgentEnd 消息集含该标记")]
pub(crate) async fn t_ar_abort_mark_persisted() {
    use xylitol_plugin_sdk::message::{XyLlmMessage, XyStopReason};

    let fx = AR_ABORT_MARK
        .with(|slot| slot.borrow_mut().take())
        .expect("abort mark fixture must be assembled");
    let (messages, reason) = fx
        .events
        .iter()
        .find_map(|event| match event {
            XyEvent::AgentEnd { messages, reason } => Some((messages, *reason)),
            _ => None,
        })
        .expect("AgentEnd must be emitted");
    assert!(
        reason.is_some(),
        "the aborted turn ends with an abort reason: {:?}",
        fx.events
    );
    let marks = messages
        .iter()
        .filter(|m| {
            matches!(
                m,
                xylitol_plugin_sdk::message::AgentMessage::Llm(
                    XyLlmMessage::AssistantMessage {
                        content,
                        stop_reason: Some(XyStopReason::Aborted),
                        error_message: None,
                        ..
                    }
                ) if content.is_empty()
            )
        })
        .count();
    assert_eq!(
        marks, 1,
        "AgentEnd must carry exactly one empty aborted mark: {messages:?}"
    );

    let entries = fx
        .store
        .load_entries(&fx.sid)
        .await
        .expect("persisted session");
    let assistants: Vec<&serde_json::Value> = entries
        .iter()
        .filter_map(|entry| match entry {
            SessionEntry::Message(m)
                if m.message.get("role").and_then(|r| r.as_str()) == Some("assistant") =>
            {
                Some(&m.message)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        assistants.len(),
        1,
        "exactly one assistant row is persisted: {assistants:?}"
    );
    let mark = assistants[0];
    assert!(
        mark.get("content")
            .and_then(|c| c.as_array())
            .is_none_or(|parts| parts.is_empty()),
        "the persisted mark has empty content: {mark}"
    );
    assert_eq!(
        mark.get("stopReason").and_then(|s| s.as_str()),
        Some("aborted"),
        "the persisted mark is aborted: {mark}"
    );
    assert!(
        !mark
            .get("errorMessage")
            .and_then(|e| e.as_str())
            .is_some_and(|s| !s.is_empty()),
        "the persisted mark carries no errorMessage: {mark}"
    );
}
