//! RPC mode — stdio transport over the [`protocol`](crate::protocol) vocabulary.
//!
//! This module is a *transport*: it reads [`Command`] lines from stdin and
//! writes [`Event`] lines to stdout. The command/event types live in
//! [`protocol`](crate::protocol) (the SSOT); a future WebSocket/REST transport
//! and a server will speak the same types without touching this file.
//!
//! Protocol:
//! - stdin: one JSON object per line, [`Command`].
//! - stdout: one JSON object per line, [`Event`] (response or streaming event).
//! - stderr: diagnostics only (never protocol).
//! - Each command carries an optional `id` echoed in the response.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;

use serde_json::Value;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::agent::compaction::CompactionSettings;
use crate::agent::facade::{Agent, AgentEvent};
use crate::agent::model::registry::ModelRegistry;
use crate::agent::tools::ToolRegistry;
use crate::core::ports::{EventSink, SessionStore};
use crate::core::types::{ModelMeta, ThinkingLevel};
use crate::infra::event::EventBus;
use crate::infra::sandbox::SandboxEngine;
use crate::infra::session::SessionManager;
use crate::protocol::{Command, Event};

// ── State ─────────────────────────────────────────────────────────

/// Holds the components needed to construct an Agent for each command.
/// After a prompt, the session auto-persists to disk; we reconstruct
/// a new Agent from these components next time.
struct RpcState {
    model_registry: ModelRegistry,
    session_mgr: SessionManager,
    current_model_id: Option<String>,
    thinking_level: ThinkingLevel,
    session_id: String,
    cwd: String,
    system_prompt: Option<String>,
    max_iterations: u32,
    compaction_threshold: f64,
    compaction_settings: CompactionSettings,
    sandbox_engine: Option<Arc<dyn SandboxEngine>>,
    /// Cancellation token for the active prompt loop.
    active_cancel: Option<CancellationToken>,
}

impl RpcState {
    fn build_agent(&self) -> Result<Agent, String> {
        let tool_registry = ToolRegistry::from_tools(crate::infra::tools::default_tools());
        let store: Arc<dyn SessionStore> = Arc::new(self.session_mgr.clone());
        let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());

        let mut agent = Agent::with_ports(
            self.model_registry.clone(),
            tool_registry,
            store,
            sink,
            self.session_mgr.clone(),
            self.system_prompt.clone(),
            self.max_iterations,
            self.compaction_threshold,
            self.cwd.clone(),
            Some(self.compaction_settings.clone()),
        );
        if let Some(ref engine) = self.sandbox_engine {
            agent.session_mut().set_sandbox_engine(Some(engine.clone()));
        }
        agent.session_mut().set_thinking_level(self.thinking_level);
        if let Some(ref mid) = self.current_model_id {
            let _ = agent.session_mut().select_model(mid);
        }
        agent.session_mut().set_session(self.session_id.clone());
        Ok(agent)
    }
}

// ── Main loop ─────────────────────────────────────────────────────

pub async fn run(
    model_registry: ModelRegistry,
    session_mgr: SessionManager,
    system_prompt: Option<String>,
    max_iterations: u32,
    compaction_threshold: f64,
    cwd: String,
    compaction_settings: Option<CompactionSettings>,
    session_id: Option<String>,
    sandbox_engine: Option<Arc<dyn SandboxEngine>>,
) -> Result<(), String> {
    let state = Arc::new(Mutex::new(RpcState {
        model_registry,
        session_mgr,
        current_model_id: None,
        thinking_level: ThinkingLevel::Medium,
        session_id: session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        cwd,
        system_prompt,
        max_iterations,
        compaction_threshold,
        compaction_settings: compaction_settings.unwrap_or_default(),
        sandbox_engine,
        active_cancel: None,
    }));

    let stdin = io::stdin();
    let reader = stdin.lock();

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                emit(&Event::Error {
                    id: None,
                    message: format!("stdin read error: {e}"),
                });
                break;
            }
        };
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let cmd: Command = match serde_json::from_str(&trimmed) {
            Ok(c) => c,
            Err(e) => {
                emit(&Event::Error {
                    id: None,
                    message: format!("parse error: {e}"),
                });
                continue;
            }
        };

        if matches!(&cmd, Command::Quit { .. }) {
            break;
        }
        dispatch(&state, cmd).await;
    }

    // Ensure any active loop is cancelled on exit.
    let mut s = state.lock().await;
    if let Some(cancel) = s.active_cancel.take() {
        cancel.cancel();
    }

    Ok(())
}

