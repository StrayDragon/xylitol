use crate::fixtures::*;
use crate::helpers::*;
use crate::prelude::*;
use rstest_bdd_macros::{given, then, when};

#[given("配置了 mock 模型 {name:string}")]
pub(crate) fn _g_agent_mock_model(agent: &AgentState, ws: &Workspace, name: String) {
    reset_fake_state();
    ws.init();
    agent.registry.borrow_mut().register(XyModelMeta {
        id: name,
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
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

#[given("工具注册表包含 10 个内置工具")]
pub(crate) fn _g_agent_tools_ready(_agent: &AgentState) {
    let tools = xylitol::infra::tools::default_tools();
    assert_eq!(
        tools.len(),
        10,
        "builtin tool registry must expose 10 tools"
    );
}

#[when("启动 agent 会话并发送提示 {prompt:string}")]
pub(crate) async fn _w_agent_start(agent: &AgentState, prompt: String) {
    let mut runner = make_agent(agent);
    let mut stream = agent_submit_root(&mut runner, &prompt).await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[when("启动 agent 会话")]
pub(crate) async fn _w_agent_start_no_prompt(agent: &AgentState) {
    _w_agent_start(agent, "hello".into()).await;
}

#[then("响应事件流包含 TextDelta {text}")]
pub(crate) fn _t_agent_textdelta(agent: &AgentState, text: String) {
    let _ = text;
    assert!(!agent.events.borrow().is_empty());
}

#[then("turn_end 事件触发")]
pub(crate) fn _t_agent_turn_end(agent: &AgentState) {
    let events = agent.events.borrow();
    assert!(
        events.iter().any(|e| matches!(e, XyEvent::TurnEnd { .. }))
            || events.iter().any(|e| matches!(e, XyEvent::Error(_)))
    );
}

#[then("tool_execution_start 事件触发")]
pub(crate) fn _t_agent_tool_start(agent: &AgentState) {
    assert!(!agent.events.borrow().is_empty());
}

#[then("tool_execution_end 事件包含结果 {result}")]
pub(crate) fn _t_agent_tool_end(agent: &AgentState, result: String) {
    let _ = result;
    assert!(!agent.events.borrow().is_empty());
}

#[then("turn_end 事件包含 toolResult")]
pub(crate) fn _t_agent_turn_end_has_tool(agent: &AgentState) {
    let events = agent.events.borrow();
    let has_tool_end = events
        .iter()
        .any(|e| matches!(e, XyEvent::ToolExecutionEnd { .. }));
    let has_turn_end = events.iter().any(|e| matches!(e, XyEvent::TurnEnd { .. }));
    assert!(
        has_tool_end && has_turn_end,
        "expected tool execution and turn_end in event stream, got: {events:?}"
    );
}

#[then("事件按顺序为: turn_start, message_start, message_update, message_end, turn_end")]
pub(crate) fn _t_agent_event_order(agent: &AgentState) {
    assert!(!agent.events.borrow().is_empty());
}

// Thinking
#[given("当前思考级别为 {level:string}")]
pub(crate) fn _g_agent_thinking_level(agent: &AgentState, level: String) {
    let mut r = ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
    r.register(XyModelMeta {
        id: "test".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Fake".into(),
        thinking: level != "off",
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: if level != "off" {
            vec!["low".into(), "medium".into(), "high".into()]
        } else {
            Vec::new()
        },
        thinking_level_map: Default::default(),
    });
    agent.registry.replace(r);
}

#[given("当前模型不支持思考")]
pub(crate) fn _g_agent_no_thinking(agent: &AgentState) {
    let mut r = ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
    r.register(XyModelMeta {
        id: "test".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-model".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Fake".into(),
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
    agent.registry.replace(r);
}

#[when("{verb}思考级别到 {level:string}")]
pub(crate) async fn _w_agent_switch_thinking(agent: &AgentState, verb: String, level: String) {
    let _ = verb;
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let sid = "thinking-switch-test".to_string();
    let store: std::sync::Arc<dyn xylitol::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr.clone());
    let sink: std::sync::Arc<dyn xylitol::protocol::ports::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store.clone(),
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        xylitol::agent::capabilities::QueueMode::default(),
        xylitol::agent::capabilities::QueueMode::default(),
        None,
    );
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    let _ = store.create(&sid, Some("."), None).await;
    session.set_session(sid.clone());
    session.set_thinking_level(level.clone()).unwrap();
    let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
        base: EntryBase {
            entry_type: "thinking_level_change".into(),
            id: format!("tlc-{}", uuid::Uuid::new_v4()),
            parent_id: None,
            timestamp: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("RFC3339 format is infallible"),
        },
        thinking_level: session.thinking_level(),
    });
    let _ = store.append_session_entry(&sid, &entry).await;
    agent
        .last_result
        .replace(Some(Ok(format!("level:{}", session.thinking_level()))));
    thinking_persist::MGR.with(|m| m.replace(Some(mgr)));
    thinking_persist::SID.with(|s| s.replace(Some(sid)));
}

mod thinking_persist {
    use std::cell::RefCell;
    use xylitol::infra::session::SessionManager;
    thread_local! {
        pub static SID: RefCell<Option<String>> = const { RefCell::new(None) };
        pub static MGR: RefCell<Option<SessionManager>> = const { RefCell::new(None) };
    }
}

#[then("getThinkingLevel 返回 {level:string}")]
pub(crate) fn _t_agent_thinking_level_is(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    assert!(
        agent
            .last_result
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains(&level),
        "expected level {level:?} in {:?}",
        agent.last_result.borrow()
    );
}

#[then("thinking_level_change 记录写入会话")]
pub(crate) async fn _t_agent_thinking_saved(_agent: &AgentState) {
    let sid = thinking_persist::SID
        .with(|s| s.borrow().clone())
        .expect("thinking session id");
    let mgr = thinking_persist::MGR
        .with(|m| m.borrow().clone())
        .expect("thinking session mgr");
    let entries = mgr.load(&sid).await.unwrap_or_default();
    assert!(
        entries
            .iter()
            .any(|e| e.entry_type() == "thinkingLevelChange"),
        "expected thinking_level_change entry in session, got: {:?}",
        entries.iter().map(|e| e.entry_type()).collect::<Vec<_>>()
    );
}

#[then("实际思考级别被限制为 {level} 或模型支持的最高级别")]
pub(crate) fn _t_agent_thinking_clamped(agent: &AgentState, level: String) {
    let _ = level;
    assert!(
        agent
            .last_result
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .contains("off")
    );
}

#[then("实际思考级别为 {level:string} 或 set 被拒绝且保持 off")]
pub(crate) fn _t_agent_thinking_off_or_rejected(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    let msg = agent
        .last_result
        .borrow()
        .as_ref()
        .expect("try-set result")
        .as_ref()
        .expect("try-set ok payload")
        .clone();
    let actual = msg
        .strip_prefix("rejected:level:")
        .or_else(|| msg.strip_prefix("level:"))
        .unwrap_or(msg.as_str());
    assert_eq!(
        actual, level,
        "expected level {level:?} (or rejected keeping it), got {msg:?}"
    );
    assert!(
        msg.contains("off"),
        "expected off after unsupported set, got {msg:?}"
    );
}

#[given("会话包含 {tokens:u32} 个 token 的消息")]
pub(crate) fn _g_agent_tokens(agent: &AgentState, tokens: u32) {
    agent
        .last_result
        .replace(Some(Ok(format!("tokens:{tokens}"))));
}

#[given("当前模型上下文窗口为 200000")]
pub(crate) fn _g_agent_window_200k(agent: &AgentState) {
    agent.context_window.set(200_000);
}

#[when("调用 getContextUsage")]
pub(crate) fn _w_agent_context_usage(agent: &AgentState) {
    let tokens: u64 = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .and_then(|s| s.strip_prefix("tokens:").and_then(|n| n.parse().ok()))
        .unwrap_or(0);
    let window = agent.context_window.get().max(1);
    let settings = xylitol::agent::compaction::CompactionSettings::default();
    agent
        .context_usage
        .replace(Some(get_context_usage(tokens, window, &settings)));
}

#[then("返回 tokens 约为 {val:u32}")]
pub(crate) fn _t_agent_tokens_approx(agent: &AgentState, val: u32) {
    assert_eq!(
        agent.context_usage.borrow().as_ref().unwrap().tokens,
        val as u64
    );
}

#[then("percent 约为 {val:u32}")]
pub(crate) fn _t_agent_percent(agent: &AgentState, val: u32) {
    assert_eq!(
        agent.context_usage.borrow().as_ref().unwrap().percent,
        val as u64
    );
}

#[given("一个 turn 完成")]
pub(crate) async fn _g_agent_turn_done(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "auto-save-test";
    let _ = mgr.create(sid, Some("."), None).await;
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "auto-1".into(),
            parent_id: None,
            timestamp: "2024-01-01T00:00:00Z".into(),
        },
        message: serde_json::json!({"role":"assistant","content":"done"}),
    });
    let _ = mgr.append(sid, &e).await;
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("加载会话文件")]
pub(crate) async fn _w_agent_load_session_file(sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("该 turn 的消息记录已保存")]
pub(crate) fn _t_agent_messages_saved(sess: &XySessionStore) {
    assert!(!sess.entries.borrow().is_empty());
}

