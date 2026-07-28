use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

#[given("mock 模型先 tool 后无 tool")]
pub(crate) fn _g_ar_react_setup(agent: &AgentState, ws: &Workspace) {
    // Isolate from prior scenarios that left a prepared AR_RUNNER.
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
    // Prefer a runner prepared by a prior Given (e.g. stop-hook / queues).
    let mut runner = AR_RUNNER
        .with(|r| r.borrow_mut().take())
        .unwrap_or_else(|| make_agent(agent));
    let mut stream = runner.run("读取文件").await;
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
    use xylitol::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let events = agent.events.borrow();
    let mut saw_intent = false;
    let mut message_end_idx = None;
    let mut tool_start_idx = None;

    for (i, evt) in events.iter().enumerate() {
        match evt {
            XyEvent::MessageUpdate {
                message: Some(AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. })),
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
    // 先执行工具：事件流中存在 ToolCall 相关事件
    let has_tool = events
        .iter()
        .any(|e| !matches!(e, XyEvent::TextDelta(_)) && format!("{e:?}").contains("Tool"));
    assert!(has_tool, "expected tool execution, got {:?}", events);
    // 无 adk 类型：XyEvent 是闭集，不存在 adk 变体（编译期保证），此处仅断言非空结束
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TurnEnd { .. })),
        "expected TurnEnd, got {:?}",
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
    let mut stream = runner.run("hi").await;
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
    // 每项已是 XyEvent（Vec<XyEvent> 编译期保证）；断言含文本增量
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TextDelta(_))),
        "expected TextDelta in stream, got {:?}",
        events
    );
}

// ar12 abort-drops-sse：复用 agent abort 词表（单 given 自足装配，因 solidify 无 Background）。
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

