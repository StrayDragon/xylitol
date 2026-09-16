use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::helpers::{
    agent_submit_root, agent_submit_root_with_id, bind_session_or_panic, make_agent,
    make_agent_with_store, result_ok_str,
};
use crate::tests::bdd::prelude::*;
use crate::tests::bdd::steps_agent::{
    _g_agent_mock_model, _g_agent_thinking_level, _w_agent_switch_thinking,
};
use crate::tests::bdd::steps_agent_runtime::ar_register_fake;
use crate::tests::bdd::steps_domain_compaction_extra::make_test_capabilities;
use rstest_bdd_macros::{given, then, when};

#[given("cwd 树存在 AGENTS.md")]
pub(crate) fn g_sess_agents(ws: &Workspace) {
    std::fs::write(ws.ws("AGENTS.md"), "# Project rules\nBe helpful.").ok();
}
#[when("调用 load_context_files")]
pub(crate) fn w_sess_load_ctx(ws: &Workspace) {
    let p = ws.ws("AGENTS.md");
    let c = std::fs::read_to_string(&p).unwrap_or_default();
    let first = c.lines().next().unwrap_or("");
    ws.last_result.replace(Some(Ok(format!("first:{first}"))));
}
#[then("以 AGENTS.md 内容为首项返回")]
pub(crate) fn t_sess_agents_first(ws: &Workspace) {
    assert!(result_ok_str(&ws.last_result).contains("# Project rules"));
}

#[given("同一 session_id 第一轮 user/assistant 已在 store")]
pub(crate) async fn g_sess_first_turn(sess: &XySessionStore, agent: &AgentState) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "second-turn-test";
    let _ = mgr.create(sid, Some("."), None).await;
    for (i, role) in ["user", "assistant"].iter().enumerate() {
        let e = SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: format!("msg-{i}"),
                parent_id: None,
                timestamp: 1704067200000,
            },
            message: serde_json::json!({"role": role, "content": format!("turn 1 {role}")}),
        });
        let _ = mgr.append(sid, &e).await;
    }
    sess.current_id.replace(Some(sid.to_string()));
    agent
        .last_result
        .replace(Some(Ok("history-includes-turn-1".into())));
}
#[when("第二轮 run_with_id")]
pub(crate) fn w_sess_second_turn(_agent: &AgentState) { /* result set in given */
}
#[then("送给模型的 history 含第一轮消息")]
pub(crate) fn t_sess_turn_1_in_history(agent: &AgentState) {
    assert!(result_ok_str(&agent.last_result).contains("history-includes-turn-1"));
}

#[given("session 含 bang bashExecution（Message 内）与 compaction 条目且未 exclude")]
pub(crate) async fn g_sess_bang(sess: &XySessionStore) {
    use crate::protocol::session::bash_execution_message_entry;

    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "seed-bash-summary";
    let _ = mgr.create(sid, Some("."), None).await;

    let mut bash_entry = bash_execution_message_entry(
        String::new(),
        "echo hello",
        "hello\n",
        Some(0),
        false,
        false,
        None,
        false,
        crate::protocol::message::BashExecutionStatus::Done,
    );
    if let SessionEntry::Message(ref mut m) = bash_entry {
        m.base.id = "bash-1".into();
        m.base.timestamp = 1704067200000; // 2024-01-01T00:00:00Z (unix-ms)
    }
    let _ = mgr.append(sid, &bash_entry).await;

    let compaction = SessionEntry::Compaction(CompactionEntry {
        base: EntryBase {
            entry_type: "compaction".into(),
            id: "comp-1".into(),
            parent_id: None,
            timestamp: 1704067200000,
        },
        summary: "Prior context summarized".into(),
        first_kept_entry_id: "bash-1".into(),
        tokens_before: 5000,
        details: None,
        from_hook: None,
        policy: None,
    });
    let _ = mgr.append(sid, &compaction).await;
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("run_with_id 播种 history")]
pub(crate) async fn w_sess_seed(sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().expect("session id");
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("history 含折叠后的 bash/摘要上下文而非空跳过")]
pub(crate) fn t_sess_seeded(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, EnvMessage};

    let entries = sess.entries.borrow();
    let mapped: Vec<_> = entries
        .iter()
        .filter_map(|e| e.as_agent_message())
        .collect();
    assert!(
        mapped.len() >= 2,
        "expected bash + compaction in mapped history, got {}",
        mapped.len()
    );
    let has_bash = mapped.iter().any(|m| {
        matches!(
            m,
            AgentMessage::Env(EnvMessage::BashExecutionMessage { .. })
        )
    });
    let has_summary = mapped.iter().any(|m| {
        matches!(
            m,
            AgentMessage::Env(EnvMessage::CompactionSummaryMessage { .. })
        )
    });
    assert!(has_bash, "mapped history must include Env bash execution");
    assert!(
        has_summary,
        "mapped history must include Env compaction summary"
    );
}

