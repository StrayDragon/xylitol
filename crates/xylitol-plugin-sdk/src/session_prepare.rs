//! Session prepare Env row + hub port (`session/prepare`).
//!
//! Same family as [`crate::session_env`]: append-only Env CustomMessage, projected
//! as user. Body is the prepare increment (armed MCP names, …), never the full
//! system prompt.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::json;

use crate::message::{AgentMessage, EnvMessage};
use crate::plugin_events::SessionPrepareResult;
use crate::ports::XyTool;

/// `CustomMessage.custom_type` for a prepare-increment Env row.
pub const CUSTOM_TYPE_SESSION_PREPARE: &str = "session_prepare";

/// User-visible notice after the first-turn prepare gate times out.
pub const SESSION_PREPARE_TIMEOUT_NOTICE: &str =
    "MCP gate timed out — tools frozen with armed subset (see /mcp). /reload to retry.";

/// Snapshot compared / persisted as a status-bar-family Env row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPrepareSnapshot {
    pub fingerprint: String,
    pub env_body: String,
}

impl From<&SessionPrepareResult> for SessionPrepareSnapshot {
    fn from(r: &SessionPrepareResult) -> Self {
        Self {
            fingerprint: r.fingerprint.clone(),
            env_body: r.env_body.clone(),
        }
    }
}

impl SessionPrepareSnapshot {
    pub fn to_agent_message(&self) -> AgentMessage {
        AgentMessage::Env(EnvMessage::CustomMessage {
            custom_type: CUSTOM_TYPE_SESSION_PREPARE.into(),
            content: json!(self.env_body),
            display: json!(CUSTOM_TYPE_SESSION_PREPARE),
            details: json!({
                "fingerprint": self.fingerprint,
                "status_bar_kind": CUSTOM_TYPE_SESSION_PREPARE,
            }),
        })
    }
}

/// Best-effort parse of the latest prepare snapshot (scan reverse).
pub fn last_session_prepare(messages: &[AgentMessage]) -> Option<SessionPrepareSnapshot> {
    for msg in messages.iter().rev() {
        if let Some(s) = session_prepare_from_message(msg) {
            return Some(s);
        }
    }
    None
}

pub fn session_prepare_from_message(msg: &AgentMessage) -> Option<SessionPrepareSnapshot> {
    let AgentMessage::Env(EnvMessage::CustomMessage {
        custom_type,
        content,
        details,
        ..
    }) = msg
    else {
        return None;
    };
    if custom_type != CUSTOM_TYPE_SESSION_PREPARE {
        return None;
    }
    let fingerprint = details
        .as_object()
        .and_then(|o| o.get("fingerprint"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let env_body = match content {
        serde_json::Value::String(s) => s.clone(),
        _ => content.to_string(),
    };
    Some(SessionPrepareSnapshot {
        fingerprint,
        env_body,
    })
}

/// Append when fingerprint changed (or missing). Empty fingerprint never appends.
pub fn should_append_session_prepare(
    history: &[AgentMessage],
    next: &SessionPrepareSnapshot,
) -> bool {
    if next.fingerprint.is_empty() {
        return false;
    }
    match last_session_prepare(history) {
        None => true,
        Some(prev) => prev.fingerprint != next.fingerprint,
    }
}

/// Returns `true` if a row was pushed.
pub fn ensure_session_prepare_in_history(
    history: &mut Vec<AgentMessage>,
    next: &SessionPrepareSnapshot,
) -> bool {
    if !should_append_session_prepare(history, next) {
        return false;
    }
    history.push(next.to_agent_message());
    true
}

/// Live MCP / prepare view for loaded-resources (driver maps to its snapshot).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionPrepareView {
    pub configured: usize,
    pub connected: Vec<(String, usize)>,
    pub diags: Vec<String>,
    pub connecting_label: Option<String>,
    pub connecting: bool,
    pub complete: bool,
    pub server_ids: Vec<String>,
    pub armed: Vec<(String, usize)>,
}

impl SessionPrepareView {
    /// Armed-subset result (timeout / snapshot); empty env body.
    pub fn armed_result(&self) -> SessionPrepareResult {
        SessionPrepareResult {
            fingerprint: self
                .armed
                .iter()
                .map(|(id, n)| format!("{id}:{n}"))
                .collect::<Vec<_>>()
                .join("\n"),
            env_body: String::new(),
        }
    }
}

/// Plugin-provided prepare work. Fire starts it; wait joins with timeout.
#[async_trait]
pub trait SessionPrepareHub: Send + Sync {
    /// Synchronous connecting cue for Assembling (no spawn). Default: no-op.
    fn cue_connecting(&self) {}

    /// Start work without waiting (switch / resume / idle reload).
    fn fire(&self);

    /// Awaitable reset (listeners that must not race the next ensure).
    async fn reset_join(&self);

    /// Join in-flight work or start it. Idempotent.
    async fn ensure(&self) -> SessionPrepareResult;

    /// Wait until ensure finishes or `timeout` elapses.
    async fn wait(&self, timeout: Duration) -> SessionPrepareResult;

    fn view(&self) -> SessionPrepareView;

    fn take_notice(&self) -> Option<String>;

    fn tools(&self) -> Vec<Arc<dyn XyTool>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_events::SessionPrepareResult;

    #[test]
    fn append_on_fingerprint_change_only() {
        let mut hist = Vec::new();
        let a = SessionPrepareSnapshot {
            fingerprint: "mcp:a".into(),
            env_body: "armed a".into(),
        };
        assert!(ensure_session_prepare_in_history(&mut hist, &a));
        assert!(!ensure_session_prepare_in_history(&mut hist, &a));
        let b = SessionPrepareSnapshot {
            fingerprint: "mcp:a,b".into(),
            env_body: "armed a,b".into(),
        };
        assert!(ensure_session_prepare_in_history(&mut hist, &b));
        assert_eq!(hist.len(), 2);
        let empty = SessionPrepareSnapshot {
            fingerprint: String::new(),
            env_body: "nope".into(),
        };
        assert!(!ensure_session_prepare_in_history(&mut hist, &empty));
        let _ = SessionPrepareResult::default();
    }

    #[test]
    fn resume_same_fingerprint_does_not_append() {
        let mut hist = Vec::new();
        let snap = SessionPrepareSnapshot {
            fingerprint: "mcp:fs".into(),
            env_body: "armed fs".into(),
        };
        assert!(ensure_session_prepare_in_history(&mut hist, &snap));
        hist.push(AgentMessage::user("hello after prepare"));
        assert!(
            !ensure_session_prepare_in_history(&mut hist, &snap),
            "resume with unchanged prepare fingerprint must not append"
        );
        let next = SessionPrepareSnapshot {
            fingerprint: "mcp:fs,git".into(),
            env_body: "armed fs,git".into(),
        };
        assert!(ensure_session_prepare_in_history(&mut hist, &next));
        assert_eq!(hist.len(), 3);
    }
}
