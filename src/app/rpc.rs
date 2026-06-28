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
use crate::agent::facade::{Agent, XyEvent};
use crate::agent::model::registry::ModelRegistry;
use crate::app::core::composition::{BuildAgentOptions, build_agent};
use crate::domain::types::{ThinkingLevel, XyModelMeta};
use crate::infra::sandbox::XySandboxEngine;
use crate::infra::session::SessionManager;
use crate::protocol::{Command, Event};
use crate::runtime_protocol::XySessionStore;

// ── State ─────────────────────────────────────────────────────────

/// Holds the components needed to construct an Agent for each command.
/// After a prompt, the session auto-persists to disk; we reconstruct
/// a new Agent from these components next time — unless the cache is
/// still valid, in which case we reuse the previously-built [`Agent`].
struct RpcState {
    model_registry: ModelRegistry,
    session_mgr: SessionManager,
    current_model_id: Option<String>,
    thinking_level: ThinkingLevel,
    session_id: String,
    cwd: String,
    system_prompt: Option<String>,
    context_files: Vec<(String, String)>,
    append_system_prompt: Vec<String>,
    max_iterations: u32,
    compaction_threshold: f64,
    compaction_settings: CompactionSettings,
    sandbox_engine: Option<Arc<dyn XySandboxEngine>>,
    /// Cancellation token for the active prompt loop.
    active_cancel: Option<CancellationToken>,
    /// Cached Agent, reused across commands until a rebuild trigger fires.
    cached_agent: Option<Agent>,
    /// Snapshot of session_id used to build `cached_agent`.
    cache_session_id: Option<String>,
    /// Snapshot of model_id used to build `cached_agent`.
    cache_model_id: Option<String>,
    /// Snapshot of thinking_level used to build `cached_agent`.
    cache_thinking_level: Option<ThinkingLevel>,
}

impl RpcState {
    /// Build a brand-new Agent from current state (no cache).
    fn build_agent_fresh(&self) -> Result<Agent, String> {
        let mut agent = build_agent(BuildAgentOptions {
            model_registry: self.model_registry.clone(),
            system_prompt: self.system_prompt.clone(),
            context_files: self.context_files.clone(),
            append_system_prompt: self.append_system_prompt.clone(),
            max_iterations: self.max_iterations,
            compaction_threshold: self.compaction_threshold,
            cwd: self.cwd.clone(),
            compaction_settings: Some(self.compaction_settings.clone()),
            sandbox_engine: self.sandbox_engine.clone(),
        })?;
        agent.session_mut().set_thinking_level(self.thinking_level);
        if let Some(ref mid) = self.current_model_id {
            let _ = agent.session_mut().select_model(mid);
        }
        agent.session_mut().set_session(self.session_id.clone());
        Ok(agent)
    }

    /// Whether the cached Agent is still valid for the current state.
    ///
    /// Rebuild triggers (T12): `session_id`, `current_model_id`, or
    /// `thinking_level` changed since the cache was populated.
    fn cache_is_valid(&self) -> bool {
        match &self.cached_agent {
            None => false,
            Some(_) => {
                self.cache_session_id.as_deref() == Some(self.session_id.as_str())
                    && self.cache_model_id == self.current_model_id
                    && self.cache_thinking_level == Some(self.thinking_level)
            }
        }
    }

    /// Record the build parameters so the next `cache_is_valid()` reflects
    /// what was used to construct the current cache.
    fn stamp_cache(&mut self) {
        self.cache_session_id = Some(self.session_id.clone());
        self.cache_model_id = self.current_model_id.clone();
        self.cache_thinking_level = Some(self.thinking_level);
    }

    /// Ensure a valid cached Agent exists and borrow it.
    ///
    /// Use this for commands that hold the lock for their entire duration
    /// (Bash, Compact, GetState, …). For `run_prompt` use [`take_agent`]
    /// so the lock can be released during streaming.
    fn ensure_agent(&mut self) -> Result<&mut Agent, String> {
        if !self.cache_is_valid() {
            let agent = self.build_agent_fresh()?;
            self.cached_agent = Some(agent);
            self.stamp_cache();
        }
        // SAFETY: cache_is_valid just confirmed Some, or we just set Some.
        Ok(self.cached_agent.as_mut().expect("cache populated above"))
    }