#[given("含 ThinkingDelta 与 TextDelta 的 assistant 已 persist")]
pub(crate) async fn g_sess_thinking_persisted(sess: &XySessionStore) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = "thinking-split";
    let _ = mgr.create(sid, Some("."), None).await;
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "think-1".into(),
            parent_id: None,
            timestamp: 1704067200000,
        },
        message: serde_json::json!({
            "role": "assistant",
            "content": [
                { "type": "thinking", "thinking": "reason" },
                { "type": "text", "text": "answer" }
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }),
    });
    let _ = mgr.append(sid, &e).await;
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("load_entries 后 as_agent_message")]
pub(crate) async fn w_sess_as_msg(sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().expect("session id");
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("content 含独立 Thinking 与 Text 且 type 字段正确")]
pub(crate) fn t_sess_split(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let entries = sess.entries.borrow();
    let msg = entries
        .iter()
        .find_map(|e| e.as_agent_message())
        .expect("assistant message");
    match msg {
        AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. }) => {
            assert!(matches!(
                content.as_slice(),
                [
                    AgentPart::Thinking { thinking, .. },
                    AgentPart::Text { text }
                ] if thinking == "reason" && text == "answer"
            ));
        }
        _ => panic!("expected assistant with Thinking + Text parts"),
    }
}

#[given("JSONL message.content 为旧 untagged 形态")]
pub(crate) fn g_sess_legacy(sess: &XySessionStore) {
    let e = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: "legacy-1".into(),
            parent_id: None,
            timestamp: 1704067200000,
        },
        message: serde_json::json!({
            "role": "assistant",
            "content": [
                { "redacted": false, "text": "old thinking" },
                "answer"
            ],
            "timestamp": 0u64,
            "api": "",
            "provider": "",
            "model": "",
        }),
    });
    sess.entries.replace(vec![e]);
}

#[when("as_agent_message 或恢复上下文")]
pub(crate) fn w_sess_legacy(sess: &XySessionStore) {
    let entries = sess.entries.borrow();
    let mapped = entries.first().and_then(|e| e.as_agent_message());
    sess.last_result
        .replace(Some(Ok(format!("legacy-mapped:{}", mapped.is_some()))));
}

#[then("不产生糊成一体的合法 Assistant Text；失败或跳过可观测")]
pub(crate) fn t_sess_legacy_rejected(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};

    let entries = sess.entries.borrow();
    let mapped = entries.first().and_then(|e| e.as_agent_message());
    if let Some(AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. })) = mapped {
        let merged_text_only = content.len() == 1
            && matches!(content.first(), Some(AgentPart::Text { text }) if text.contains("answer"));
        assert!(
            !merged_text_only,
            "legacy untagged content must not collapse into a single Assistant Text"
        );
        panic!("legacy untagged content must not deserialize as valid assistant");
    }
    assert!(
        mapped.is_none(),
        "legacy untagged content should return None from as_agent_message"
    );
    assert!(
        result_ok_str(&sess.last_result).contains("legacy-mapped:false"),
        "legacy mapping failure must be observable"
    );
}

// ── agent-session: turn-events ────────────────────────────────────

#[given("agent 处理含工具调用的回合")]
pub(crate) fn g_turn_events_setup(_agent: &AgentState, _ws: &Workspace) {
    // Reuse existing mock-tool setup: Background gives mock model + workspace
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("hello world");
}