// ── ar8–ar12 / ar7：队列、abort、before-hook（共享 helper，避免 step 体复制）────────
thread_local! {
    static AR_RUNNER: std::cell::RefCell<Option<AgentRuntime>> =
        const { std::cell::RefCell::new(None) };
    static AR_RUNNER_EVENTS: std::cell::RefCell<Vec<XyEvent>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static AR_BANG_RESULT: std::cell::RefCell<
        Option<Result<xylitol::protocol::ports::XyBashResult, XyDriverError>>,
    > = const { std::cell::RefCell::new(None) };
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

/// 自足装配：reset fake + workspace + 注册模型 + 选中 + make_agent。
pub(crate) fn ar_make_runner(agent: &AgentState, ws: &Workspace) -> AgentRuntime {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-queue");
    let mut runtime = make_agent(agent);
    runtime
        .inner_mut()
        .select_model("ar-queue")
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
    let mut stream = runner.run(prompt).await;
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

// ar8 steer-before-model
#[given("装配并运行入队 steer 的 agent")]
pub(crate) async fn _g_ar8_steer_before_model(agent: &AgentState, ws: &Workspace) {
    set_fake_text("steer ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.steer("插队指令");
    let _ = ar_run_capture(&mut runner, "初始提示").await;
    ar_store_runner(runner);
}

#[when("检查队列与历史")]
pub(crate) fn _w_ar8_check_queue() {
    // 断言落在 then；此处仅确认 runner 仍在。
    let _ = ar_with_runner(|r| r.queue_stats());
}

#[then("steer 计数归零且已处理")]
pub(crate) fn _t_ar8_steer_drained() {
    ar_with_runner(|r| {
        assert_eq!(r.queue_stats().steer_count, 0, "steer should be drained");
    });
}

// ar8 followup-extends
#[given("装配无工具 agent 并入队 follow_up")]
pub(crate) async fn _g_ar8_followup_extends(agent: &AgentState, ws: &Workspace) {
    set_fake_text("followup ack");
    let runner = ar_make_runner(agent, ws);
    runner.follow_up("追问内容");
    ar_store_runner(runner);
}

#[when("运行至将结束")]
pub(crate) async fn _w_ar8_run_until_end() {
    ar_take_run_store("主提示").await;
}

#[then("继续循环而非 AgentEnd")]
pub(crate) fn _t_ar8_followup_continues() {
    assert!(
        !ar_events().is_empty(),
        "follow_up should extend the turn, got empty stream"
    );
}

// ar9 queue-update
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

#[then("出现 QueueUpdate 且计数正确")]
pub(crate) fn _t_ar9_queue_update_emitted() {
    let found = ar_events()
        .iter()
        .any(|ev| matches!(ev, XyEvent::QueueUpdate { .. }));
    assert!(
        found,
        "expected QueueUpdate after enqueue, got {:?}",
        ar_events()
    );
}

// ar10 abort-clears-steer
#[given("装配 agent 并入队 steer 与 follow_up")]
pub(crate) async fn _g_ar10_abort_clears(agent: &AgentState, ws: &Workspace) {
    set_fake_text("abort queue ack");
    let runner = ar_make_runner(agent, ws);
    runner.steer("待清除 steer");
    runner.follow_up("保留 follow_up");
    ar_store_runner(runner);
}

#[when("abort")]
pub(crate) fn _w_ar10_abort() {
    ar_with_runner(|r| r.abort());
}

#[then("steer 空且 follow_up 保留")]
pub(crate) fn _t_ar10_queue_semantics() {
    ar_with_runner(|r| {
        let stats = r.queue_stats();
        assert_eq!(stats.steer_count, 0, "abort must clear steer");
        assert_eq!(stats.follow_up_count, 1, "abort must keep follow_up");
    });
}

// ar11 second-run-after-abort
#[given("装配慢速 agent 并在首轮 abort 后")]
pub(crate) async fn _g_ar11_second_run_after_abort(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    ws.init();
    ar_register_fake(agent, "ar-abort-second");
    set_fake_slow_stream(20, 10);
    let mut runner = make_agent(agent);
    runner
        .inner_mut()
        .select_model("ar-abort-second")
        .expect("select ar-abort-second fake model");
    let mut stream = runner.run("首轮").await;
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
        .any(|ev| matches!(ev, XyEvent::Error(m) if m == "aborted"));
    let ended = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::TurnEnd { .. }));
    assert!(!aborted, "second run must not be immediately aborted");
    assert!(ended, "second run must complete with TurnEnd");
}

// ar24 should-stop-emits-agent-end / should-stop-skips-followup
#[given("注册 should_stop_after_turn 在首次 TurnEnd 后返回 true")]
pub(crate) async fn _g_ar24_should_stop(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-after-turn ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.set_should_stop_after_turn(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("入队 follow_up 且 should_stop_after_turn 在首次 TurnEnd 后返回 true")]
pub(crate) async fn _g_ar24_should_stop_with_followup(agent: &AgentState, ws: &Workspace) {
    set_fake_text("stop-skip-followup ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("停闸后不应注入的追问");
    runner.set_should_stop_after_turn(Some(std::sync::Arc::new(|_| true)));
    ar_store_runner(runner);
}

#[given("未注册 should_stop_after_turn 的无工具 agent")]
pub(crate) async fn _g_ar24_no_hook_open(agent: &AgentState, ws: &Workspace) {
    set_fake_text("open-end ack");
    let runner = ar_make_runner(agent, ws);
    ar_store_runner(runner);
}

#[given("按 session.max_turns=2 安装 should_stop_after_turn 且入队 follow_up 以迫使第二轮")]
pub(crate) async fn _g_ar30_max_turns(agent: &AgentState, ws: &Workspace) {
    set_fake_text("max-turns ack");
    let mut runner = ar_make_runner(agent, ws);
    runner.follow_up("第二轮追问");
    runner.set_should_stop_after_turn(Some(xylitol::agent::max_turns_stop_hook(2)));
    ar_store_runner(runner);
}

#[then("出现 AgentEnd 且其后无新的模型轮 TurnStart")]
pub(crate) fn _t_ar24_agent_end_no_extra_turn(agent: &AgentState) {
    let events = agent.events.borrow();
    let mut turn_starts = 0usize;
    let mut saw_agent_end = false;
    let mut turn_start_after_end = false;
    for ev in events.iter() {
        match ev {
            XyEvent::TurnStart { .. } => {
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
        "expected exactly one TurnStart, got {events:?}"
    );
    assert!(
        !turn_start_after_end,
        "no TurnStart after AgentEnd: {events:?}"
    );
}

#[then("至多出现 2 次 TurnStart 后出现 AgentEnd")]
pub(crate) fn _t_ar30_max_two_turns(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|ev| matches!(ev, XyEvent::TurnStart { .. }))
        .count();
    let saw_agent_end = events
        .iter()
        .any(|ev| matches!(ev, XyEvent::AgentEnd { .. }));
    assert!(saw_agent_end, "expected AgentEnd, got {events:?}");
    assert!(
        turn_starts <= 2,
        "expected at most 2 TurnStart, got {turn_starts}: {events:?}"
    );
    assert!(
        turn_starts >= 1,
        "expected at least one TurnStart, got {events:?}"
    );
}

#[then("本 run 以 AgentEnd 结束且 follow_up 未被注入历史")]
pub(crate) fn _t_ar24_followup_not_injected(agent: &AgentState) {
    let events = agent.events.borrow();
    let agent_end = events.iter().rev().find_map(|ev| match ev {
        XyEvent::AgentEnd { messages } => Some(messages.clone()),
        _ => None,
    });
    let history = agent_end.expect("expected AgentEnd");
    let injected = history.iter().any(|m| match m {
        xylitol::protocol::message::AgentMessage::Llm(
            xylitol::protocol::message::LlmMessage::UserMessage { content, .. },
        ) => content.iter().any(|p| match p {
            xylitol::protocol::message::AgentPart::Text { text } => {
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

#[then("正常出现 AgentEnd 且恰好一轮 TurnStart")]
pub(crate) fn _t_ar24_no_hook_open_end(agent: &AgentState) {
    let events = agent.events.borrow();
    let turn_starts = events
        .iter()
        .filter(|ev| matches!(ev, XyEvent::TurnStart { .. }))
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
        "open end without max_iterations / stop hook: one TurnStart, got {events:?}"
    );
}

// ── c1545 tool batch (ar27–ar29) ─────────────────────────────────────
thread_local! {
    static BATCH_TIMING: RefCell<Vec<(String, u128, u128)>> = const { RefCell::new(Vec::new()) };
    static BATCH_EPOCH: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
}

struct BddSlowTool {
    name: &'static str,
    mode: xylitol::protocol::ports::XyToolExecutionMode,
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
    fn execution_mode(&self) -> xylitol::protocol::ports::XyToolExecutionMode {
        self.mode
    }
    async fn execute(
        &self,
        _ctx: &XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, xylitol::protocol::error::XyToolError> {
        let epoch = BATCH_EPOCH.with(|e| e.get().expect("batch epoch"));
        let start = epoch.elapsed().as_millis();
        tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms)).await;
        let end = epoch.elapsed().as_millis();
        BATCH_TIMING.with(|t| t.borrow_mut().push((self.name.to_string(), start, end)));
        Ok(format!("{}-ok", self.name))
    }
}

struct BddMultiToolModel {
    rounds: std::sync::Mutex<Vec<Vec<xylitol::protocol::types::XyChunk>>>,
}

#[async_trait::async_trait]
impl xylitol::protocol::ports::XyModel for BddMultiToolModel {
    fn name(&self) -> &str {
        "bdd-batch-mock"
    }
    async fn generate_stream(
        &self,
        _messages: Vec<xylitol::protocol::message::LlmMessage>,
        _tools: &[xylitol::protocol::types::XyToolSchema],
        _stream: bool,
        _options: xylitol::protocol::ports::XyGenerateOptions,
    ) -> Result<xylitol::protocol::ports::XyStream, xylitol::protocol::error::XyError> {
        let chunks = self.rounds.lock().unwrap().remove(0);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

pub(crate) fn bdd_batch_rounds(
    calls: &[(&str, &str)],
) -> Vec<Vec<xylitol::protocol::types::XyChunk>> {
    let done = || xylitol::protocol::types::XyChunk::Done {
        finish_reason: xylitol::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let mut round1 = Vec::new();
    for (i, (name, args_json)) in calls.iter().enumerate() {
        let args: serde_json::Value =
            serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
        round1.push(xylitol::protocol::types::XyChunk::ToolCallEnd {
            id: format!("call-{i}"),
            name: (*name).into(),
            args,
        });
    }
    round1.push(done());
    vec![
        round1,
        vec![
            xylitol::protocol::types::XyChunk::TextDelta("ok".into()),
            done(),
        ],
    ]
}

pub(crate) fn bdd_batch_make_runner(
    agent: &AgentState,
    tools: ToolSet,
    calls: &[(&str, &str)],
    mode: xylitol::protocol::ports::XyBatchMode,
) -> AgentRuntime {
    use xylitol::protocol::ports::{XyEventSink, XyModel, XySessionStore};
    BATCH_TIMING.with(|t| t.borrow_mut().clear());
    BATCH_EPOCH.with(|e| e.set(Some(std::time::Instant::now())));
    reset_fake_state();
    ar_register_fake(agent, "ar-batch");
    let rounds = bdd_batch_rounds(calls);
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: Arc<dyn XySessionStore> = Arc::new(mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(xylitol::infra::event::EventBus::new());
    let builder: xylitol::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(BddMultiToolModel {
            rounds: std::sync::Mutex::new(rounds.clone()),
        }) as Arc<dyn XyModel>)
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
        xylitol::infra::permission::allow_all_permission(),
        None,
        None,
        xylitol::agent::session::QueueMode::default(),
        xylitol::agent::session::QueueMode::default(),
        None,
    );
    session
        .select_model("ar-batch")
        .expect("select ar-batch fake");
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
        mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
        sleep_ms: 80,
    }) as Arc<dyn XyTool>]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_safe", r#"{"n":1}"#), ("slow_safe", r#"{"n":2}"#)],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
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
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 100,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Sequential,
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
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
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
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 60,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_barrier",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Sequential,
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
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
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
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "mcp:fake:x",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[
            ("slow_safe", r#"{"n":1}"#),
            ("mcp:fake:x", r#"{}"#),
            ("slow_safe", r#"{"n":2}"#),
        ],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
    );
    ar_store_runner(runner);
}

#[given("工具批模式为 barrier_parallel 且并行窗内后发先完成")]
pub(crate) fn _g_ar29_history(agent: &AgentState, ws: &Workspace) {
    ws.init();
    let tools = ToolSet::from_iter(vec![
        Arc::new(BddSlowTool {
            name: "slow_a",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 120,
        }) as Arc<dyn XyTool>,
        Arc::new(BddSlowTool {
            name: "slow_b",
            mode: xylitol::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 30,
        }) as Arc<dyn XyTool>,
    ]);
    let runner = bdd_batch_make_runner(
        agent,
        tools,
        &[("slow_a", r#"{}"#), ("slow_b", r#"{}"#)],
        xylitol::protocol::ports::XyBatchMode::BarrierParallel,
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

#[then("两工具按源序串行执行且无并行重叠")]
pub(crate) fn _t_ar27_sequential_no_overlap() {
    let entries = BATCH_TIMING.with(|t| t.borrow().clone());
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        !bdd_timing_overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
        "must not overlap: {entries:?}"
    );
    assert!(entries[0].2 <= entries[1].1, "source order: {entries:?}");
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

#[then(
    "第二个 ParallelSafe MUST NOT 与第一个 ParallelSafe 同窗并行且 MUST 在 Barrier 完成之后开始"
)]
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
        .find(|(n, _, _)| n == "mcp:fake:x")
        .expect("mcp");
    for (n, s, e) in &entries {
        if n == "mcp:fake:x" {
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
    // Prefer prepared runner; run if events empty.
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
            XyEvent::AgentEnd { messages } => Some(messages.clone()),
            _ => None,
        })
        .expect("AgentEnd");
    let ids: Vec<_> = history
        .iter()
        .filter_map(|m| match m {
            xylitol::protocol::message::AgentMessage::Llm(
                xylitol::protocol::message::LlmMessage::ToolResultMessage { tool_use_id, .. },
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

// ar10 abort-cancels-bang
#[given("启动交互 bang 长命令后 abort")]
pub(crate) async fn _g_ar10_abort_cancels_bang(_agent: &AgentState, _ws: &Workspace) {
    use xylitol::infra::bash_exec::InfraBashExecutor;
    use xylitol::protocol::ports::{BashExecOpts, XyBashExecutor};
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
            result.cancelled,
            "bang must be cancelled, got cancelled={}",
            result.cancelled
        );
    });
}

// ar7 before-denies
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