    /// Take the cached Agent out of the cache (for `run_prompt`, which needs
    /// ownership so the lock can be released during streaming).
    ///
    /// Rebuilds if the cache is invalid. Pair with [`return_agent`].
    fn take_agent(&mut self) -> Result<Agent, String> {
        if !self.cache_is_valid() {
            // Build fresh; cache stays None until return_agent restores it.
            let agent = self.build_agent_fresh()?;
            self.stamp_cache();
            return Ok(agent);
        }
        self.cached_agent
            .take()
            .ok_or_else(|| "agent cache inconsistency (valid but empty)".into())
    }

    /// Restore an Agent previously taken via [`take_agent`].
    fn return_agent(&mut self, agent: Agent) {
        self.cached_agent = Some(agent);
    }
}

// ── Main loop ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn run(
    model_registry: ModelRegistry,
    session_mgr: SessionManager,
    system_prompt: Option<String>,
    context_files: Vec<(String, String)>,
    append_system_prompt: Vec<String>,
    max_iterations: u32,
    compaction_threshold: f64,
    cwd: String,
    compaction_settings: Option<CompactionSettings>,
    session_id: Option<String>,
    sandbox_engine: Option<Arc<dyn XySandboxEngine>>,
) -> Result<(), String> {
    let state = Arc::new(Mutex::new(RpcState {
        model_registry,
        session_mgr,
        current_model_id: None,
        thinking_level: ThinkingLevel::Medium,
        session_id: session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        cwd,
        system_prompt,
        context_files,
        append_system_prompt,
        max_iterations,
        compaction_threshold,
        compaction_settings: compaction_settings.unwrap_or_default(),
        sandbox_engine,
        active_cancel: None,
        cached_agent: None,
        cache_session_id: None,
        cache_model_id: None,
        cache_thinking_level: None,
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
        | Command::ExportJsonl { id, .. }
        | Command::ImportJsonl { id, .. }
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
            let mut s = state.lock().await;
            // Capture scalar state first so the agent borrow doesn't conflict.
            let session_id = s.session_id.clone();
            let thinking_level = s.thinking_level;
            match s.ensure_agent() {
                Ok(agent) => {
                    let model = agent
                        .session()
                        .current_model()
                        .map(|m| serde_json::json!({"id": m.id, "display_name": m.display_name}));
                    emit(&Event::Response {
                        id,
                        payload: Some(serde_json::json!({
                            "session_id": session_id,
                            "model": model,
                            "thinking_level": thinking_level.as_str(),
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
            let list: Vec<XyModelMeta> = s.model_registry.list().to_vec();
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
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let result = agent
                .session_mut()
                .execute_bash(&command, exclude_from_context)
                .await;
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
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
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
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
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
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
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
        Command::ExportJsonl { output_path, .. } => {
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let path = output_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("export.jsonl"));
            match agent.session_mut().export_to_jsonl(&path).await {
                Ok(_) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"path": path.to_string_lossy()})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::ImportJsonl { input_path, .. } => {
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
                Ok(a) => a,
                Err(e) => {
                    emit(&Event::Error { id, message: e });
                    return;
                }
            };
            let path = PathBuf::from(&input_path);
            match agent.session_mut().import_from_jsonl(&path).await {
                Ok(new_id) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({"new_session": new_id})),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::SwitchSession { session_path, .. } => {
            let mut s = state.lock().await;
            // Derive session id from path (file stem), validate existence.
            let new_id = std::path::Path::new(&session_path)
                .file_stem()
                .and_then(|st| st.to_str())
                .unwrap_or(&session_path)
                .to_string();
            if !s.session_mgr.exists(&new_id) {
                emit(&Event::Error {
                    id,
                    message: format!("session not found: {new_id}"),
                });
                return;
            }
            // Switching session_id invalidates the cached agent (cache_is_valid
            // compares against session_id), so the next ensure_agent rebuilds.
            s.session_id = new_id.clone();
            emit(&Event::Response {
                id,
                payload: Some(serde_json::json!({"session": new_id})),
            });
        }
        Command::Fork { entry_id, .. } => {
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
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
            let s = state.lock().await;
            match s.session_mgr.load_entries(&s.session_id).await {
                Ok(entries) => emit(&Event::Response {
                    id,
                    payload: Some(serde_json::json!({
                        "session_id": s.session_id,
                        "messages": entries,
                    })),
                }),
                Err(e) => emit(&Event::Error { id, message: e }),
            }
        }
        Command::GetCommands { .. } => {
            let mut s = state.lock().await;
            let agent = match s.ensure_agent() {
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
        Command::ApproveTool {
            call_id, approved, ..
        } => {
            emit(&Event::Error {
                id,
                message: format!(
                    "approve_tool requires WebSocket connection (call_id={}, approved={})",
                    call_id, approved
                ),
            });
        }
        Command::AnswerQuestion {
            call_id, answer: _, ..
        } => {
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
        let agent = match s.take_agent() {
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
                        if let Some(pe) = evt.to_wire_event() {
                            emit(&pe);
                        }
                        if matches!(evt, XyEvent::AgentEnd { .. }) {
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

    // Restore the cached agent and clear the active cancel token.
    let mut s = state.lock().await;
    s.return_agent(agent);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{ThinkingLevel, XyModelMeta};
    use crate::infra::config::value::InfraSecretResolver;

    fn make_state() -> RpcState {
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().keep());
        let mut reg = ModelRegistry::new(Arc::new(InfraSecretResolver::new()));
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
        RpcState {
            model_registry: reg,
            session_mgr,
            current_model_id: None,
            thinking_level: ThinkingLevel::Medium,
            session_id: "test-session".into(),
            cwd: ".".into(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            max_iterations: 50,
            compaction_threshold: 0.8,
            compaction_settings: CompactionSettings::default(),
            sandbox_engine: None,
            active_cancel: None,
            cached_agent: None,
            cache_session_id: None,
            cache_model_id: None,
            cache_thinking_level: None,
        }
    }

    #[test]
    fn cache_invalid_when_empty() {
        let state = make_state();
        assert!(!state.cache_is_valid());
    }

    #[test]
    fn cache_valid_after_ensure_agent() {
        let mut state = make_state();
        // First ensure_agent builds + stamps the cache.
        assert!(state.ensure_agent().is_ok());
        assert!(state.cache_is_valid());
    }

    #[test]
    fn cache_invalidated_by_session_id_change() {
        let mut state = make_state();
        assert!(state.ensure_agent().is_ok());
        assert!(state.cache_is_valid());
        state.session_id = "different-session".into();
        assert!(!state.cache_is_valid());
    }

    #[test]
    fn cache_invalidated_by_model_id_change() {
        let mut state = make_state();
        assert!(state.ensure_agent().is_ok());
        assert!(state.cache_is_valid());
        state.current_model_id = Some("mock".into());
        assert!(!state.cache_is_valid());
    }

    #[test]
    fn cache_invalidated_by_thinking_level_change() {
        let mut state = make_state();
        assert!(state.ensure_agent().is_ok());
        assert!(state.cache_is_valid());
        state.thinking_level = ThinkingLevel::High;
        assert!(!state.cache_is_valid());
    }

    /// T13: three sequential commands construct the Agent only once.
    #[test]
    fn three_commands_build_agent_once() {
        let mut state = make_state();
        // Simulate three commands reusing the cache.
        let a1 = state.ensure_agent().unwrap() as *const Agent;
        let a2 = state.ensure_agent().unwrap() as *const Agent;
        let a3 = state.ensure_agent().unwrap() as *const Agent;
        // Same underlying Agent object (reused, not rebuilt).
        assert_eq!(a1, a2);
        assert_eq!(a2, a3);
    }

    /// T13: take_agent + return_agent preserves the cache across a prompt.
    #[test]
    fn take_and_return_preserves_cache() {
        let mut state = make_state();
        assert!(state.ensure_agent().is_ok());
        let agent = state.take_agent().unwrap();
        // While taken, the slot is empty → not reusable until restored.
        assert!(!state.cache_is_valid());
        // After return_agent, the cache is valid again (same params).
        state.return_agent(agent);
        assert!(state.cache_is_valid());
    }
}