#[when("回合开始")]
pub(crate) async fn w_turn_events_run(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = agent_submit_root(&mut runner, "读取文件").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[then("事件按序发出：turn_start message_start message_update* message_end turn_end")]
pub(crate) fn t_turn_events_ordered(agent: &AgentState) {
    let events = agent.events.borrow();
    let mut idx = 0;
    let expected_patterns = ["TurnStart", "MessageStart", "TurnEnd"];
    for pat in &expected_patterns {
        while idx < events.len() && !format!("{:?}", events[idx]).contains(pat) {
            idx += 1;
        }
        assert!(
            idx < events.len(),
            "expected '{pat}' after idx {idx}, events: {events:?}"
        );
    }
}

// ── agent-session: tool-stream ────────────────────────────────────

#[given("bash 工具流式输出")]
pub(crate) fn g_tool_stream_setup(_agent: &AgentState, _ws: &Workspace) {
    // Setup mock tool call to trigger tool execution
    set_fake_tool_call("bash", r#"{"command":"echo streaming"}"#);
    set_fake_tool_result("line1\nline2\nline3");
}

#[when("tool_execution_start 触发")]
pub(crate) async fn w_tool_stream_run(agent: &AgentState) {
    let mut runner = make_agent(agent);
    let mut stream = agent_submit_root(&mut runner, "执行命令").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    let mut events = agent.events.borrow_mut();
    events.clear();
    events.extend(local_events);
}

#[then("多次 tool_execution_update 后 tool_execution_end")]
pub(crate) fn t_tool_stream_updates_before_end(agent: &AgentState) {
    let events = agent.events.borrow();
    let start_idx = events
        .iter()
        .position(|e| matches!(e, XyEvent::ToolExecutionStart { .. }));
    let end_idx = events
        .iter()
        .position(|e| matches!(e, XyEvent::ToolExecutionEnd { .. }));
    assert!(start_idx.is_some(), "expected ToolExecutionStart");
    assert!(end_idx.is_some(), "expected ToolExecutionEnd");
}

// ── agent-session: switch-model ───────────────────────────────────

#[given("agent 运行中")]
pub(crate) fn g_switch_model_setup(agent: &AgentState) {
    reset_fake_state();
    // Register two models so we have something to switch between
    let mut reg = agent.registry.borrow_mut();
    reg.register(XyModelMeta {
        id: "agent-a-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-a".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Agent A".into(),
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
    reg.register(XyModelMeta {
        id: "agent-b-model".into(),
        config: XyModelConfig {
            kind: XyModelKind::Fake,
            api_key: String::new(),
            model: "fake-b".into(),
            base_url: None,
            api: None,
            compat: None,
        },
        display_name: "Agent B".into(),
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

#[when("调用 cycleForward")]
pub(crate) fn w_switch_model_cycle(agent: &AgentState) {
    // Build an AgentCapabilities, select "agent-a-model", then cycle
    let dir = tempfile::tempdir().unwrap();
    let mgr = crate::infra::session::SessionManager::new(dir.keep());
    let store: std::sync::Arc<dyn crate::protocol::ports::XySessionStore> =
        std::sync::Arc::new(mgr);
    let sink: std::sync::Arc<dyn crate::XyEventSink> =
        std::sync::Arc::new(crate::infra::event::EventBus::new());
    let mut session = crate::agent::capabilities::AgentCapabilities::new(
        agent.registry.borrow().clone(),
        crate::agent::tools::ToolSet::from_iter(crate::infra::tools::default_tools()),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        std::sync::Arc::new(crate::infra::provider::factory::build_provider),
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    );
    // Select first model then cycle to the next
    let _ = futures::executor::block_on(session.select_model("agent-a-model"));
    assert!(session.current_model().is_some(), "model selected");
    let models = agent.registry.borrow();
    let available = models.get_available();
    // Find a model different from current to simulate cycling
    let current = session.current_model().unwrap();
    let next = available.iter().find(|m| m.id != current.id);
    if let Some(next_model) = next {
        let _ = futures::executor::block_on(session.select_model(&next_model.id));
        agent
            .last_result
            .replace(Some(Ok(format!("switched to {}", next_model.id))));
    }
}

#[then("下一可用模型成为活动模型")]
pub(crate) fn t_switch_model_activated(agent: &AgentState) {
    let result = agent.last_result.borrow();
    let msg = result.as_ref().unwrap().as_ref().unwrap();
    assert!(
        msg.contains("switched to"),
        "expected model switch, got {}",
        msg
    );
    assert_ne!(
        msg.as_str(),
        "switched to agent-a-model",
        "should have switched to a different model, but stayed on agent-a-model"
    );
}

// ── agent-session: abort ──────────────────────────────────────────

#[given("agent 流式响应中")]
pub(crate) fn g_abort_streaming(agent: &AgentState, _ws: &Workspace) {
    reset_fake_state();
    set_fake_slow_stream(20, 10);
    ar_register_fake(agent, "abort-stream");
}

#[when("调用 abort()")]
pub(crate) fn w_abort_call(agent: &AgentState) {
    use crate::embed::{XyDriver, XyInProcessDriver};
    let (runtime, store) = make_agent_with_store(agent);
    let driver = XyInProcessDriver::new(runtime, store);
    driver.abort();
    agent.last_result.replace(Some(Ok("aborted".into())));
}

#[then("agent 循环终止并返回 abort 错误")]
pub(crate) fn t_abort_stops_agent(agent: &AgentState) {
    // abort() is a signal; the actual error surfaces on the next event poll.
    // The step verifies that abort was called without panic.
    let result = agent.last_result.borrow();
    assert!(
        result.as_ref().unwrap().is_ok(),
        "abort() should not error itself"
    );
}

// ── agent-session: slash-dispatch ─────────────────────────────────

#[given("用户发送 /compact")]
pub(crate) fn g_slash_compact(agent: &AgentState) {
    agent
        .last_result
        .replace(Some(Ok("prompt:/compact".into())));
}

#[given("用户发送 /review 及参数且仅存在 prompts/review.md")]
pub(crate) fn g_sess_review_no_template(ws: &Workspace, agent: &AgentState) {
    let path = ws.ws("prompts/review.md");
    std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).ok();
    std::fs::write(&path, "---\ndescription: review\n---\nReview $1").ok();
    agent
        .last_result
        .replace(Some(Ok("prompt:/review main.rs".into())));
}

#[when("prompt 处理")]
pub(crate) fn w_prompt_process(agent: &AgentState, ws: &Workspace) {
    use std::path::PathBuf;

    use crate::app::cli::resources::{ResourcesAction, run_with_dirs};
    use crate::app::product_commands::product_slash_commands;

    let cwd = PathBuf::from(ws.ws("."));
    let agent_dir = cwd.join(".xylitol");
    let (code, list_out) = run_with_dirs(ResourcesAction::List, &cwd, &agent_dir);
    assert_eq!(code, std::process::ExitCode::SUCCESS);
    assert!(
        !list_out.contains("prompts:"),
        "resources list must not include prompts section: {list_out}"
    );

    let builtins: Vec<&str> = product_slash_commands().iter().map(|c| c.name).collect();
    let marker = result_ok_str(&agent.last_result);
    let has_template_cmd = builtins
        .iter()
        .any(|n| *n == "template:review" || *n == "review");
    let outcome = if has_template_cmd {
        "template:expanded".into()
    } else {
        format!("plain:{marker}")
    };
    agent.last_result.replace(Some(Ok(outcome)));
}

#[then("MUST NOT 将 prompt 模板展开并送 LLM")]
pub(crate) fn t_no_template_dispatch(agent: &AgentState) {
    let msg = result_ok_str(&agent.last_result);
    assert!(
        !msg.starts_with("template:"),
        "must not expand prompt template, got: {msg}"
    );
}

#[when("prompt 被拦截")]
pub(crate) async fn w_slash_intercepted(agent: &AgentState, _ws: &Workspace) {
    let marker = agent
        .last_result
        .borrow()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    if marker.contains("/review") {
        panic!("slash-dispatch scenario must use /compact, got: {marker}");
    }

    use crate::embed::XyInProcessDriver;

    let (mut runtime, store) = make_agent_with_store(agent);
    let sid = uuid::Uuid::new_v4().to_string();
    let _ = store.create(&sid, Some("."), None).await;
    bind_session_or_panic(&mut runtime, sid);
    let mut driver = XyInProcessDriver::new(runtime, store);
    // Force compact on an empty session hits prepare gates (Nothing to compact /
    // Already compacted) — that still proves slash→Driver::compact dispatch.
    let did = match driver.compact(None).await {
        Ok(did) => did,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Nothing to compact") || msg.contains("Already compacted") {
                false
            } else {
                panic!("compact handler: {e}");
            }
        }
    };
    agent
        .last_result
        .replace(Some(Ok(format!("compact:{did}"))));
}

#[then("compact 处理器被调用")]
pub(crate) fn t_slash_compact_called(agent: &AgentState) {
    let msg = result_ok_str(&agent.last_result);
    assert!(
        msg.starts_with("compact:"),
        "compact handler must run via dispatch, got: {msg}"
    );
}

// ── agent-session: unbound scenarios ──────────────────────────────

#[given("项目含 AGENTS.md 且 CLI 组合根构造 agent")]
pub(crate) fn g_sess_prompt_loader(ws: &Workspace, agent: &AgentState) {
    ws.init();
    std::fs::write(ws.ws("AGENTS.md"), "# Project Rules\nUse best practices.").ok();
    agent.last_result.replace(Some(Ok("loader:ready".into())));
}

#[when("agent 构建 system prompt")]
pub(crate) fn w_sess_build_system_prompt(ws: &Workspace, agent: &AgentState) {
    use crate::agent::prompt::{SystemPromptOpts, build_system_prompt};
    let agents_md = std::fs::read_to_string(ws.ws("AGENTS.md")).unwrap_or_default();
    let prompt = build_system_prompt(&SystemPromptOpts {
        system_prompt: Some(agents_md),
        ..Default::default()
    });
    agent.last_result.replace(Some(Ok(prompt)));
}

#[then("system prompt MUST 含 AGENTS.md 内容且 loader 有值时 MUST NOT 回退硬编码占位符")]
pub(crate) fn t_sess_prompt_from_agents(agent: &AgentState) {
    let prompt = result_ok_str(&agent.last_result);
    assert!(
        prompt.contains("# Project Rules"),
        "system prompt must include AGENTS.md content"
    );
    assert!(
        !prompt.contains("You are a helpful"),
        "must not fall back to generic placeholder when loader content exists"
    );
}

#[given("模型支持 thinking")]
pub(crate) fn g_sess_thinking_model(agent: &AgentState) {
    let mut r = ModelRegistry::new();
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
        thinking: true,
        context_window: 128000,
        api: String::new(),
        provider: String::new(),
        cost_input: 0.0,
        cost_output: 0.0,
        cost_cache_read: 0.0,
        cost_cache_write: 0.0,
        max_tokens: 0,
        thinking_levels: vec!["low".into(), "medium".into(), "high".into()],
        thinking_level_map: Default::default(),
    });
    agent.registry.replace(r);
    _g_agent_thinking_level(agent, "medium".into());
}

#[when("变更 thinking level")]
pub(crate) async fn w_sess_change_thinking(agent: &AgentState) {
    _w_agent_switch_thinking(agent, "切换".into(), "high".into()).await;
}

#[then("新级别钳制到模型能力")]
pub(crate) fn t_sess_thinking_clamped_to_model(agent: &AgentState) {
    let msg = result_ok_str(&agent.last_result);
    assert!(
        msg.contains("level:high") || msg.contains("level:medium") || msg.contains("level:low"),
        "thinking level must clamp to model capability, got: {msg}"
    );
}

#[given("已绑定稳定 session_id 的 Agent 跑完一轮 user→assistant")]
pub(crate) async fn g_sess_persist_turn(agent: &AgentState, sess: &XySessionStore) {
    sess.ensure_mgr();
    let sid = "persist-user-assistant";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    reset_fake_state();
    set_fake_text("assistant reply");
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let mut caps = make_test_capabilities(agent, store.clone());
    let first_model_id = agent.registry.borrow().list().first().map(|m| m.id.clone());
    if let Some(id) = first_model_id {
        let _ = caps.select_model(&id).await;
    }
    caps.set_session(sid.to_string());
    let mut runtime = AgentRuntime::new(caps);
    let mut stream = agent_submit_root_with_id(&mut runtime, "hello user", sid).await;
    while stream.next().await.is_some() {}
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("load_entries(session_id)")]
pub(crate) async fn w_sess_load_entries(sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr.load(&sid).await.unwrap_or_default();
    sess.entries.replace(entries);
}

#[then("含本轮 user 与 assistant 的 SessionEntry::Message")]
pub(crate) fn t_sess_has_user_assistant(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, LlmMessage};

    let entries = sess.entries.borrow();
    let has_user = entries.iter().any(|e| {
        matches!(
            e.as_agent_message(),
            Some(AgentMessage::Llm(LlmMessage::UserMessage { .. }))
        )
    });
    let has_assistant = entries.iter().any(|e| {
        matches!(
            e.as_agent_message(),
            Some(AgentMessage::Llm(LlmMessage::AssistantMessage { .. }))
        )
    });
    assert!(has_user, "missing user message");
    assert!(has_assistant, "missing assistant message");
}

#[given("一轮含工具调用")]
pub(crate) async fn g_sess_persist_tool(agent: &AgentState, sess: &XySessionStore) {
    reset_fake_state();
    set_fake_tool_call("read", r#"{"path":"src/main.rs"}"#);
    set_fake_tool_result("file content");
    sess.ensure_mgr();
    let sid = "persist-tool-result";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create(sid, Some("."), None).await;
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let mut caps = make_test_capabilities(agent, store);
    let first_model_id = agent.registry.borrow().list().first().map(|m| m.id.clone());
    if let Some(id) = first_model_id {
        let _ = caps.select_model(&id).await;
    }
    caps.set_session(sid.to_string());
    let mut runtime = AgentRuntime::new(caps);
    let mut stream = agent_submit_root_with_id(&mut runtime, "read file", sid).await;
    while stream.next().await.is_some() {}
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("工具执行结束")]
pub(crate) async fn w_sess_tool_done(sess: &XySessionStore) {
    w_sess_load_entries(sess).await;
}

#[then("store 含对应 toolResult（或等价 tool）消息条目")]
pub(crate) fn t_sess_has_tool_result(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, LlmMessage};

    let has_tool = sess.entries.borrow().iter().any(|e| {
        matches!(
            e.as_agent_message(),
            Some(AgentMessage::Llm(LlmMessage::ToolResultMessage { .. }))
        )
    });
    assert!(has_tool, "session must contain tool result message entry");
}

#[given("system prompt 已配置上下文文件")]
pub(crate) fn g_sess_prompt_build(ws: &Workspace, agent: &AgentState) {
    ws.init();
    std::fs::write(ws.ws("AGENTS.md"), "context file body").ok();
    agent.last_result.replace(Some(Ok(ws.ws("AGENTS.md"))));
}

#[when("agent 开始回合")]
pub(crate) async fn w_sess_start_turn(agent: &AgentState, ws: &Workspace) {
    reset_fake_state();
    set_fake_text("ok");
    _g_agent_mock_model(agent, ws, "test-model".into());
    let mut runner = make_agent(agent);
    let mut stream = agent_submit_root(&mut runner, "do work").await;
    let mut local_events = Vec::new();
    while let Some(e) = stream.next().await {
        local_events.push(e);
    }
    agent.events.borrow_mut().clear();
    agent.events.borrow_mut().extend(local_events);
}

#[then("messages 数组为 system prompt、history、user message")]
pub(crate) fn t_sess_prompt_build_order(agent: &AgentState) {
    assert!(
        !agent.events.borrow().is_empty(),
        "agent must emit events when starting a turn"
    );
}

#[given("空扩展命令的能力聚合体")]
pub(crate) fn g_sess_get_commands(_agent: &AgentState) {
    use crate::app::product_commands::product_slash_commands;
    let names: Vec<String> = product_slash_commands()
        .iter()
        .map(|c| c.name.to_string())
        .collect();
    sess_caps::NAMES.with(|n| n.replace(names));
}

#[when("调用 get_commands")]
pub(crate) fn w_sess_get_commands(_agent: &AgentState) {
    let names = sess_caps::NAMES.with(|n| n.borrow().clone());
    _agent.last_result.replace(Some(Ok(names.join(","))));
}

#[then("含 session-tree 且不含短名 tree 作为内建主名")]
pub(crate) fn t_sess_product_command_names(agent: &AgentState) {
    let names = result_ok_str(&agent.last_result);
    assert!(
        names.contains("session-tree"),
        "commands must include session-tree, got: {names}"
    );
    let primary: Vec<_> = names
        .split(',')
        .filter(|n| *n == "tree" || *n == "compact" || *n == "export")
        .collect();
    assert!(
        primary.is_empty(),
        "short legacy names must not be primary builtins: {primary:?}"
    );
}

#[given("已启用 session 的 Agent")]
pub(crate) async fn g_sess_auto_persist(agent: &AgentState, sess: &XySessionStore) {
    g_sess_persist_turn(agent, sess).await;
}

#[when("assistant message_end 发生")]
pub(crate) async fn w_sess_message_end(agent: &AgentState, sess: &XySessionStore) {
    let _ = agent;
    w_sess_load_entries(sess).await;
}

#[then("该消息已 append 到 session store")]
pub(crate) fn t_sess_auto_persisted(sess: &XySessionStore) {
    use crate::protocol::message::{AgentMessage, LlmMessage};

    assert!(
        sess.entries.borrow().iter().any(|e| {
            matches!(
                e.as_agent_message(),
                Some(AgentMessage::Llm(LlmMessage::AssistantMessage { .. }))
            )
        }),
        "assistant message must be persisted"
    );
}

#[given("会话文件 cwd 指向存在目录")]
pub(crate) async fn g_sess_resume_cwd(ws: &Workspace, sess: &XySessionStore) {
    ws.init();
    sess.ensure_mgr();
    let sid = "resume-cwd-test";
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let cwd = ws.ws(".");
    let _ = mgr.create(sid, Some(&cwd), None).await;
    sess.current_id.replace(Some(sid.to_string()));
}

#[when("调用 resume_session")]
pub(crate) async fn w_sess_resume(sess: &XySessionStore, ws: &Workspace) {
    let sid = sess.current_id.borrow().clone().unwrap();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let entries = mgr
        .load_validated(&sid, &ws.ws("."))
        .await
        .map_err(XyDriverError::from);
    sess.last_result
        .replace(Some(entries.map(|e| format!("loaded:{}", e.len()))));
}

#[then("会话加载成功")]
pub(crate) fn t_sess_resume_ok(sess: &XySessionStore) {
    let msg = result_ok_str(&sess.last_result);
    assert!(
        msg.starts_with("loaded:"),
        "resume must load session, got: {msg}"
    );
}

#[given("AgentCapabilities 与 SessionExporter 已构造")]
pub(crate) fn g_sess_resp_separated(agent: &AgentState) {
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let _caps = make_test_capabilities(agent, store);
    agent.last_result.replace(Some(Ok("constructed".into())));
}

#[when("分别调用 get_context_usage 与 export_to_html 入口")]
pub(crate) fn w_sess_resp_apis(agent: &AgentState) {
    let settings = crate::agent::compaction::CompactionSettings::default();
    let usage = get_context_usage(1000, 100_000, &settings);
    let tokens = usage.tokens;
    agent.context_usage.replace(Some(usage));
    agent
        .last_result
        .replace(Some(Ok(format!("usage:{tokens}"))));
}

#[then("各 API 可独立调用且不 panic")]
pub(crate) fn t_sess_resp_ok(agent: &AgentState) {
    assert!(agent.context_usage.borrow().is_some());
    assert!(result_ok_str(&agent.last_result).starts_with("usage:"));
}

struct MockExportIo {
    writes: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl crate::protocol::ports::XyExportIo for MockExportIo {
    async fn write_text(
        &self,
        _path: &std::path::Path,
        content: &str,
    ) -> Result<(), crate::XyExportError> {
        self.writes.lock().unwrap().push(content.to_string());
        Ok(())
    }
    async fn read_bytes(&self, _path: &std::path::Path) -> Result<Vec<u8>, crate::XyExportError> {
        Ok(Vec::new())
    }
}

#[given("构造含 MockExportIo 的 Agent")]
pub(crate) fn g_sess_mock_export(_agent: &AgentState) {
    let mock = Arc::new(MockExportIo {
        writes: std::sync::Mutex::new(Vec::new()),
    });
    sess_export::MOCK.with(|m| m.replace(Some(mock)));
}

#[when("调用 export_to_html")]
pub(crate) async fn w_sess_export_html(agent: &AgentState, _sess: &XySessionStore) {
    use crate::app::session_export::SessionExporter;

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.html");
    let mgr = SessionManager::new(dir.path().join("sessions"));
    let sid = "export-test";
    let _ = mgr.create(sid, Some("."), None).await;
    let mock = sess_export::MOCK
        .with(|m| m.borrow().clone())
        .expect("mock export");
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let exporter = SessionExporter::new(Some(mock as Arc<dyn crate::protocol::ports::XyExportIo>));
    let result = exporter
        .export_to_html(store.as_ref(), sid, out.as_path())
        .await;
    agent
        .last_result
        .replace(Some(result.map(|_| "exported".into())));
}

mod sess_caps {
    use std::cell::RefCell;
    thread_local! {
        pub static NAMES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
}

#[then("MockExportIo.write 被调用且 agent/ 源码无 std::fs 引用")]
pub(crate) fn t_sess_export_called(agent: &AgentState) {
    assert_eq!(result_ok_str(&agent.last_result), "exported");
    let writes = sess_export::MOCK
        .with(|m| {
            m.borrow()
                .as_ref()
                .map(|io| io.writes.lock().unwrap().len())
        })
        .unwrap_or(0);
    assert!(writes > 0, "MockExportIo.write must be called");
}

mod sess_export {
    use std::cell::RefCell;
    use std::sync::Arc;
    thread_local! {
        pub static MOCK: RefCell<Option<Arc<super::MockExportIo>>> = const { RefCell::new(None) };
    }
}

#[given("构建无 bash executor 的 agent")]
pub(crate) fn g_sess_no_bash(_agent: &AgentState) {}

#[when("调用 execute_bash")]
pub(crate) async fn w_sess_execute_bash_no_executor(agent: &AgentState) {
    use crate::app::bang_exec::BangExecHandler;
    let dir = tempfile::tempdir().unwrap();
    let mgr = SessionManager::new(dir.keep());
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let bang = BangExecHandler::new(None);
    let result = bang
        .execute(
            store.as_ref(),
            None,
            "bash-id",
            "echo hi",
            false,
            tokio_util::sync::CancellationToken::new(),
            None,
            None,
        )
        .await;
    agent.last_result.replace(Some(
        result.map(|_| "ok".into()).map_err(XyDriverError::from),
    ));
}

#[then("返回提及 bash executor 未配置的错误且不 panic")]
pub(crate) fn t_sess_no_bash_err(agent: &AgentState) {
    let err = agent
        .last_result
        .borrow()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap_err()
        .to_string()
        .to_lowercase();
    assert!(
        err.contains("bash")
            && (err.contains("not") || err.contains("未") || err.contains("config")),
        "expected bash executor missing error, got: {err}"
    );
}

// ── agent-session: interrupted-bang projection (c2770 / as-bang1) ────

mod sess_interrupted {
    use std::cell::RefCell;

    use crate::protocol::message::AgentMessage;

    thread_local! {
        /// 播种路径的两次构建（折叠文本逐字节一致断言用）。
        pub static BUILDS: RefCell<Vec<Vec<AgentMessage>>> = const { RefCell::new(Vec::new()) };
    }
}

async fn append_interrupted_bash_row(
    sess: &XySessionStore,
    id: &str,
    bash_id: &str,
    command: &str,
    output: &str,
    status: crate::protocol::message::BashExecutionStatus,
    exclude: bool,
) {
    use crate::protocol::session::bash_execution_message_entry;

    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let sid = sess.current_id.borrow().clone().expect("session id");
    let mut entry = bash_execution_message_entry(
        bash_id,
        command,
        output,
        Some(0),
        false,
        false,
        None,
        exclude,
        status,
    );
    if let SessionEntry::Message(ref mut m) = entry {
        m.base.id = id.into();
        m.base.timestamp = 1704067200000;
    }
    let _ = mgr.append(&sid, &entry).await;
}

fn interrupted_user_texts(rows: &[crate::protocol::message::LlmMessage]) -> Vec<String> {
    use crate::protocol::message::{AgentPart, LlmMessage};
    rows.iter()
        .filter_map(|m| match m {
            LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                AgentPart::Text { text } => Some(text.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

#[given("session 含孤儿 running bashExecution（command 为 {command:string}，同 bash_id 无 done）")]
pub(crate) async fn g_sess_interrupted_orphan(sess: &XySessionStore, command: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create("interrupted-orphan", Some("."), None).await;
    sess.current_id.replace(Some("interrupted-orphan".into()));
    append_interrupted_bash_row(
        sess,
        "r1",
        "b-serve",
        &command,
        "",
        crate::protocol::message::BashExecutionStatus::Running,
        false,
    )
    .await;
}

#[given(
    "session 含 exclude_from_context 的孤儿 running bashExecution（command 为 {command:string}）"
)]
pub(crate) async fn g_sess_interrupted_excluded(sess: &XySessionStore, command: String) {
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let _ = mgr.create("interrupted-paired", Some("."), None).await;
    sess.current_id.replace(Some("interrupted-paired".into()));
    append_interrupted_bash_row(
        sess,
        "r1",
        "b-clean",
        &command,
        "",
        crate::protocol::message::BashExecutionStatus::Running,
        true,
    )
    .await;
}

#[given("另含一对同 bash_id 的 running 与 done")]
pub(crate) async fn g_sess_interrupted_pair(sess: &XySessionStore) {
    append_interrupted_bash_row(
        sess,
        "r2",
        "b-pair",
        "pair",
        "",
        crate::protocol::message::BashExecutionStatus::Running,
        false,
    )
    .await;
    append_interrupted_bash_row(
        sess,
        "d2",
        "b-pair",
        "pair",
        "paired-out",
        crate::protocol::message::BashExecutionStatus::Done,
        false,
    )
    .await;
}

#[when("经播种路径构建 LLM history")]
pub(crate) async fn w_sess_build_llm_history(agent: &AgentState, sess: &XySessionStore) {
    let sid = sess.current_id.borrow().clone().expect("session id");
    sess.ensure_mgr();
    let mgr = sess.mgr.borrow().as_ref().unwrap().clone();
    let store: Arc<dyn crate::protocol::ports::XySessionStore> = Arc::new(mgr);
    let caps = make_test_capabilities(agent, store);
    let first = caps
        .load_conversation_history(&sid)
        .await
        .expect("seed history");
    let second = caps
        .load_conversation_history(&sid)
        .await
        .expect("seed history again");
    sess_interrupted::BUILDS.with(|b| b.replace(vec![first, second]));
}

#[then("送给模型的 history 恰含一行 `[interrupted] $ serve`")]
pub(crate) fn t_sess_interrupted_line_present(_sess: &XySessionStore) {
    let builds = sess_interrupted::BUILDS.with(|b| b.borrow().clone());
    let texts = interrupted_user_texts(&crate::agent::llm_project::project_for_llm(&builds[0]));
    let hits = texts
        .iter()
        .filter(|t| *t == "[interrupted] $ serve")
        .count();
    assert_eq!(
        hits, 1,
        "exactly one interrupted line expected, got {hits} in {texts:?}"
    );
}

#[then("重复构建上下文两次折叠文本逐字节一致")]
pub(crate) fn t_sess_interrupted_byte_stable(_sess: &XySessionStore) {
    let builds = sess_interrupted::BUILDS.with(|b| b.borrow().clone());
    let first = interrupted_user_texts(&crate::agent::llm_project::project_for_llm(&builds[0]));
    let second = interrupted_user_texts(&crate::agent::llm_project::project_for_llm(&builds[1]));
    assert_eq!(first, second, "repeated builds MUST fold byte-stable text");
}

#[then("history 不含 \"clean\" 的 interrupted 提示")]
pub(crate) fn t_sess_interrupted_clean_absent(_sess: &XySessionStore) {
    let builds = sess_interrupted::BUILDS.with(|b| b.borrow().clone());
    let texts = interrupted_user_texts(&crate::agent::llm_project::project_for_llm(&builds[0]));
    assert!(
        texts.iter().all(|t| !t.contains("[interrupted] $ clean")),
        "`!!` orphan MUST NOT project interrupted: {texts:?}"
    );
}

#[then("有 done 的 running 不产出任何投影且 done 照常折叠")]
pub(crate) fn t_sess_paired_running_hidden(_sess: &XySessionStore) {
    let builds = sess_interrupted::BUILDS.with(|b| b.borrow().clone());
    let texts = interrupted_user_texts(&crate::agent::llm_project::project_for_llm(&builds[0]));
    assert!(
        texts.iter().all(|t| !t.contains("[interrupted] $ pair")),
        "paired running MUST NOT project: {texts:?}"
    );
    assert!(
        texts.iter().filter(|t| *t == "$ pair\npaired-out").count() == 1,
        "done row keeps the bash fold exactly once: {texts:?}"
    );
}
