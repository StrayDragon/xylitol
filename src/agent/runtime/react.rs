//! Agent execution loop — core ReAct loop with full event stream, hooks, and tool execution modes.
//!
//! NOTE: 本文件聚焦 ReAct 算法主体. 天花板: ~500 行 (含 inline tests), 因 run_react_loop
//! 的 async_stream 宏块是原子逻辑单元, 跨函数 yield 不可行. 升级: 当工具执行/流处理逻辑
//! 显著膨胀时, 考虑引入 sub-turn state machine 替代单宏块.

//!
//! Key features:
//! - ReAct loop with turn-based execution
//! - `AgentHooks`: before_tool_call, after_tool_call, transform_context
//! - Steering/follow-up message queue callbacks
//! - Per-tool execution modes: sequential / parallel
//! - Auto-retry on transient errors

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::permission_router::permission_target;
use super::retry::{RetryState, is_retryable_error};
use super::{AgentHooks, XyEvent, XyEventStream};
use crate::agent::session::Agent;
use crate::agent::tools::ToolSet;
use crate::domain::error::XyError;
use crate::domain::message::{AgentMessage, AgentPart};
use crate::domain::types::{XyChunk, XyToolSchema};
use crate::runtime_protocol::{XyModel, XyToolCtx, XyToolExecutionMode};

use crate::runtime_protocol::XyPermissionVerdict;

// ── ReActAgent ───────────────────────────────────────────────────────

pub struct ReActAgent {
    pub(crate) session: Agent,
    /// Cancellation token.
    cancel: CancellationToken,
}

impl ReActAgent {
    pub fn new(session: Agent) -> Self {
        Self {
            session,
            cancel: CancellationToken::new(),
        }
    }

    /// Get a reference to the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Signal cancellation to abort the agent loop.
    pub fn abort(&self) {
        self.cancel.cancel();
    }

    pub fn session(&self) -> &Agent {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Agent {
        &mut self.session
    }

    /// Replace the tool set. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tools(&mut self, tools: ToolSet) {
        self.session.set_tools(tools);
    }