async fn dispatch(state: &Arc<Mutex<RpcState>>, cmd: Command) {
    let id = match &cmd {
        Command::Prompt { id, .. }
        | Command::Abort { id, .. }
        | Command::GetState { id, .. }
        | Command::SetModel { id, .. }
        | Command::CycleModel { id, .. }
        | Command::GetAvailableModels { id, .. }
        | Command::SetThinkingLevel { id, .. }
        | Command::Bash { id, .. }
        | Command::Compact { id, .. }
        | Command::GetSessionStats { id, .. }
        | Command::ExportHtml { id, .. }
        | Command::SwitchSession { id, .. }
        | Command::Fork { id, .. }
        | Command::GetMessages { id, .. }
        | Command::GetCommands { id, .. }
        | Command::Subscribe { id, .. }
        | Command::ApproveTool { id, .. }
        | Command::AnswerQuestion { id, .. }
        | Command::Quit { id, .. } => id.clone(),
    };

    match cmd {
        Command::Prompt { message, .. } => {
            run_prompt(state, &id, &message).await;
        }
        Command::Abort { .. } => {
            let mut s = state.lock().await;
            if let Some(cancel) = s.active_cancel.take() {
                cancel.cancel();
                emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"aborted": true})),
                });
            } else {
                emit(&Event::Response {
                    id,
                    payload: Some(
                        serde_json::json!({"aborted": false, "reason": "no active loop"}),
                    ),
                });
            }
        }
        Command::GetState { .. } => {
            let s = state.lock().await;
            let agent = s.build_agent();
            match agent {
                Ok(agent) => {
                    let sess = agent.session();
                    let model = sess
                        .current_model()
                        .map(|m| serde_json::json!({"id": m.id, "display_name": m.display_name}));
                    emit(&Event::Response {
                        id,
                        payload: Some(serde_json::json!({
                            "session_id": s.session_id,
                            "model": model,
                            "thinking_level": s.thinking_level.as_str(),
                        })),
                    });
                }
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::SetModel {
            provider, model_id, ..
        } => {
            let mut s = state.lock().await;
            let found = s
                .model_registry
                .list()
                .iter()
                .find(|m| m.config.model == model_id || m.id == model_id)
                .map(|m| m.id.clone());
            match found {
                Some(mid) => {
                    s.current_model_id = Some(mid.clone());
                    emit(&Event::Response {
                        id,
                        payload: Some(serde_json::json!({"model": mid})),
                    });
                }
                None => emit(&Event::Error {
                    id,
                    message: format!("model not found: {provider}/{model_id}"),
                }),
            }
        }
        Command::CycleModel { .. } => {
            let mut s = state.lock().await;
            let list: Vec<ModelMeta> = s.model_registry.list().to_vec();
            if list.is_empty() {
                emit(&Event::Error {
                    id,
                    message: "no models available".into(),
                });
                return;
            }
            let current_idx = s
                .current_model_id
                .as_ref()
                .and_then(|cur| list.iter().position(|m| m.id == *cur))
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % list.len();
            s.current_model_id = Some(list[next_idx].id.clone());
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"model": list[next_idx].id})),
            });
        }
        Command::GetAvailableModels { .. } => {
            let s = state.lock().await;
            let models: Vec<Value> = s.model_registry.list().iter().map(|m| {
                serde_json::json!({"id": m.id, "display_name": m.display_name, "thinking": m.thinking, "context_window": m.context_window})
            }).collect();
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"models": models})),
            });
        }
        Command::SetThinkingLevel { level, .. } => {
            let mut s = state.lock().await;
            let tl = match level.to_lowercase().as_str() {
                "off" => ThinkingLevel::Off,
                "minimal" => ThinkingLevel::Minimal,
                "low" => ThinkingLevel::Low,
                "medium" => ThinkingLevel::Medium,
                "high" => ThinkingLevel::High,
                _ => {
                    emit(&Event::Error {
                        id,
                        message: format!("unknown thinking level: {level}"),
                    });
                    return;
                }
            };
            s.thinking_level = tl;
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"thinking_level": level})),
            });
        }
        Command::Bash {
            command,
            exclude_from_context,
            ..
        } => {
            let s = state.lock().await;
            let mut agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let result = agent.session_mut().execute_bash(&command, exclude_from_context).await;
            match result {
                Ok(br) => emit(&Event::BashResult {
                    id: id.clone(),
                    output: br.output,
                    exit_code: br.exit_code,
                    cancelled: br.cancelled,
                    truncated: br.truncated,
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::Compact { .. } => {
            let s = state.lock().await;
            let mut agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match agent.session_mut().maybe_auto_compact().await {
                Ok(did) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"compacted": did})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::GetSessionStats { .. } => {
            let s = state.lock().await;
            let mut agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match agent.session_mut().get_session_stats().await {
                Ok(stats) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({
                        "session_id": stats.session_id, "user_messages": stats.user_messages,
                        "assistant_messages": stats.assistant_messages, "total_messages": stats.total_messages,
                        "thinking_level": stats.thinking_level,
                    })),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::ExportHtml { output_path, .. } => {
            let s = state.lock().await;
            let mut agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let path = output_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("export.html"));
            match agent.session_mut().export_to_html(&path).await {
                Ok(_) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"path": path.to_string_lossy()})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::SwitchSession { session_path, .. } => {
            let mut s = state.lock().await;
            s.session_id = session_path.clone();
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"session": session_path})),
            });
        }
        Command::Fork { entry_id, .. } => {
            let s = state.lock().await;
            let mut agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match agent.session_mut().fork_session(&entry_id).await {
                Ok(new_id) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"new_session": new_id})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::GetMessages { .. } => {
            let _s = state.lock().await;
            // Stub: session manager can load entries.
            emit(&Event::Error {
                id,
                message: "get_messages not yet implemented in RPC".into(),
            });
        }
        Command::GetCommands { .. } => {
            let s = state.lock().await;
            let agent = match s.build_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let cmds = agent.session().get_commands();
            let payload: Vec<Value> = cmds
                .iter()
                .map(|c| serde_json::json!({"name": c.name, "description": c.description}))
                .collect();
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"commands": payload})),
            });
        }
        Command::Subscribe { .. } => {
            emit(&Event::Error {
                id,
                message: "subscribe requires WebSocket connection (not stdio RPC)".into(),
            });
        }
        Command::ApproveTool { call_id, approved, .. } => {
            emit(&Event::Error {
                id,
                message: format!(
                    "approve_tool requires WebSocket connection (call_id={}, approved={})",
                    call_id, approved
                ),
            });
        }
        Command::AnswerQuestion { call_id, answer: _, .. } => {
            emit(&Event::Error {
                id,
                message: format!(
                    "answer_question requires WebSocket connection (call_id={})",
                    call_id
                ),
            });
        }
        Command::Quit { .. } => {} // handled in loop
    }
}

