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
use crate::agent::session::{AgentSession, ModelRegistry};
use crate::agent::tools::ToolRegistry;
use crate::core::types::{ModelMeta, ThinkingLevel};
use crate::infra::session::SessionManager;
use crate::protocol::{Command, Event};

// ── State ─────────────────────────────────────────────────────────

/// Holds the components needed to construct AgentSession for each prompt.
/// After a prompt, the session auto-persists to disk (c75); we reconstruct
/// it from these components (plus the on-disk session file) next time.
struct RpcState {
    model_registry: ModelRegistry,
    current_model_id: Option<String>,
    thinking_level: ThinkingLevel,
    session_id: String,
    cwd: String,
    system_prompt: Option<String>,
    max_iterations: u32,
    compaction_threshold: f64,
    compaction_settings: CompactionSettings,
    /// Cancellation token for the active prompt loop.
    active_cancel: Option<CancellationToken>,
}

impl RpcState {
    fn from_session(s: &AgentSession) -> Self {
        Self {
            model_registry: s.registry_clone(),
            current_model_id: s.current_model().map(|m| m.id.clone()),
            thinking_level: s.thinking_level(),
            session_id: s.session_id().unwrap_or("rpc").to_string(),
            cwd: s.cwd().to_string(),
            system_prompt: s.system_prompt().map(|s| s.to_string()),
            max_iterations: s.max_iterations(),
            compaction_threshold: s.compaction_threshold(),
            compaction_settings: s.compaction_settings().clone(),
            active_cancel: None,
        }
    }

    fn build_session(&self) -> Result<AgentSession, String> {
        let model_registry = self.model_registry.clone();
        let tool_registry = ToolRegistry::from_tools(crate::infra::tools::default_tools());
        let session_dir = SessionManager::default_dir();
        std::fs::create_dir_all(&session_dir).map_err(|e| format!("session dir: {e}"))?;
        let session_mgr = SessionManager::new(session_dir);

        let mut session = AgentSession::new(
            model_registry,
            tool_registry,
            session_mgr,
            self.system_prompt.clone(),
            self.max_iterations,
            self.compaction_threshold,
            self.cwd.clone(),
            Some(self.compaction_settings.clone()),
        );
        session.set_thinking_level(self.thinking_level);
        if let Some(ref mid) = self.current_model_id {
            let _ = session.select_model(mid);
        }
        session.set_session(self.session_id.clone());
        Ok(session)
    }
}

// ── Main loop ─────────────────────────────────────────────────────

pub async fn run(initial_session: AgentSession) -> Result<(), String> {
    let state = Arc::new(Mutex::new(RpcState::from_session(&initial_session)));

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
            let session = s.build_session();
            match session {
                Ok(sess) => {
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
            let mut session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let result = session.execute_bash(&command, exclude_from_context).await;
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
            let session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match session.maybe_auto_compact().await {
                Ok(did) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"compacted": did})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::GetSessionStats { .. } => {
            let s = state.lock().await;
            let session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match session.get_session_stats().await {
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
            let session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let path = output_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("export.html"));
            match session.export_to_html(&path).await {
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
            let session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            match session.fork_session(&entry_id).await {
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
            let session = match s.build_session() {
                Ok(s) => s,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let cmds = session.get_commands();
            let payload: Vec<Value> = cmds
                .iter()
                .map(|c| serde_json::json!({"name": c.name, "description": c.description}))
                .collect();
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"commands": payload})),
            });
        }
        Command::Quit { .. } => {} // handled in loop
    }
}

async fn run_prompt(state: &Arc<Mutex<RpcState>>, _id: &Option<String>, message: &str) {
    let (session, cancel_token) = {
        let mut s = state.lock().await;
        let session = match s.build_session() {
            Ok(s) => s,
            Err(e) => {
                emit(&Event::Error {
                    id: _id.clone(),
                    message: e,
                });
                return;
            }
        };
        let cancel = CancellationToken::new();
        s.active_cancel = Some(cancel.clone());
        (session, cancel)
    };

    let session_id = session.session_id().unwrap_or("prompt").to_string();
    let mut agent = Agent::new(session);
    let mut stream = agent.run(message, &session_id).await;

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
