use serial_test::serial;
use std::sync::Arc;

use super::*;
use crate::agent::capabilities::AgentCapabilities;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::RunPolicy;
use crate::infra::session::SessionManager;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyModelConfig;
use crate::protocol::model::XyModelMeta;
use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore, XyStream};
use crate::protocol::session::SessionEntry;

type ModelBuilderFn = Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync>;

fn bind_session_or_panic(agent: &mut AgentRuntime, session_id: impl Into<String>) {
    agent.bind_session(session_id).expect("bind_session");
}

fn ensure_bound_session(agent: &mut AgentRuntime) {
    if agent.session_id().is_none() {
        bind_session_or_panic(agent, uuid::Uuid::new_v4().to_string());
    }
}

async fn run_agent(agent: &mut AgentRuntime, prompt: &str) -> XyEventStream {
    ensure_bound_session(agent);
    agent.submit_root(prompt, RunPolicy::Reject).await
}

async fn run_agent_with_id(agent: &mut AgentRuntime, prompt: &str, sid: &str) -> XyEventStream {
    bind_session_or_panic(agent, sid.to_string());
    agent.submit_root(prompt, RunPolicy::Reject).await
}

/// Model builder for tests — the real factory (tests register `Fake`/`OpenAi`
/// model configs and rely on `build_provider` constructing the provider struct;
/// no real network calls are made in unit assertions).
fn fake_model_builder() -> ModelBuilderFn {
    Arc::new(crate::infra::provider::factory::build_provider)
}

#[tokio::test]
#[serial(obs_global)]

async fn test_agent_session_builds_model() {
    let mut reg = ModelRegistry::new(std::sync::Arc::new(
        crate::infra::config::value::InfraSecretResolver::new(),
    ));
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "mock-model".into(),
            base_url: None,
            api: None,
            compat: None,
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

    let session_mgr = SessionManager::new(SessionManager::default_dir());
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        reg,
        ToolSet::from_iter(crate::infra::tools::default_tools()),
        store,
        sink,
        Some("You are helpful.".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        fake_model_builder(),
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    );
    session.select_model("mock").expect("select mock");

    assert!(session.current_model().is_some());
    assert_eq!(session.current_model().unwrap().id, "mock");
}

#[tokio::test]
#[serial(obs_global)]

async fn test_agent_loop_emits_events() {
    let mut reg = ModelRegistry::new(std::sync::Arc::new(
        crate::infra::config::value::InfraSecretResolver::new(),
    ));
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: crate::protocol::model::XyModelConfig {
            kind: crate::protocol::model::XyModelKind::OpenAi,
            api_key: "sk-test".into(),
            model: "mock-model".into(),
            base_url: None,
            api: None,
            compat: None,
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

    let session_mgr = SessionManager::new(SessionManager::default_dir());
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::from_iter(crate::infra::tools::default_tools()),
        store,
        sink,
        Some("You are helpful.".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        fake_model_builder(),
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));

    let mut loop_runner = AgentRuntime::new(session);
    let _stream = run_agent_with_id(&mut loop_runner, "hello", "test-session").await;
}

// ── Mock model / tool helpers for hook and snapshot tests ───────

struct MockModel {
    chunks: Vec<crate::protocol::model::XyChunk>,
    /// Without max_iterations (c1430), a constant tool-call mock would loop
    /// forever. First `generate_stream` returns `chunks`; later calls stop.
    calls: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl XyModel for MockModel {
    fn name(&self) -> &str {
        "mock"
    }

    async fn generate_stream(
        &self,
        _messages: Vec<crate::protocol::message::LlmMessage>,
        _tools: &[crate::protocol::model::XyToolSchema],
        _stream: bool,
        _options: crate::protocol::ports::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        use std::sync::atomic::Ordering;
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let chunks = if n == 0 {
            self.chunks.clone()
        } else {
            vec![
                crate::protocol::model::XyChunk::TextDelta("(mock end)".into()),
                crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                },
            ]
        };
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

struct MockTool;

#[async_trait::async_trait]
impl crate::protocol::ports::XyTool for MockTool {
    fn name(&self) -> &str {
        "mock_tool"
    }

    fn description(&self) -> &str {
        "mock"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        })
    }

    async fn execute(
        &self,
        _ctx: &crate::protocol::ports::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, crate::protocol::error::XyToolError> {
        Ok("executed".into())
    }
}

/// Emits multiple live output chunks before returning (bash-like uplink).
struct StreamingMockTool;

#[async_trait::async_trait]
impl crate::protocol::ports::XyTool for StreamingMockTool {
    fn name(&self) -> &str {
        "mock_tool"
    }

    fn description(&self) -> &str {
        "streaming mock"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }

    async fn execute(
        &self,
        ctx: &crate::protocol::ports::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, crate::protocol::error::XyToolError> {
        if let Some(tx) = &ctx.output_tx {
            for part in ["chunk-a\n", "chunk-b\n", "chunk-c\n"] {
                let _ = tx.send(part.into()).await;
                tokio::task::yield_now().await;
            }
        }
        Ok("done".into())
    }
}

fn mock_model_registry() -> ModelRegistry {
    let mut reg = ModelRegistry::new(std::sync::Arc::new(
        crate::infra::config::value::InfraSecretResolver::new(),
    ));
    reg.register(XyModelMeta {
        id: "mock".into(),
        config: XyModelConfig {
            kind: crate::protocol::model::XyModelKind::Fake,
            api_key: String::new(),
            model: "mock".into(),
            base_url: None,
            api: None,
            compat: None,
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
    reg
}

fn select_mock(mut session: AgentCapabilities) -> AgentCapabilities {
    session
        .select_model("mock")
        .expect("select mock model for react tests");
    session
}

fn mock_model_builder(chunks: Vec<crate::protocol::model::XyChunk>) -> ModelBuilderFn {
    Arc::new(move |_| {
        Ok(Arc::new(MockModel {
            chunks: chunks.clone(),
            calls: std::sync::atomic::AtomicUsize::new(0),
        }) as Arc<dyn XyModel>)
    })
}

fn make_agent_with_tools(
    chunks: Vec<crate::protocol::model::XyChunk>,
    tools: ToolSet,
) -> AgentRuntime {
    make_agent_with_tools_and_store(chunks, tools).0
}

fn make_agent_with_tools_and_store(
    chunks: Vec<crate::protocol::model::XyChunk>,
    tools: ToolSet,
) -> (AgentRuntime, Arc<dyn XySessionStore>) {
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let mut session = AgentCapabilities::new(
        reg,
        tools,
        Arc::clone(&store),
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        mock_model_builder(chunks),
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    );
    session
        .select_model("mock")
        .expect("select mock model for react tests");
    (AgentRuntime::new(session), store)
}

#[tokio::test]
#[serial(obs_global)]

async fn test_persist_done_usage() {
    use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason, XyUsage};
    use futures::StreamExt;

    let usage = XyUsage {
        input: 11,
        output: 7,
        total_tokens: 18,
        ..Default::default()
    };
    let chunks = vec![
        crate::protocol::model::XyChunk::TextDelta("hi".into()),
        crate::protocol::model::XyChunk::Done {
            finish_reason: XyStopReason::Stop,
            usage: Some(usage),
        },
    ];
    let (mut agent, store) = make_agent_with_tools_and_store(chunks, ToolSet::from_iter([]));
    let mut stream = run_agent_with_id(&mut agent, "ping", "sess-usage").await;
    while stream.next().await.is_some() {}

    let entries = store.load_entries("sess-usage").await.expect("entries");
    let mut found = false;
    for entry in entries {
        let SessionEntry::Message(m) = entry else {
            continue;
        };
        let Ok(AgentMessage::Llm(LlmMessage::AssistantMessage {
            usage: Some(u),
            stop_reason: Some(XyStopReason::Stop),
            ..
        })) = serde_json::from_value(m.message)
        else {
            continue;
        };
        assert_eq!(u.input, 11);
        assert_eq!(u.output, 7);
        found = true;
    }
    assert!(found, "expected persisted assistant with Done.usage");
}

#[tokio::test]
#[serial(obs_global)]

async fn test_tool_intent_before_execution() {
    use crate::protocol::lifecycle::XyEvent;
    use crate::protocol::message::{AgentMessage, AgentPart, LlmMessage};
    use futures::StreamExt;

    let chunks = vec![
        crate::protocol::model::XyChunk::ToolCallStart {
            id: "call-1".into(),
            name: "mock_tool".into(),
        },
        crate::protocol::model::XyChunk::ToolCallDelta {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args_delta: r#"{"input":"#.into(),
            args: serde_json::json!({"input": ""}),
        },
        crate::protocol::model::XyChunk::ToolCallDelta {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args_delta: r#"x"}"#.into(),
            args: serde_json::json!({"input": "x"}),
        },
        crate::protocol::model::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        },
        crate::protocol::model::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::ToolUse,
            usage: None,
        },
    ];
    let mut agent = make_agent_with_tools(
        chunks,
        ToolSet::from_iter(vec![
            Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );

    let mut stream = run_agent(&mut agent, "go").await;
    let mut saw_intent_update = false;
    let mut message_end_seen = false;
    let mut tool_start_after_end = false;
    let mut tool_update_seen = false;

    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::MessageUpdate {
                message: Some(AgentMessage::Llm(LlmMessage::AssistantMessage { content, .. })),
                ..
            } if !message_end_seen
                && content.iter().any(|p| {
                    matches!(
                        p,
                        AgentPart::ToolCall {
                            name,
                            ..
                        } if name == "mock_tool"
                    )
                }) =>
            {
                saw_intent_update = true;
            }
            XyEvent::MessageEnd { .. } => {
                message_end_seen = true;
                assert!(
                    saw_intent_update,
                    "expected MessageUpdate with ToolCall before MessageEnd"
                );
            }
            XyEvent::ToolExecutionStart { .. } => {
                assert!(
                    message_end_seen,
                    "ToolExecutionStart must not precede MessageEnd"
                );
                tool_start_after_end = true;
            }
            XyEvent::ToolExecutionUpdate { .. } => {
                tool_update_seen = true;
            }
            _ => {}
        }
    }

    assert!(
        saw_intent_update,
        "expected streaming tool intent MessageUpdate"
    );
    assert!(
        tool_start_after_end,
        "expected ToolExecutionStart after MessageEnd"
    );
    assert!(
        tool_update_seen,
        "expected ToolExecutionUpdate during execution"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn test_tool_execution_streams_multiple_updates() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let chunks = vec![
        crate::protocol::model::XyChunk::ToolCallEnd {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({}),
        },
        crate::protocol::model::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::ToolUse,
            usage: None,
        },
    ];
    let mut agent = make_agent_with_tools(
        chunks,
        ToolSet::from_iter(vec![
            Arc::new(StreamingMockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );

    let mut stream = run_agent(&mut agent, "go").await;
    let mut updates = Vec::new();
    let mut saw_end = false;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::ToolExecutionUpdate { output, .. } => updates.push(output),
            XyEvent::ToolExecutionEnd { .. } => saw_end = true,
            _ => {}
        }
    }
    assert!(
        updates.len() >= 3,
        "expected ≥3 live updates, got {updates:?}"
    );
    assert_eq!(updates[0], "chunk-a\n");
    assert_eq!(updates[1], "chunk-b\n");
    assert_eq!(updates[2], "chunk-c\n");
    assert!(saw_end, "expected ToolExecutionEnd");
}

#[tokio::test]
#[serial(obs_global)]

async fn tool_execute_err_ends_with_tool_end_not_global_error() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    // Missing tool → ExecutionFailed; surfaces must not get a second XyEvent::Error.
    let chunks = vec![
        crate::protocol::model::XyChunk::ToolCallEnd {
            id: "call-missing".into(),
            name: "no_such_tool".into(),
            args: serde_json::json!({}),
        },
        crate::protocol::model::XyChunk::Done {
            finish_reason: crate::protocol::message::XyStopReason::ToolUse,
            usage: None,
        },
    ];
    let mut agent = make_agent_with_tools(chunks, ToolSet::empty());

    let mut stream = run_agent(&mut agent, "go").await;
    let mut tool_ends = Vec::new();
    let mut global_errors = Vec::new();
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::ToolExecutionEnd {
                name,
                result,
                is_error,
                ..
            } => tool_ends.push((name, result, is_error)),
            XyEvent::Error(err) => global_errors.push(err.message),
            _ => {}
        }
    }
    assert_eq!(
        tool_ends.len(),
        1,
        "expected one ToolExecutionEnd: {tool_ends:?}"
    );
    assert_eq!(tool_ends[0].0, "no_such_tool");
    assert!(tool_ends[0].2, "is_error");
    assert!(
        tool_ends[0].1.contains("Unknown tool"),
        "result: {}",
        tool_ends[0].1
    );
    assert!(
        global_errors.is_empty(),
        "tool failure must not also yield XyEvent::Error: {global_errors:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn test_before_hook_denies_tool_call() {
    use crate::agent::runtime::hooks::BeforeToolHook;
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let chunks = vec![crate::protocol::model::XyChunk::ToolCallEnd {
        id: "call-1".into(),
        name: "mock_tool".into(),
        args: serde_json::json!({"input": "x"}),
    }];
    let mut agent = make_agent_with_tools(
        chunks,
        ToolSet::from_iter(vec![
            Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );

    let hook: BeforeToolHook = Arc::new(|name, _id, _args| {
        if name == "mock_tool" {
            Some("denied by test hook".into())
        } else {
            None
        }
    });
    agent.add_hook(hook);

    let mut stream = run_agent(&mut agent, "go").await;
    let mut found = false;
    while let Some(evt) = stream.next().await {
        if let XyEvent::ToolExecutionEnd {
            name,
            result,
            is_error,
            id: _,
        } = evt
        {
            assert_eq!(name, "mock_tool");
            assert!(is_error);
            assert!(result.contains("denied by test hook"));
            found = true;
        }
    }
    assert!(found, "expected a denied tool execution event");
}

#[tokio::test]
#[serial(obs_global)]

async fn test_after_hook_modifies_tool_result() {
    use crate::agent::runtime::hooks::AfterToolHook;
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let chunks = vec![crate::protocol::model::XyChunk::ToolCallEnd {
        id: "call-1".into(),
        name: "mock_tool".into(),
        args: serde_json::json!({"input": "x"}),
    }];
    let hooks = {
        let mut h = AgentHooks::empty();
        let after: AfterToolHook = Arc::new(|_name, _id, _result, _is_error| {
            Some((serde_json::Value::String("modified".into()), false))
        });
        h.add_after(after);
        h
    };

    let mut agent = make_agent_with_tools(
        chunks,
        ToolSet::from_iter(vec![
            Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );
    agent.replace_hooks(hooks);

    let mut stream = run_agent(&mut agent, "go").await;
    let mut found = false;
    while let Some(evt) = stream.next().await {
        if let XyEvent::ToolExecutionEnd {
            name,
            result,
            is_error,
            id: _,
        } = evt
        {
            assert_eq!(name, "mock_tool");
            assert!(!is_error);
            assert_eq!(result, "modified");
            found = true;
        }
    }
    assert!(found, "expected a modified tool execution event");
}

#[tokio::test]
#[serial(obs_global)]

async fn test_set_tools_takes_effect_on_next_turn() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let chunks = vec![crate::protocol::model::XyChunk::ToolCallEnd {
        id: "call-1".into(),
        name: "mock_tool".into(),
        args: serde_json::json!({"input": "x"}),
    }];
    let mut agent = make_agent_with_tools(
        chunks.clone(),
        ToolSet::from_iter(vec![
            Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );

    // First turn: mock_tool is available.
    let mut stream = run_agent(&mut agent, "go").await;
    let mut first_turn_executed = false;
    while let Some(evt) = stream.next().await {
        if let XyEvent::ToolExecutionEnd {
            ref name, is_error, ..
        } = evt
        {
            assert_eq!(name, "mock_tool");
            assert!(!is_error);
            first_turn_executed = true;
        }
    }
    assert!(first_turn_executed);

    // Replace tools with an empty set between turns (Unfrozen / non-FROZEN path).
    // After c1900 freeze, settle/hot-merge MUST use freeze_tools / reopen — set_tools
    // is ignored while FROZEN (see session::freeze_then_set_tools_does_not_expand).
    agent.set_tools(ToolSet::empty());

    // Second turn: the loop still saw mock_tool in the original snapshot if
    // we had mutated it mid-stream, but because setters apply to the next
    // turn, this turn should report the tool as unknown.
    let mut stream = run_agent(&mut agent, "go").await;
    let mut second_turn_error = false;
    while let Some(evt) = stream.next().await {
        if let XyEvent::ToolExecutionEnd {
            ref name, is_error, ..
        } = evt
        {
            assert_eq!(name, "mock_tool");
            assert!(is_error);
            second_turn_error = true;
        }
    }
    assert!(second_turn_error);
}

// ── Multi-round tool ReAct (真 bug 复现) ──────────────────────
//
// The bug: react.rs:455 `if done { break }` treated XyChunk::Done (which
// just marks the end of ONE model stream) as the end of the whole turn.
// Providers emit Done right after a FunctionCall (openai.rs:172), so a
// tool-calling turn broke out of the for-loop after executing the tool,
// and the model never got a continuation round with the tool result.
// Correct ReAct: keep looping while the model calls tools; stop only when
// a round produces NO tool call (pure text reply = done).

/// A stateful mock that returns a DIFFERENT chunk sequence per model call,
/// so a multi-round turn (tool call → text reply) can be exercised. Each
/// `generate_stream` call pops the front sequence.
struct StatefulMockModel {
    rounds: std::sync::Mutex<Vec<Vec<crate::protocol::model::XyChunk>>>,
}
#[async_trait::async_trait]
impl XyModel for StatefulMockModel {
    fn name(&self) -> &str {
        "stateful-mock"
    }
    async fn generate_stream(
        &self,
        _messages: Vec<crate::protocol::message::LlmMessage>,
        _tools: &[crate::protocol::model::XyToolSchema],
        _stream: bool,
        _options: crate::protocol::ports::XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let chunks = self.rounds.lock().unwrap().remove(0);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

fn make_agent_with_rounds(
    rounds: Vec<Vec<crate::protocol::model::XyChunk>>,
    tools: ToolSet,
) -> AgentRuntime {
    use crate::protocol::ports::XyModelBuilder;
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(StatefulMockModel {
            rounds: std::sync::Mutex::new(rounds.clone()),
        }) as Arc<dyn XyModel>)
    });
    let mut session = AgentCapabilities::new(
        reg,
        tools,
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    );
    session
        .select_model("mock")
        .expect("select mock model for round tests");
    AgentRuntime::new(session)
}

#[tokio::test]
#[serial(obs_global)]

async fn tool_call_then_continuation_round_reaches_final_text() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    // Round 1: model calls a tool. The provider appends Done after the
    // FunctionCall (openai.rs:156-176), so Done sets `done=true` — this is
    // exactly the case where the old `if done { break }` wrongly aborted.
    // Round 2: model gives the final text reply (no tool call) + Done.
    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let rounds = vec![
        vec![
            crate::protocol::model::XyChunk::ToolCallEnd {
                id: "call-1".into(),
                name: "mock_tool".into(),
                args: serde_json::json!({"input": "x"}),
            },
            done_stop(),
        ],
        vec![
            crate::protocol::model::XyChunk::TextDelta("the answer is 42".into()),
            done_stop(),
        ],
    ];
    let mut agent = make_agent_with_rounds(
        rounds,
        ToolSet::from_iter(vec![
            Arc::new(MockTool) as Arc<dyn crate::protocol::ports::XyTool>
        ]),
    );

    let mut stream = run_agent(&mut agent, "go").await;
    let mut saw_tool = false;
    let mut saw_final_text = false;
    let mut turn_end_count = 0;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::ToolExecutionEnd { .. } => saw_tool = true,
            XyEvent::TextDelta(t) if t.contains("the answer is 42") => saw_final_text = true,
            XyEvent::TurnEnd { .. } => turn_end_count += 1,
            _ => {}
        }
    }
    assert!(saw_tool, "tool must execute in round 1");
    assert!(
        saw_final_text,
        "continuation text after the tool call MUST reach the caller (the bug dropped it)"
    );
    assert_eq!(
        turn_end_count, 2,
        "two ReAct iterations → two TurnEnd events"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn steer_before_run_is_injected_into_history() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let rounds = vec![vec![
        crate::protocol::model::XyChunk::TextDelta("ok".into()),
        done_stop(),
    ]];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    agent.steer("please be brief");

    let mut stream = run_agent(&mut agent, "hello").await;
    let mut history = Vec::new();
    while let Some(evt) = stream.next().await {
        if let XyEvent::AgentEnd { messages } = evt {
            history = messages;
        }
    }
    let texts: Vec<String> = history
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t == "please be brief"),
        "steering text must appear in history: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == "hello"));
}

#[tokio::test]
#[serial(obs_global)]

async fn follow_up_continues_after_text_only_turn() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let rounds = vec![
        vec![
            crate::protocol::model::XyChunk::TextDelta("first".into()),
            done_stop(),
        ],
        vec![
            crate::protocol::model::XyChunk::TextDelta("second".into()),
            done_stop(),
        ],
    ];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    agent.follow_up("and also this");

    let mut stream = run_agent(&mut agent, "start").await;
    let mut texts = Vec::new();
    let mut turn_ends = 0;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::TextDelta(t) => texts.push(t),
            XyEvent::TurnEnd { .. } => turn_ends += 1,
            _ => {}
        }
    }
    assert!(texts.iter().any(|t| t == "first"));
    assert!(
        texts.iter().any(|t| t == "second"),
        "follow-up must trigger a second model round: {texts:?}"
    );
    assert!(turn_ends >= 2);
}