async fn run_prompt(state: &Arc<Mutex<RpcState>>, _id: &Option<String>, message: &str) {
    let (mut agent, cancel_token, session_id) = {
        let mut s = state.lock().await;
        let agent = match s.build_agent() {
            Ok(a) => a,
            Err(e) => {
                emit(&Event::Error {
                    id: _id.clone(),
                    message: e,
                });
                return;
            }
        };
        let session_id = agent.session().session_id().unwrap_or("prompt").to_string();
        let cancel = CancellationToken::new();
        s.active_cancel = Some(cancel.clone());
        (agent, cancel, session_id)
    };

    let mut stream = agent.run_with_id(message, &session_id).await;

    loop {
        tokio::select! {
            event = stream.next() => {
                match event {
                    Some(evt) => {
                        let emitted = match &evt {
                            AgentEvent::TextDelta(t) => { emit(&Event::TextDelta { text: t.clone() }); true }
                            AgentEvent::ThinkingDelta(_) => false,
                            AgentEvent::ToolExecutionStart { id, name, .. } => { emit(&Event::ToolStart { id: id.clone(), name: name.clone() }); true }
                            AgentEvent::ToolExecutionEnd { id, name, result, .. } => { emit(&Event::ToolEnd { id: id.clone(), name: name.clone(), result: result.clone() }); true }
                            AgentEvent::ModelSelect { provider, model_id } => { emit(&Event::ModelSelect { provider: provider.clone(), model_id: model_id.clone() }); true }
                            AgentEvent::CompactionStart { reason } => { emit(&Event::CompactionStart { reason: reason.clone() }); true }
                            AgentEvent::AgentEnd { .. } => { emit(&Event::AgentEnd); false }
                            AgentEvent::Error(msg) => { emit(&Event::Error { id: None, message: msg.clone() }); true }
                            _ => false,
                        };
                        if !emitted && matches!(evt, AgentEvent::AgentEnd { .. }) {
                            break;
                        }
                        if matches!(evt, AgentEvent::AgentEnd { .. }) {
                            break;
                        }
                    }
                    None => break,
                }
            }
            _ = cancel_token.cancelled() => {
                agent.abort();
                emit(&Event::AgentEnd);
                break;
            }
        }
    }

    // Clear active cancel.
    let mut s = state.lock().await;
    s.active_cancel = None;
}

// ── I/O helpers ─────────────────────────────────────────────────

fn emit(event: &Event) {
    let line = serde_json::to_string(event).expect("Event serialization never fails");
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{line}");
    let _ = handle.flush();
}
