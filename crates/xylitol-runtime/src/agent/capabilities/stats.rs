use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::XyError;

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub session_id: String,
    pub user_messages: usize,
    pub assistant_messages: usize,
    pub total_messages: usize,
    pub thinking_level: String,
    pub model: Option<(String, String)>,
}

pub async fn compute(
    store: &dyn XySessionStore,
    session_id: &str,
) -> Result<SessionStats, XyError> {
    let ctx = store
        .build_session_context(session_id)
        .await
        .map_err(XyError::from)?;
    let user_messages = ctx
        .messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .count();
    let assistant_messages = ctx
        .messages
        .iter()
        .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"))
        .count();
    let total_messages = ctx.messages.len();

    Ok(SessionStats {
        session_id: session_id.to_string(),
        user_messages,
        assistant_messages,
        total_messages,
        thinking_level: ctx.thinking_level,
        model: ctx.model,
    })
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ContextUsage {
    pub tokens: u64,
    pub context_window: u64,
    pub percent: u64,
    pub should_compact: bool,
}

pub use crate::agent::compaction::should_compact;

#[allow(dead_code)]
pub fn get_context_usage(
    token_estimate: u64,
    context_window: u64,
    settings: &crate::agent::compaction::CompactionSettings,
) -> ContextUsage {
    let percent = if context_window > 0 {
        ((token_estimate as f64 / context_window as f64) * 100.0) as u64
    } else {
        0
    };
    ContextUsage {
        tokens: token_estimate,
        context_window,
        percent,
        should_compact: should_compact(token_estimate, context_window, settings, 0),
    }
}