#[tokio::test]
#[serial(obs_global)]

async fn should_stop_after_turn_skips_follow_up_and_ends() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    // Two rounds available — stop hook must prevent the second.
    let rounds = vec![
        vec![
            crate::protocol::model::XyChunk::TextDelta("first".into()),
            done_stop(),
        ],
        vec![
            crate::protocol::model::XyChunk::TextDelta("second".into()),
            done_stop(),
        ],
    ];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    agent.follow_up("queued follow-up must stay");
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let calls_hook = calls.clone();
    let seen_new = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_new_hook = seen_new.clone();
    agent.set_should_stop_after_turn(Some(std::sync::Arc::new(move |ctx| {
        calls_hook.fetch_add(1, Ordering::SeqCst);
        *seen_new_hook.lock().unwrap() = ctx.new_messages.clone();
        true
    })));

    let mut stream = run_agent(&mut agent, "start").await;
    let mut texts = Vec::new();
    let mut turn_starts = 0u32;
    let mut turn_ends = 0u32;
    let mut agent_end_msgs = None;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::TextDelta(t) => texts.push(t),
            XyEvent::TurnStart { .. } => turn_starts += 1,
            XyEvent::TurnEnd { .. } => turn_ends += 1,
            XyEvent::AgentEnd { messages } => agent_end_msgs = Some(messages),
            _ => {}
        }
    }

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "stop hook once after TurnEnd"
    );
    assert_eq!(turn_starts, 1, "no second model turn");
    assert_eq!(turn_ends, 1);
    assert!(texts.iter().any(|t| t == "first"));
    assert!(
        !texts.iter().any(|t| t == "second"),
        "second model round must not run: {texts:?}"
    );
    assert_eq!(agent.queue_stats().follow_up_count, 1);
    let new_msgs = seen_new.lock().unwrap();
    let new_user: Vec<String> = new_msgs
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    assert!(
        new_user.iter().any(|t| t == "start"),
        "new_messages must carry the run prompt (pi newMessages): {new_user:?}"
    );
    let history = agent_end_msgs.expect("AgentEnd");
    let user_texts: Vec<String> = history
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    assert!(
        !user_texts.iter().any(|t| t.contains("queued follow-up")),
        "follow_up must not be injected: {user_texts:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn abort_before_run_does_not_stick_to_next_run() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let rounds = vec![vec![
        crate::protocol::model::XyChunk::TextDelta("recovered".into()),
        done_stop(),
    ]];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    // Sticky-cancel bug: abort left the token cancelled forever.
    agent.abort();

    let mut stream = run_agent(&mut agent, "hello").await;
    let mut texts = Vec::new();
    let mut aborted = false;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::TextDelta(t) => texts.push(t),
            XyEvent::Error(err) if err.is_aborted() => aborted = true,
            _ => {}
        }
    }
    assert!(
        !aborted,
        "run() must install a fresh cancel token after abort"
    );
    assert_eq!(texts, vec!["recovered".to_string()]);
}