    /// Replace the hook set. Takes effect on the next [`run`](Self::run) call.
    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.session.replace_hooks(hooks);
    }

    /// Add a before-tool hook. Takes effect on the next [`run`](Self::run) call.
    pub fn add_hook(&mut self, hook: super::hooks::BeforeToolHook) {
        self.session.hooks_mut().add_before(hook);
    }

    /// Set the permission port. Takes effect on the next [`run`](Self::run) call.
    pub fn set_permission(&mut self, permission: Arc<dyn crate::runtime_protocol::XyPermission>) {
        self.session.set_permission(permission);
    }

    /// Set the tool execution mode. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tool_mode(&mut self, mode: crate::runtime_protocol::XyToolExecutionMode) {
        self.session.set_tool_mode(mode);
    }

    /// Set the system prompt. Takes effect on the next [`run`](Self::run) call.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.session.set_system_prompt(prompt);
    }

    /// Run a turn with an auto-generated session_id.
    pub async fn run(&mut self, prompt: &str) -> XyEventStream {
        self.run_with_id(prompt, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id.
    #[allow(clippy::type_complexity)]
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> XyEventStream {
        // Build permission check callback from session
        let permission_check: Option<
            std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>,
        >;
        {
            let engine = self.session.get_permission();
            permission_check = Some(std::sync::Arc::new(
                move |tool_name: &str, tool_path: &str| -> Option<String> {
                    // NOTE: tool_name → permission check dispatch is hard-coded here.
                    // Ceiling: adding a new sandbox-sensitive tool requires editing this match.
                    // Upgrade: when xylitol matures as a harness, introduce tool-declared
                    // capability categories and replace this match with a capability-driven router.
                    match tool_name {
                        "read" => match engine.check_read(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "write" | "edit" => match engine.check_write(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        "bash" => match engine.check_network(tool_path) {
                            XyPermissionVerdict::Deny { reason } => Some(reason),
                            _ => None,
                        },
                        _ => None,
                    }
                },
            ));
        }
        // Ensure session exists
        let sid = session_id.to_string();
        self.session.set_session(sid.clone());
        if let Err(e) = self.session.ensure_session(&sid, None).await {
            return XyEventStream::error(format!("session error: {e}"));
        }

        let model = match self.session.build_current_model() {
            Ok(m) => m,
            Err(e) => return XyEventStream::error(format!("model build error: {e}")),
        };

        let tools = self.session.tools().clone();
        let max_iterations = self.session.max_iterations();
        let system_prompt = self.session.system_prompt().map(|s| s.to_string());
        let hooks = self.session.hooks().clone();
        let tool_mode = self.session.tool_mode();
        let prompt = prompt.to_string();

        // Build tool schemas
        let tool_schemas: Vec<XyToolSchema> = tools
            .iter()
            .map(|t| XyToolSchema {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();

        let cancel = self.cancel.clone();

        let inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>> =
            Box::pin(run_react_loop(ReActConfig {
                model,
                tools,
                tool_schemas,
                system_prompt,
                max_iterations: max_iterations as usize,
                user_prompt: prompt,
                cancel,
                permission_check,
                hooks,
                tool_mode,
            }));

        XyEventStream {
            inner,
            done: false,
            turn_index: 0,
        }
    }
}

// ── Core ReAct loop config ─────────────────────────────────────────

/// Parameters for the ReAct agent loop.
struct ReActConfig {
    model: Arc<dyn XyModel>,
    tools: ToolSet,
    tool_schemas: Vec<XyToolSchema>,
    system_prompt: Option<String>,
    max_iterations: usize,
    user_prompt: String,
    cancel: CancellationToken,
    /// Optional permission check. Called with (tool_name, target_path_or_domain).
    /// Returns Some(reason) if the operation is denied.
    #[allow(clippy::type_complexity)]
    permission_check: Option<std::sync::Arc<dyn Fn(&str, &str) -> Option<String> + Send + Sync>>,
    /// Hooks consulted at tool-call boundaries.
    hooks: AgentHooks,
    /// Tool execution mode (currently advisory; sequential execution is the
    /// conservative default).
    tool_mode: XyToolExecutionMode,
}

// ── Core ReAct loop ─────────────────────────────────────────────────

fn run_react_loop(cfg: ReActConfig) -> impl Stream<Item = XyEvent> + Send {
    let ReActConfig {
        model,
        tools,
        tool_schemas,
        system_prompt,
        max_iterations,
        user_prompt,
        cancel,
        permission_check,
        hooks,
        tool_mode: _tool_mode,
    } = cfg;
    async_stream::stream! {
        let mut history: Vec<AgentMessage> = Vec::new();

        // Add system prompt to history (as user message — AgentMessage has no system variant)
        if let Some(ref sp) = system_prompt {
            history.push(AgentMessage::UserMessage {
                content: vec![AgentPart::Text(sp.clone())],
                timestamp: crate::domain::message::now_ms(),
            });
        }

        // Add user message
        history.push(AgentMessage::UserMessage {
            content: vec![AgentPart::Text(user_prompt.clone())],
            timestamp: crate::domain::message::now_ms(),
        });

        let retry_state = RetryState::new(3, 1000);

        for turn in 0..max_iterations {
            // Check for cancellation before each turn
            if cancel.is_cancelled() {
                yield XyEvent::Error("aborted".to_string());
                break;
            }

            yield XyEvent::TurnStart { turn_index: turn as u32 };

            // Send accumulated history to the model.
            // History includes system prompt, user messages, assistant responses,
            // and tool results from previous turns — giving the LLM full context.
            let messages = history.clone();

            // Call model with retry support
            let stream_result = call_with_retry(
                &model, messages.clone(), &tool_schemas, &retry_state,
            ).await;

            let mut chunk_stream: Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>;
            match stream_result {
                Ok(s) => chunk_stream = s,
                Err(e) => {
                    yield XyEvent::Error(e);
                    break;
                }
            }

            yield XyEvent::MessageStart {
                role: "assistant".to_string(),
                message: None,
            };

            let mut text_acc = String::new();
            let mut thinking_acc = String::new();
            let mut tool_calls: Vec<(String, String, Value)> = Vec::new();
            let mut done = false;

            while let Some(chunk_result) = chunk_stream.next().await {
                match chunk_result {
                    Ok(chunk) => match chunk {
                        XyChunk::TextDelta(text) => {
                            text_acc.push_str(&text);
                            yield XyEvent::TextDelta(text.clone());
                            yield XyEvent::MessageUpdate {
                                text: text_acc.clone(),
                                thinking: if thinking_acc.is_empty() { None } else { Some(thinking_acc.clone()) },
                                message: None,
                            };
                        }
                        XyChunk::ThinkingDelta(text) => {
                            thinking_acc.push_str(&text);
                            yield XyEvent::ThinkingDelta(text);
                        }
                        XyChunk::FunctionCall { name, args, id } => {
                            yield XyEvent::ToolExecutionStart {
                                id: id.clone(),
                                name: name.clone(),
                                args: args.clone(),
                            };
                            tool_calls.push((id, name, args));
                        }
                        XyChunk::Done { .. } => {
                            done = true;
                        }
                    },
                    Err(e) => {
                        yield XyEvent::Error(format!("stream error: {e}"));
                        break;
                    }
                }
            }

            yield XyEvent::MessageEnd {
                role: "assistant".to_string(),
                message: None,
            };

            // Build assistant message
            let mut assistant_parts = Vec::new();
            if !thinking_acc.is_empty() {
                assistant_parts.push(AgentPart::Thinking { text: thinking_acc, redacted: false, signature: None });
            }
            if !text_acc.is_empty() {
                assistant_parts.push(AgentPart::Text(text_acc));
            }
            for (id, name, args) in &tool_calls {
                assistant_parts.push(AgentPart::ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    arguments: args.clone(),
                });
            }
            if !assistant_parts.is_empty() {
                history.push(AgentMessage::AssistantMessage {
                    content: assistant_parts,
                    stop_reason: None,
                    usage: None,
                    api: String::new(),
                    provider: String::new(),
                    model: String::new(),
                    response_id: None,
                    error_message: None,
                    timestamp: crate::domain::message::now_ms(),
                    diagnostics: Vec::new(),
                });
            }

            // If no tool calls, done
            if tool_calls.is_empty() {
                yield XyEvent::TurnEnd { turn_index: turn as u32 };
                break;
            }

            // Execute tool calls
            for (id, name, args) in &tool_calls {
                let tool = tools.get(name);
                let ctx = XyToolCtx::with_cancel(id, cancel.clone());

                // ── Before-tool hooks ─────────────────────────────
                let mut denied_reason: Option<String> = None;
                if !hooks.before_tool_call.is_empty() {
                    for hook in &hooks.before_tool_call {
                        if let Some(reason) = hook(name, id, args) {
                            denied_reason = Some(reason);
                            break;
                        }
                    }
                }

                // ── Permission check ──────────────────────────────
                if denied_reason.is_none()
                    && let Some(ref check) = permission_check
                {
                    let target = permission_target(name, args);
                    if let Some(reason) = check(name, &target) {
                        denied_reason = Some(format!("permission denied: {reason}"));
                    }
                }

                if let Some(reason) = denied_reason {
                    let err = format!("Tool '{name}' blocked: {reason}");
                    yield XyEvent::ToolExecutionEnd {
                        id: id.clone(),
                        name: name.clone(),
                        result: err.clone(),
                        is_error: true,
                    };
                    history.push(AgentMessage::ToolResultMessage {
                        tool_use_id: id.clone(),
                        tool_name: name.clone(),
                        content: vec![AgentPart::Text(err.clone())],
                        details: None,
                        is_error: true,
                        timestamp: crate::domain::message::now_ms(),
                    });
                    continue;
                }

                let mut result = match tool {
                    Some(t) => match t.execute(&ctx, args.clone()).await {
                        Ok(output) => (serde_json::Value::String(output), false),
                        Err(e) => {
                            let err = format!("Tool '{name}' error: {e}");
                            yield XyEvent::Error(err.clone());
                            (serde_json::Value::String(err), true)
                        }
                    },
                    None => {
                        let err = format!("Unknown tool: {name}");
                        yield XyEvent::Error(err.clone());
                        (serde_json::Value::String(err), true)
                    }
                };

                // ── After-tool hooks ──────────────────────────────
                if !hooks.after_tool_call.is_empty() {
                    for hook in &hooks.after_tool_call {
                        if let Some((new_value, new_is_error)) = hook(name, id, result.0.clone(), result.1) {
                            result = (new_value, new_is_error);
                        }
                    }
                }

                let result_text = match result.0 {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };

                yield XyEvent::ToolExecutionEnd {
                    id: id.clone(),
                    name: name.clone(),
                    result: result_text.clone(),
                    is_error: result.1,
                };

                history.push(AgentMessage::ToolResultMessage {
                    tool_use_id: id.clone(),
                    tool_name: name.clone(),
                    content: vec![AgentPart::Text(result_text.clone())],
                    details: None,
                    is_error: result.1,
                    timestamp: crate::domain::message::now_ms(),
                });
            }

            yield XyEvent::TurnEnd { turn_index: turn as u32 };

            if done {
                break;
            }
        }

        yield XyEvent::AgentEnd { messages: history };
    }
}

/// Helper: call model with retry for transient errors.
async fn call_with_retry(
    model: &Arc<dyn XyModel>,
    messages: Vec<AgentMessage>,
    tool_schemas: &[XyToolSchema],
    retry_state: &RetryState,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, String> {
    loop {
        match model
            .generate_stream(messages.clone(), tool_schemas, true)
            .await
        {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                let err_msg = format!("model error: {e}");
                if is_retryable_error(&err_msg) && retry_state.can_retry() {
                    let delay = retry_state.next_delay();
                    retry_state.backoff(delay).await;
                    continue;
                }
                return Err(err_msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::agent::model::registry::ModelRegistry;
    use crate::domain::model::XyModelConfig;
    use crate::domain::types::XyModelMeta;
    use crate::infra::session::SessionManager;
    use crate::runtime_protocol::{XyEventSink, XyModel, XySessionStore, XyStream};

    /// Model builder for tests — the real factory (tests register `Fake`/`OpenAi`
    /// model configs and rely on `build_provider` constructing the provider struct;
    /// no real network calls are made in unit assertions).
    fn fake_model_builder()
    -> Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync> {
        Arc::new(crate::infra::provider::factory::build_provider)
    }

    #[tokio::test]
    async fn test_agent_session_builds_model() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: crate::domain::model::XyModelConfig {
                kind: crate::domain::model::XyModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
                api: None,
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
        });

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = Agent::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            50,
            0.8,
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::permission::allow_all_permission(),
            Some(std::sync::Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            )),
            Some(std::sync::Arc::new(crate::infra::export::StdExportIo::new())),
        );

        assert!(session.current_model().is_some());
        assert_eq!(session.current_model().unwrap().id, "mock");
    }

    #[tokio::test]
    async fn test_agent_loop_emits_events() {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: crate::domain::model::XyModelConfig {
                kind: crate::domain::model::XyModelKind::OpenAi,
                api_key: "sk-test".into(),
                model: "mock-model".into(),
                base_url: None,
                api: None,
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
        });

        let session_mgr = SessionManager::new(SessionManager::default_dir());
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr.clone());
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = Agent::new(
            reg,
            ToolSet::from_iter(crate::infra::tools::default_tools()),
            store,
            sink,
            Some("You are helpful.".into()),
            Vec::new(),
            Vec::new(),
            50,
            0.8,
            ".".into(),
            None,
            fake_model_builder(),
            crate::infra::permission::allow_all_permission(),
            Some(std::sync::Arc::new(
                crate::infra::bash_exec::InfraBashExecutor::new(),
            )),
            Some(std::sync::Arc::new(crate::infra::export::StdExportIo::new())),
        );

        let mut loop_runner = ReActAgent::new(session);
        let _stream = loop_runner.run_with_id("hello", "test-session").await;
    }

    // ── Mock model / tool helpers for hook and snapshot tests ───────

    struct MockModel {
        chunks: Vec<crate::domain::types::XyChunk>,
    }

    #[async_trait::async_trait]
    impl XyModel for MockModel {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate_stream(
            &self,
            _messages: Vec<AgentMessage>,
            _tools: &[crate::domain::types::XyToolSchema],
            _stream: bool,
        ) -> Result<XyStream, XyError> {
            let chunks = self.chunks.clone();
            Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
        }
    }

    struct MockTool;

    #[async_trait::async_trait]
    impl crate::runtime_protocol::XyTool for MockTool {
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
            _ctx: &crate::runtime_protocol::XyToolCtx,
            _args: serde_json::Value,
        ) -> Result<String, crate::domain::error::XyToolError> {
            Ok("executed".into())
        }
    }

    fn mock_model_registry(chunks: Vec<crate::domain::types::XyChunk>) -> ModelRegistry {
        let mut reg = ModelRegistry::new(std::sync::Arc::new(
            crate::infra::config::value::InfraSecretResolver::new(),
        ));
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: crate::domain::model::XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
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
        });
        reg
    }

    fn mock_model_builder(
        chunks: Vec<crate::domain::types::XyChunk>,
    ) -> Arc<dyn Fn(&XyModelConfig) -> Result<Arc<dyn XyModel>, String> + Send + Sync> {
        Arc::new(move |_| {
            Ok(Arc::new(MockModel {
                chunks: chunks.clone(),
            }) as Arc<dyn XyModel>)
        })
    }

    fn make_agent_with_tools(
        chunks: Vec<crate::domain::types::XyChunk>,
        tools: ToolSet,
    ) -> ReActAgent {
        let reg = mock_model_registry(chunks.clone());
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        let session = Agent::new(
            reg,
            tools,
            store,
            sink,
            None,
            Vec::new(),
            Vec::new(),
            50,
            0.8,
            ".".into(),
            None,
            mock_model_builder(chunks),
            crate::infra::permission::allow_all_permission(),
            None,
            None,
        );
        ReActAgent::new(session)
    }

    #[tokio::test]
    async fn test_before_hook_denies_tool_call() {
        use crate::agent::runtime::hooks::BeforeToolHook;
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::FunctionCall {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks,
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
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

        let mut stream = agent.run("go").await;
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
    async fn test_after_hook_modifies_tool_result() {
        use crate::agent::runtime::hooks::AfterToolHook;
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::FunctionCall {
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
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
            ]),
        );
        agent.replace_hooks(hooks);

        let mut stream = agent.run("go").await;
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
    async fn test_set_tools_takes_effect_on_next_turn() {
        use crate::domain::lifecycle::XyEvent;
        use futures::StreamExt;

        let chunks = vec![crate::domain::types::XyChunk::FunctionCall {
            id: "call-1".into(),
            name: "mock_tool".into(),
            args: serde_json::json!({"input": "x"}),
        }];
        let mut agent = make_agent_with_tools(
            chunks.clone(),
            ToolSet::from_iter(vec![
                Arc::new(MockTool) as Arc<dyn crate::runtime_protocol::XyTool>
            ]),
        );

        // First turn: mock_tool is available.
        let mut stream = agent.run("go").await;
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

        // Replace tools with an empty set between turns.
        agent.set_tools(ToolSet::empty());

        // Second turn: the loop still saw mock_tool in the original snapshot if
        // we had mutated it mid-stream, but because setters apply to the next
        // turn, this turn should report the tool as unknown.
        let mut stream = agent.run("go").await;
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
}