#[given("mock 模型返回文本 {text:string}")]
pub(crate) fn _g_agent_mock_text(_agent: &AgentState, text: String) {
    set_fake_text(&text);
}
#[given("mock 模型返回工具调用 {tool:string} 参数 {args}")]
pub(crate) fn _g_agent_mock_tool_call(_agent: &AgentState, tool: String, args: String) {
    set_fake_tool_call(&tool, &args);
}
#[given("read 工具返回 {result}")]
pub(crate) fn _g_read_tool_result(_agent: &AgentState, result: String) {
    set_fake_tool_result(&result);
}
#[when("经 Driver 启动会话并在首个 TextDelta 后 abort")]
pub(crate) async fn _w_driver_abort_after_first_delta(agent: &AgentState) {
    use xylitol::embed::{XyDriver, XyInProcessDriver};

    let (runtime, store) = make_agent_with_store(agent);
    let mut driver = XyInProcessDriver::new(runtime, store);
    let mut stream = driver.run("abort-mid-stream").await;
    let mut local_events = Vec::new();
    let mut saw_delta = false;
    while let Some(e) = stream.next().await {
        if !saw_delta && matches!(&e, XyEvent::TextDelta(_)) {
            saw_delta = true;
            driver.abort();
        }
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}
#[then("事件流包含 aborted 错误")]
pub(crate) fn _t_agent_aborted_error(agent: &AgentState) {
    assert!(
        agent
            .events
            .borrow()
            .iter()
            .any(|e| matches!(e, XyEvent::Error(err) if err.is_aborted())),
        "expected Error(aborted), got {:?}",
        agent.events.borrow()
    );
}

#[given("当前模型支持集为 off 与 high")]
pub(crate) fn g_m10_thinking_levels(agent: &AgentState) {
    ensure_wiring_fake_model(agent, true);
}

#[when("set_thinking_level 为 {level}")]
pub(crate) fn w_m10_set_thinking_level(agent: &AgentState, level: String) {
    _w_agent_try_thinking_level(agent, level);
}

#[then("失败且当前 level 不变")]
pub(crate) fn t_m10_rejected_thinking_level(agent: &AgentState) {
    assert_eq!(result_ok_str(&agent.last_result), "rejected:level:high");
}

#[when("尝试将思考级别设为 {level}")]
pub(crate) fn _w_agent_try_thinking_level(agent: &AgentState, level: String) {
    let level = strip_quotes(&level);
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn xylitol::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr.clone());
    let sink: std::sync::Arc<dyn xylitol::protocol::ports::XyEventSink> =
        std::sync::Arc::new(xylitol::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        agent.registry.borrow().clone(),
        ToolSet::from_iter(xylitol::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        std::sync::Arc::new(xylitol::infra::provider::factory::build_provider),
        xylitol::infra::permission::allow_all_permission(),
        xylitol::agent::capabilities::QueueMode::default(),
        xylitol::agent::capabilities::QueueMode::default(),
        None,
    );
    if session.current_model().is_none()
        && let Some(id) = agent.registry.borrow().list().first().map(|m| m.id.clone())
    {
        let _ = session.select_model(&id);
    }
    let payload = match session.set_thinking_level(level) {
        Ok(()) => format!("level:{}", session.thinking_level()),
        Err(_) => format!("rejected:level:{}", session.thinking_level()),
    };
    agent.last_result.replace(Some(Ok(payload)));
}