#[tokio::test]
#[serial(obs_global)]

async fn abort_after_completed_run_allows_second_run() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    // Each `run` rebuilds the mock from the same round template — we only
    // assert the second run is not sticky-aborted (c482).
    let rounds = vec![vec![
        crate::protocol::model::XyChunk::TextDelta("ok".into()),
        done_stop(),
    ]];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());

    let mut first = run_agent(&mut agent, "1").await;
    while first.next().await.is_some() {}

    agent.abort();

    let mut second = run_agent(&mut agent, "2").await;
    let mut texts = Vec::new();
    let mut aborted = false;
    while let Some(evt) = second.next().await {
        match evt {
            XyEvent::TextDelta(t) => texts.push(t),
            XyEvent::Error(err) if err.is_aborted() => aborted = true,
            _ => {}
        }
    }
    assert!(!aborted, "second run must not immediately abort: {texts:?}");
    assert_eq!(texts, vec!["ok".to_string()]);
}

/// Mid-stream abort MUST stop polling the model stream (c680). Without
/// `select!` on cancel inside the chunk loop, the consumer would drain all
/// slow chunks even after `abort()` — proving UI-only abort is insufficient.
#[tokio::test]
#[serial(obs_global)]

async fn abort_mid_stream_stops_polling_model_chunks() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowMock {
        polled: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let polled = self.polled.clone();
            Ok(Box::pin(async_stream::stream! {
                for i in 0..80u32 {
                    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                    polled.fetch_add(1, Ordering::SeqCst);
                    yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("c{i}")));
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let polled = Arc::new(AtomicUsize::new(0));
    let polled_for_builder = polled.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowMock {
            polled: polled_for_builder.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store.clone(),
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);

    let mut stream = run_agent(&mut agent, "go").await;
    let mut aborted = false;
    let mut text_count = 0usize;
    while let Some(evt) = stream.next().await {
        match evt {
            XyEvent::TextDelta(_) => {
                text_count += 1;
                if text_count == 1 {
                    agent.abort();
                }
            }
            XyEvent::Error(err) if err.is_aborted() => aborted = true,
            _ => {}
        }
    }

    assert!(aborted, "must surface aborted after mid-stream cancel");
    let n = polled.load(Ordering::SeqCst);
    assert!(
        n < 40,
        "abort must stop polling model chunks (polled={n}, text_count={text_count})"
    );
    assert!(
        text_count < 40,
        "UI/event consumer must not see a full drain after abort (text_count={text_count})"
    );

    // c1595: partial assistant persisted with stop_reason=Aborted.
    let sid = agent.session_id().expect("session id after run");
    let entries = store.load_entries(sid).await.expect("load entries");
    let mut found_aborted = false;
    for e in &entries {
        let SessionEntry::Message(m) = e else {
            continue;
        };
        if m.message.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }
        if m.message.get("stopReason").and_then(|r| r.as_str()) == Some("aborted")
            || m.message.get("stop_reason").and_then(|r| r.as_str()) == Some("aborted")
        {
            found_aborted = true;
            break;
        }
    }
    assert!(
        found_aborted,
        "mid-stream abort must persist assistant with aborted stop_reason: {entries:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn persist_turn_writes_user_and_assistant_messages() {
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let rounds = vec![vec![
        crate::protocol::model::XyChunk::TextDelta("hello back".into()),
        done_stop(),
    ]];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    let sid = "persist-test-session".to_string();
    bind_session_or_panic(&mut agent, sid.clone());
    let store = agent.session_store();

    let mut stream = run_agent_with_id(&mut agent, "hello", &sid).await;
    while stream.next().await.is_some() {}

    let entries = store.load_entries(&sid).await.expect("load entries");
    let messages: Vec<_> = entries
        .iter()
        .filter_map(|e| match e {
            SessionEntry::Message(m) => m.message.get("role").and_then(|r| r.as_str()),
            _ => None,
        })
        .collect();
    assert!(messages.contains(&"user"));
    assert!(messages.contains(&"assistant"));
}

#[tokio::test]
#[serial(obs_global)]

async fn second_turn_model_input_includes_first_turn_messages() {
    use futures::StreamExt;

    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };

    struct RecordingMockModel {
        seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
        chunks: Vec<crate::protocol::model::XyChunk>,
    }

    #[async_trait::async_trait]
    impl XyModel for RecordingMockModel {
        fn name(&self) -> &str {
            "recording-mock"
        }

        async fn generate_stream(
            &self,
            messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.seen.lock().unwrap().push(messages);
            let chunks = self.chunks.clone();
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: ModelBuilderFn = {
        let seen = seen.clone();
        Arc::new(move |_| {
            Ok(Arc::new(RecordingMockModel {
                seen: seen.clone(),
                chunks: vec![
                    crate::protocol::model::XyChunk::TextDelta("ok".into()),
                    done_stop(),
                ],
            }) as Arc<dyn XyModel>)
        })
    };
    let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    )));
    let sid = "multi-turn-session".to_string();
    bind_session_or_panic(&mut agent, sid.clone());

    let mut first = run_agent_with_id(&mut agent, "turn one", &sid).await;
    while first.next().await.is_some() {}

    let mut second = run_agent_with_id(&mut agent, "turn two", &sid).await;
    while second.next().await.is_some() {}

    let rounds = seen.lock().unwrap();
    assert_eq!(rounds.len(), 2, "expected two model calls");
    let second_input = rounds.last().expect("second round");
    let user_texts: Vec<String> = second_input
        .iter()
        .filter_map(|m| match m {
            LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                AgentPart::Text { text: t } => Some(t.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert!(
        user_texts.iter().any(|t| t == "turn one"),
        "second turn must include first user prompt: {user_texts:?}"
    );
    assert!(user_texts.iter().any(|t| t == "turn two"));
}

#[tokio::test]
#[serial(obs_global)]

async fn system_prompt_via_options_not_user_history() {
    use futures::StreamExt;

    struct RecordingMockModel {
        seen_msgs: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
        seen_opts: std::sync::Arc<std::sync::Mutex<Vec<crate::protocol::ports::XyGenerateOptions>>>,
    }

    #[async_trait::async_trait]
    impl XyModel for RecordingMockModel {
        fn name(&self) -> &str {
            "recording-mock"
        }

        async fn generate_stream(
            &self,
            messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.seen_msgs.lock().unwrap().push(messages);
            self.seen_opts.lock().unwrap().push(options);
            Ok(Box::pin(futures::stream::iter(vec![Ok(
                crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                },
            )])))
        }
    }

    let seen_msgs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_opts = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: ModelBuilderFn = {
        let seen_msgs = seen_msgs.clone();
        let seen_opts = seen_opts.clone();
        Arc::new(move |_| {
            Ok(Arc::new(RecordingMockModel {
                seen_msgs: seen_msgs.clone(),
                seen_opts: seen_opts.clone(),
            }) as Arc<dyn XyModel>)
        })
    };
    let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        Some("CUSTOM_SYSTEM_MARKER".into()),
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    )));

    let mut stream = run_agent(&mut agent, "real user hello").await;
    while stream.next().await.is_some() {}

    let opts = seen_opts.lock().unwrap();
    assert_eq!(opts.len(), 1);
    let sp = opts[0]
        .system_prompt
        .as_deref()
        .expect("system_prompt in options");
    assert!(
        sp.contains("CUSTOM_SYSTEM_MARKER"),
        "options must carry system: {sp}"
    );

    let rounds = seen_msgs.lock().unwrap();
    let first = &rounds[0];
    assert!(!first.is_empty(), "history must include user message");
    let first_text = match &first[0] {
        LlmMessage::UserMessage { content, .. } => content
            .iter()
            .find_map(|p| match p {
                AgentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .unwrap_or(""),
        other => panic!("first history entry must be user, got {other:?}"),
    };
    assert_eq!(first_text, "real user hello");
    assert!(!first_text.contains("CUSTOM_SYSTEM_MARKER"));
}

#[tokio::test]
#[serial(obs_global)]

async fn dollar_skill_expanded_for_model_history_stays_raw() {
    use crate::protocol::resource::SkillInfo;
    use crate::protocol::source_info::{SourceInfo, SourceOrigin, SourceScope};
    use futures::StreamExt;

    struct RecordingMockModel {
        seen: std::sync::Arc<std::sync::Mutex<Vec<Vec<crate::protocol::message::LlmMessage>>>>,
    }

    #[async_trait::async_trait]
    impl XyModel for RecordingMockModel {
        fn name(&self) -> &str {
            "recording-mock"
        }

        async fn generate_stream(
            &self,
            messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.seen.lock().unwrap().push(messages);
            Ok(Box::pin(futures::stream::iter(
                [
                    crate::protocol::model::XyChunk::TextDelta("ok".into()),
                    crate::protocol::model::XyChunk::Done {
                        finish_reason: crate::protocol::message::XyStopReason::Stop,
                        usage: None,
                    },
                ]
                .into_iter()
                .map(Ok),
            )))
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: demo\ndescription: d\n---\n\nREACT_SKILL_BODY_MARKER\n",
    )
    .unwrap();

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: ModelBuilderFn = {
        let seen = seen.clone();
        Arc::new(move |_| {
            Ok(Arc::new(RecordingMockModel { seen: seen.clone() }) as Arc<dyn XyModel>)
        })
    };
    let mut agent = AgentRuntime::new(select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    )));
    agent.apply_skills(vec![SkillInfo {
        name: "demo".into(),
        description: Some("d".into()),
        source_info: SourceInfo {
            path: skill_path,
            source: "test".into(),
            scope: SourceScope::User,
            origin: SourceOrigin::TopLevel,
            base_dir: None,
        },
        disable_model_invocation: false,
    }]);

    let mut stream = run_agent(&mut agent, "please use $demo and $nosuch").await;
    let mut history = Vec::new();
    while let Some(evt) = stream.next().await {
        if let XyEvent::AgentEnd { messages } = evt {
            history = messages;
        }
    }

    let rounds = seen.lock().unwrap();
    assert_eq!(rounds.len(), 1);
    let model_user: Vec<String> = rounds[0]
        .iter()
        .filter_map(|m| match m {
            LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                AgentPart::Text { text: t } => Some(t.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert!(
        model_user
            .iter()
            .any(|t| t.contains("REACT_SKILL_BODY_MARKER") && t.contains("$demo")),
        "model input must expand known $skill: {model_user:?}"
    );
    assert!(
        model_user.iter().any(|t| t.contains("$nosuch")),
        "unknown $ must pass through: {model_user:?}"
    );

    let hist_user: Vec<String> = history
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(LlmMessage::UserMessage { content, .. }) => {
                content.iter().find_map(|p| match p {
                    AgentPart::Text { text: t } => Some(t.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    assert!(
        hist_user
            .iter()
            .any(|t| t == "please use $demo and $nosuch"),
        "session history must stay raw: {hist_user:?}"
    );
    assert!(
        !hist_user
            .iter()
            .any(|t| t.contains("REACT_SKILL_BODY_MARKER")),
        "session history must not persist expanded body: {hist_user:?}"
    );
}

// ── c1545 tool batch (S1–S5 harness) ─────────────────────────────

use std::sync::Mutex;
use std::time::{Duration, Instant};

struct SlowTool {
    name: &'static str,
    mode: crate::protocol::ports::XyToolExecutionMode,
    sleep_ms: u64,
    /// Shared wall-clock log: (name, start_ms, end_ms) from a fixed epoch.
    log: Arc<Mutex<Vec<(String, u128, u128)>>>,
    epoch: Instant,
}

#[async_trait::async_trait]
impl crate::protocol::ports::XyTool for SlowTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "slow test tool"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({"type": "object", "properties": {}})
    }
    fn execution_mode(&self) -> crate::protocol::ports::XyToolExecutionMode {
        self.mode
    }
    async fn execute(
        &self,
        _ctx: &crate::protocol::ports::XyToolCtx,
        _args: serde_json::Value,
    ) -> Result<String, crate::protocol::error::XyToolError> {
        let start = self.epoch.elapsed().as_millis();
        tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
        let end = self.epoch.elapsed().as_millis();
        self.log
            .lock()
            .unwrap()
            .push((self.name.to_string(), start, end));
        Ok(format!("{}-done", self.name))
    }
}

fn multi_tool_rounds(calls: Vec<(&str, &str)>) -> Vec<Vec<crate::protocol::model::XyChunk>> {
    let done_stop = || crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    };
    let mut round1 = Vec::new();
    for (i, (name, args_json)) in calls.iter().enumerate() {
        let args: serde_json::Value =
            serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
        round1.push(crate::protocol::model::XyChunk::ToolCallEnd {
            id: format!("call-{i}"),
            name: (*name).into(),
            args,
        });
    }
    round1.push(done_stop());
    vec![
        round1,
        vec![
            crate::protocol::model::XyChunk::TextDelta("done".into()),
            done_stop(),
        ],
    ]
}

fn overlaps(a: (u128, u128), b: (u128, u128)) -> bool {
    a.0 < b.1 && b.0 < a.1
}

#[tokio::test]
#[serial(obs_global)]

async fn batch_default_sequential_no_overlap() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let epoch = Instant::now();
    let tools = ToolSet::from_iter(vec![Arc::new(SlowTool {
        name: "slow_safe",
        mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
        sleep_ms: 80,
        log: log.clone(),
        epoch,
    }) as Arc<dyn crate::protocol::ports::XyTool>]);
    // Two calls to the same ParallelSafe tool — Sequential batch must not overlap.
    let rounds = multi_tool_rounds(vec![
        ("slow_safe", r#"{"n":1}"#),
        ("slow_safe", r#"{"n":2}"#),
    ]);
    let mut agent = make_agent_with_rounds(rounds, tools);
    agent.set_batch_mode(XyBatchMode::Sequential);
    let mut stream = run_agent(&mut agent, "go").await;
    let mut ends = Vec::new();
    while let Some(ev) = stream.next().await {
        if let XyEvent::ToolExecutionEnd { id, .. } = ev {
            ends.push(id);
        }
    }
    assert_eq!(ends, vec!["call-0".to_string(), "call-1".to_string()]);
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries.len(), 2);
    assert!(
        !overlaps((entries[0].1, entries[0].2), (entries[1].1, entries[1].2)),
        "Sequential must not overlap: {entries:?}"
    );
    assert!(
        entries[0].2 <= entries[1].1,
        "source-order serial: {entries:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn batch_barrier_parallel_overlap_then_barrier() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let epoch = Instant::now();
    let tools = ToolSet::from_iter(vec![
        Arc::new(SlowTool {
            name: "slow_safe",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 100,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
        Arc::new(SlowTool {
            name: "slow_barrier",
            mode: crate::protocol::ports::XyToolExecutionMode::Sequential,
            sleep_ms: 50,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
    ]);
    let rounds = multi_tool_rounds(vec![
        ("slow_safe", r#"{"n":1}"#),
        ("slow_safe", r#"{"n":2}"#),
        ("slow_barrier", r#"{}"#),
    ]);
    let mut agent = make_agent_with_rounds(rounds, tools);
    agent.set_batch_mode(XyBatchMode::BarrierParallel);
    let t0 = Instant::now();
    let mut stream = run_agent(&mut agent, "go").await;
    while stream.next().await.is_some() {}
    let elapsed = t0.elapsed();
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries.len(), 3, "{entries:?}");
    let by = |n: &str| {
        entries
            .iter()
            .find(|(name, _, _)| name == n)
            .cloned()
            .unwrap_or_else(|| panic!("missing {n} in {entries:?}"))
    };
    // Two slow_safe — find both by order of log push (start order may race).
    let safes: Vec<_> = entries
        .iter()
        .filter(|(n, _, _)| n == "slow_safe")
        .cloned()
        .collect();
    assert_eq!(safes.len(), 2);
    assert!(
        overlaps((safes[0].1, safes[0].2), (safes[1].1, safes[1].2)),
        "ParallelSafe window must overlap: {entries:?}"
    );
    let barrier = by("slow_barrier");
    let safe_end_max = safes.iter().map(|e| e.2).max().unwrap();
    assert!(
        safe_end_max <= barrier.1,
        "both safes must finish before barrier starts: {entries:?}"
    );
    // S3: wall clock ≪ 200ms serial (two 100ms safes).
    assert!(
        elapsed < Duration::from_millis(280),
        "expected parallel speedup, elapsed={elapsed:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn batch_barrier_preserves_source_windows() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let epoch = Instant::now();
    let tools = ToolSet::from_iter(vec![
        Arc::new(SlowTool {
            name: "slow_safe",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 60,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
        Arc::new(SlowTool {
            name: "slow_barrier",
            mode: crate::protocol::ports::XyToolExecutionMode::Sequential,
            sleep_ms: 40,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
    ]);
    // safe → barrier → safe : second safe must start after barrier ends.
    let rounds = multi_tool_rounds(vec![
        ("slow_safe", r#"{"n":1}"#),
        ("slow_barrier", r#"{}"#),
        ("slow_safe", r#"{"n":2}"#),
    ]);
    let mut agent = make_agent_with_rounds(rounds, tools);
    agent.set_batch_mode(XyBatchMode::BarrierParallel);
    let mut stream = run_agent(&mut agent, "go").await;
    while stream.next().await.is_some() {}
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries.len(), 3, "{entries:?}");
    // Log order = start order for serial barriers between safes.
    let first_safe = &entries[0];
    let barrier = entries
        .iter()
        .find(|(n, _, _)| n == "slow_barrier")
        .unwrap();
    let second_safe = entries
        .iter()
        .rev()
        .find(|(n, _, _)| n == "slow_safe")
        .unwrap();
    assert_eq!(first_safe.0, "slow_safe");
    assert!(
        first_safe.2 <= barrier.1,
        "first safe before barrier: {entries:?}"
    );
    assert!(
        barrier.2 <= second_safe.1,
        "second safe after barrier: {entries:?}"
    );
    assert!(
        !overlaps((first_safe.1, first_safe.2), (second_safe.1, second_safe.2)),
        "safes must not share a window across barrier: {entries:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn batch_mcp_never_parallel_even_if_trait_lies() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let epoch = Instant::now();
    let tools = ToolSet::from_iter(vec![
        Arc::new(SlowTool {
            name: "slow_safe",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 80,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
        Arc::new(SlowTool {
            name: "mcp_fake_x",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel, // lie
            sleep_ms: 80,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
    ]);
    let rounds = multi_tool_rounds(vec![
        ("slow_safe", r#"{"n":1}"#),
        ("mcp_fake_x", r#"{}"#),
        ("slow_safe", r#"{"n":2}"#),
    ]);
    let mut agent = make_agent_with_rounds(rounds, tools);
    agent.set_batch_mode(XyBatchMode::BarrierParallel);
    let mut stream = run_agent(&mut agent, "go").await;
    while stream.next().await.is_some() {}
    let entries = log.lock().unwrap().clone();
    assert_eq!(entries.len(), 3, "{entries:?}");
    let mcp = entries.iter().find(|(n, _, _)| n == "mcp_fake_x").unwrap();
    for (n, s, e) in &entries {
        if n == "mcp_fake_x" {
            continue;
        }
        assert!(
            !overlaps((*s, *e), (mcp.1, mcp.2)),
            "mcp must not overlap with {n}: {entries:?}"
        );
    }
}

#[tokio::test]
#[serial(obs_global)]

async fn batch_history_source_order_despite_completion_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let epoch = Instant::now();
    // First call sleeps longer so second finishes first under BarrierParallel.
    let tools = ToolSet::from_iter(vec![
        Arc::new(SlowTool {
            name: "slow_a",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 120,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
        Arc::new(SlowTool {
            name: "slow_b",
            mode: crate::protocol::ports::XyToolExecutionMode::Parallel,
            sleep_ms: 30,
            log: log.clone(),
            epoch,
        }) as Arc<dyn crate::protocol::ports::XyTool>,
    ]);
    let rounds = multi_tool_rounds(vec![("slow_a", r#"{}"#), ("slow_b", r#"{}"#)]);
    let mut agent = make_agent_with_rounds(rounds, tools);
    agent.set_batch_mode(XyBatchMode::BarrierParallel);
    let mut stream = run_agent(&mut agent, "go").await;
    let mut history = Vec::new();
    let mut end_order = Vec::new();
    while let Some(ev) = stream.next().await {
        match ev {
            XyEvent::ToolExecutionEnd { id, .. } => end_order.push(id),
            XyEvent::AgentEnd { messages } => history = messages,
            _ => {}
        }
    }
    // End MAY be completion order (b before a).
    assert!(end_order.contains(&"call-0".to_string()) && end_order.contains(&"call-1".to_string()));
    let tool_results: Vec<_> = history
        .iter()
        .filter_map(|m| match m {
            AgentMessage::Llm(crate::protocol::message::LlmMessage::ToolResultMessage {
                tool_use_id,
                ..
            }) => Some(tool_use_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        tool_results,
        vec!["call-0".to_string(), "call-1".to_string()],
        "history toolResults must be source order; ends were {end_order:?}"
    );
}

// ── Session-bound single-flight / RunPolicy regressions ───────────────

#[tokio::test]
#[serial(obs_global)]

async fn reject_second_root_while_first_live() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowMock {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                for i in 0..40u32 {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("c{i}")));
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "reject-busy");

    let mut first = agent.submit_root("one", RunPolicy::Reject).await;
    // Wait until first model call is in flight.
    let mut saw = false;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            saw = true;
            break;
        }
    }
    assert!(saw, "first stream must emit text");

    let mut second = agent.submit_root("two", RunPolicy::Reject).await;
    let mut busy = false;
    let mut second_text = false;
    while let Some(ev) = second.next().await {
        match ev {
            XyEvent::Error(err) if err.kind == "Busy" => busy = true,
            XyEvent::TextDelta(_) => second_text = true,
            _ => {}
        }
    }
    assert!(busy, "second root must be Busy");
    assert!(!second_text, "rejected root must not stream model text");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only one provider stream");

    // Drain first to completion.
    while first.next().await.is_some() {}
    assert!(
        !agent.has_active_turn(),
        "actor must be idle after first ends"
    );

    // Third submit after idle succeeds.
    let mut third = agent.submit_root("three", RunPolicy::Reject).await;
    let mut third_ok = false;
    while let Some(ev) = third.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            third_ok = true;
        }
    }
    assert!(third_ok);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
#[serial(obs_global)]

async fn abort_and_replace_starts_after_cancel() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowMock {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                if n == 0 {
                    for i in 0..80u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("old{i}")));
                    }
                } else {
                    yield Ok(crate::protocol::model::XyChunk::TextDelta("replaced".into()));
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "replace-sess");

    let mut first = agent.submit_root("old", RunPolicy::Reject).await;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            break;
        }
    }

    let mut second = agent.submit_root("new", RunPolicy::AbortAndReplace).await;

    let mut first_aborted = false;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::Error(err) if err.is_aborted()) {
            first_aborted = true;
        }
    }
    assert!(first_aborted, "first root must abort under AbortAndReplace");

    let mut texts = Vec::new();
    while let Some(ev) = second.next().await {
        if let XyEvent::TextDelta(t) = ev {
            texts.push(t);
        }
    }
    assert_eq!(texts, vec!["replaced".to_string()]);
    assert!(calls.load(Ordering::SeqCst) >= 2);
}

#[tokio::test]
#[serial(obs_global)]

async fn queue_after_run_fifo_and_drop_revokes() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingMock {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for CountingMock {
        fn name(&self) -> &str {
            "counting-mock"
        }
        async fn generate_stream(
            &self,
            messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let label = if n == 0 { "first" } else { "queued" };
            // Touch messages so multi-turn history is exercised on second call.
            let _ = messages.len();
            Ok(Box::pin(async_stream::stream! {
                if n == 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                }
                yield Ok(crate::protocol::model::XyChunk::TextDelta(label.into()));
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(CountingMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "queue-sess");

    let mut first = agent.submit_root("1", RunPolicy::Reject).await;
    // Start first turn.
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TurnStart { .. } | XyEvent::TextDelta(_)) {
            break;
        }
    }

    // Enqueue then immediately drop — must not run.
    {
        let doomed = agent.submit_root("doomed", RunPolicy::QueueAfterRun).await;
        drop(doomed);
    }

    let mut queued = agent.submit_root("2", RunPolicy::QueueAfterRun).await;

    // Finish first.
    while first.next().await.is_some() {}

    let mut texts = Vec::new();
    while let Some(ev) = queued.next().await {
        if let XyEvent::TextDelta(t) = ev {
            texts.push(t);
        }
    }
    assert_eq!(texts, vec!["queued".to_string()]);
    // first + queued only (doomed revoked).
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

/// Esc/abort cancels the active root but MUST keep an explicit QueueAfterRun pending.
#[tokio::test]
#[serial(obs_global)]

async fn abort_keeps_queued_root_after_active_cancels() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowThenFast {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowThenFast {
        fn name(&self) -> &str {
            "slow-then-fast"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                if n == 0 {
                    for i in 0..80u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("live{i}")));
                    }
                } else {
                    yield Ok(crate::protocol::model::XyChunk::TextDelta(format!(
                        "call-{n}"
                    )));
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowThenFast {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "abort-queue-sess");

    let mut first = agent.submit_root("live", RunPolicy::Reject).await;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            break;
        }
    }

    let mut queued = agent.submit_root("next", RunPolicy::QueueAfterRun).await;
    agent.abort();

    let mut first_aborted = false;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::Error(err) if err.is_aborted()) {
            first_aborted = true;
        }
    }
    assert!(first_aborted, "active root must abort");

    let mut texts = Vec::new();
    while let Some(ev) = queued.next().await {
        if let XyEvent::TextDelta(t) = ev {
            texts.push(t);
        }
    }
    assert!(
        texts.iter().any(|t| t == "call-1"),
        "queued root must run after abort: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.starts_with("live")),
        "queued stream must not carry aborted live text: {texts:?}"
    );
    assert!(
        calls.load(Ordering::SeqCst) >= 2,
        "active + queued provider calls"
    );
}

/// AbortAndReplace: stale first-stream cleanup must not clear replacement active_turn /
/// event_tx (steer QueueUpdate still reaches the new stream).
#[tokio::test]
#[serial(obs_global)]

async fn abort_and_replace_keeps_new_run_event_tx_and_active_turn() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowMock {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                if n == 0 {
                    for i in 0..80u32 {
                        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("old{i}")));
                    }
                } else {
                    yield Ok(crate::protocol::model::XyChunk::TextDelta("replaced".into()));
                    // Yield to the event merge select so QueueUpdate can surface.
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "replace-event-tx");

    let mut first = agent.submit_root("old", RunPolicy::Reject).await;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            break;
        }
    }

    let mut second = agent.submit_root("new", RunPolicy::AbortAndReplace).await;

    let mut first_aborted = false;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::Error(err) if err.is_aborted()) {
            first_aborted = true;
        }
    }
    assert!(first_aborted, "first root must abort under AbortAndReplace");

    let mut texts = Vec::new();
    let mut saw_queue = false;
    let mut steered = false;
    while let Some(ev) = second.next().await {
        match ev {
            XyEvent::TextDelta(t) => {
                if !steered {
                    assert!(
                        agent.has_active_turn(),
                        "replacement must own active_turn after stale first cleanup"
                    );
                    assert!(
                        agent.inflight_turn_binding().is_some(),
                        "replacement must expose inflight binding once live"
                    );
                    agent.steer("nudge-during-replace");
                    steered = true;
                }
                texts.push(t);
            }
            XyEvent::QueueUpdate { steer_count, .. } if steer_count >= 1 => {
                saw_queue = true;
            }
            _ => {}
        }
    }
    assert!(
        texts.iter().any(|t| t == "replaced"),
        "replacement must emit text: {texts:?}"
    );
    assert!(steered, "must have steered during replacement");
    assert!(
        saw_queue,
        "replacement event_tx must still receive QueueUpdate after stale cleanup"
    );
    assert!(calls.load(Ordering::SeqCst) >= 2);
}

/// QueueAfterRun second root MUST see first root's persisted user/assistant history.
#[tokio::test]
#[serial(obs_global)]

async fn queue_after_run_second_reads_persisted_history() {
    use crate::protocol::lifecycle::XyEvent;
    use crate::protocol::message::AgentPart;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct HistoryMock {
        calls: Arc<AtomicUsize>,
        seen: Arc<std::sync::Mutex<Vec<Vec<LlmMessage>>>>,
    }

    #[async_trait::async_trait]
    impl XyModel for HistoryMock {
        fn name(&self) -> &str {
            "history-mock"
        }
        async fn generate_stream(
            &self,
            messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            self.seen.lock().unwrap().push(messages);
            let label = if n == 0 {
                "first-reply"
            } else {
                "queued-reply"
            };
            Ok(Box::pin(async_stream::stream! {
                if n == 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                }
                yield Ok(crate::protocol::model::XyChunk::TextDelta(label.into()));
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let calls_b = calls.clone();
    let seen_b = seen.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(HistoryMock {
            calls: calls_b.clone(),
            seen: seen_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "queue-history");

    let mut first = agent.submit_root("turn-one", RunPolicy::Reject).await;
    while let Some(ev) = first.next().await {
        if matches!(ev, XyEvent::TextDelta(_)) {
            break;
        }
    }

    let mut queued = agent
        .submit_root("turn-two", RunPolicy::QueueAfterRun)
        .await;
    while first.next().await.is_some() {}

    let mut texts = Vec::new();
    while let Some(ev) = queued.next().await {
        if let XyEvent::TextDelta(t) = ev {
            texts.push(t);
        }
    }
    assert_eq!(texts, vec!["queued-reply".to_string()]);
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    let rounds = seen.lock().unwrap();
    assert_eq!(rounds.len(), 2);
    let second_input = &rounds[1];
    let user_texts: Vec<String> = second_input
        .iter()
        .filter_map(|m| match m {
            LlmMessage::UserMessage { content, .. } => content.iter().find_map(|p| match p {
                AgentPart::Text { text: t } => Some(t.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert!(
        user_texts.iter().any(|t| t == "turn-one"),
        "queued root must see first persisted user: {user_texts:?}"
    );
    assert!(
        user_texts.iter().any(|t| t == "turn-two"),
        "queued root must include its own prompt: {user_texts:?}"
    );
    let has_assistant = second_input
        .iter()
        .any(|m| matches!(m, LlmMessage::AssistantMessage { .. }));
    assert!(
        has_assistant,
        "queued root must see first assistant in history: {second_input:?}"
    );
}

#[tokio::test]
#[serial(obs_global)]

async fn bind_session_rejects_while_busy() {
    use crate::agent::runtime::RuntimeControlError;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct SlowMock {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<crate::protocol::message::LlmMessage>,
            _tools: &[crate::protocol::model::XyToolSchema],
            _stream: bool,
            _options: crate::protocol::ports::XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::pin(async_stream::stream! {
                for i in 0..40u32 {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    yield Ok(crate::protocol::model::XyChunk::TextDelta(format!("c{i}")));
                }
                yield Ok(crate::protocol::model::XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_b = calls.clone();
    let reg = mock_model_registry();
    let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
    let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
    let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
    let builder: crate::protocol::ports::XyModelBuilder = Arc::new(move |_| {
        Ok(Arc::new(SlowMock {
            calls: calls_b.clone(),
        }) as Arc<dyn XyModel>)
    });
    let session = select_mock(AgentCapabilities::new(
        reg,
        ToolSet::empty(),
        store,
        sink,
        None,
        Vec::new(),
        Vec::new(),
        ".".into(),
        None,
        builder,
        crate::infra::permission::allow_all_permission(),
        crate::agent::capabilities::QueueMode::default(),
        crate::agent::capabilities::QueueMode::default(),
        None,
    ));
    let mut agent = AgentRuntime::new(session);
    bind_session_or_panic(&mut agent, "bind-a");
    // Idle rebind is allowed.
    assert!(agent.bind_session("bind-b").is_ok());

    let mut stream = agent.submit_root("hi", RunPolicy::Reject).await;
    let mut saw = false;
    while let Some(ev) = stream.next().await {
        if matches!(ev, crate::protocol::lifecycle::XyEvent::TextDelta(_)) {
            saw = true;
            break;
        }
    }
    assert!(saw, "must be mid-run before busy bind check");
    assert!(
        matches!(
            agent.bind_session("bind-c"),
            Err(RuntimeControlError::SessionBusy)
        ),
        "rebind while live must be SessionBusy"
    );
    assert_eq!(agent.session_id(), Some("bind-b"));

    while stream.next().await.is_some() {}
    assert!(agent.bind_session("bind-c").is_ok());
    assert_eq!(agent.session_id(), Some("bind-c"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[serial(obs_global)]

async fn submit_without_bind_returns_no_session_error() {
    use crate::protocol::lifecycle::XyEvent;
    use futures::StreamExt;

    let rounds = vec![vec![crate::protocol::model::XyChunk::Done {
        finish_reason: crate::protocol::message::XyStopReason::Stop,
        usage: None,
    }]];
    let mut agent = make_agent_with_rounds(rounds, ToolSet::empty());
    let mut stream = agent.submit_root("x", RunPolicy::Reject).await;
    let mut saw = false;
    while let Some(ev) = stream.next().await {
        if let XyEvent::Error(err) = ev {
            assert_eq!(err.kind, "NoSession");
            saw = true;
        }
    }
    assert!(saw);
}
