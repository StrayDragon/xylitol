//! Shared helpers for ReAct setup and persistence.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;

use super::super::state::{RunId, SharedRunCoordinator};
use crate::protocol::error::XyError;
use crate::protocol::message::AgentMessage;
use crate::protocol::model::{XyChunk, XyToolSchema};
use crate::protocol::ports::{XyHookBus, XyHookOutcome, XyModel, XySessionStore};
use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};
use crate::utils::StreamNodeClock;

pub(crate) fn prepare_turn_binding(
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    coordinator: &SharedRunCoordinator,
    run_id: RunId,
    system_prompt: &Option<String>,
    run_model: &mut Option<(String, Arc<dyn XyModel>)>,
) -> Result<(Arc<dyn XyModel>, crate::protocol::ports::XyGenerateOptions), XyError> {
    let mm = crate::utils::lock_mutex(model_manager);
    let thinking = mm.thinking_level();
    let binding = crate::agent::capabilities::ActiveTurnBinding::from_manager(&mm)
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let meta = mm
        .current_model()
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let model_id = meta.id.clone();
    let generate_options = crate::protocol::ports::XyGenerateOptions {
        thinking_level: thinking,
        level_map: meta.thinking_level_map.clone(),
        thinking_budgets: mm.thinking_budgets().cloned(),
        system_prompt: system_prompt.clone(),
        obs_parent: None,
        obs_session: xylitol_ai_bridge::ObsSessionContext::default(),
    };
    let model = match run_model.as_ref() {
        Some((id, model)) if id == &model_id => Arc::clone(model),
        _ => {
            let built = mm.build_current_model()?;
            *run_model = Some((model_id, Arc::clone(&built)));
            built
        }
    };
    drop(mm);
    coordinator.with_mut(|c| c.set_active_turn(run_id, binding));
    Ok((model, generate_options))
}

/// Clears active-turn binding for `run_id` when the ReAct stream drops.
pub(crate) struct ClearActiveTurn {
    pub(crate) coordinator: SharedRunCoordinator,
    pub(crate) run_id: RunId,
}

impl Drop for ClearActiveTurn {
    fn drop(&mut self) {
        self.coordinator
            .with_mut(|c| c.clear_active_turn_if(self.run_id));
    }
}

pub(crate) async fn persist_agent_message(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    message: &AgentMessage,
) {
    persist_agent_message_with_thought_elapsed(store, session_id, message, None).await;
}

/// Persist an assistant message, optionally stamping `thinkingElapsedSecs` and
/// `streamTiming` node unix-ms for TUI resume. Extra JSON is ignored when
/// history deserializes to [`AgentMessage`] for LLM projection.
pub(crate) async fn persist_agent_message_with_thought_elapsed(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    message: &AgentMessage,
    stream_clock: Option<&StreamNodeClock>,
) {
    let Ok(mut message) = serde_json::to_value(message) else {
        return;
    };
    if let Some(clock) = stream_clock {
        if let Some(secs) = clock.thinking_elapsed_secs() {
            message["thinkingElapsedSecs"] = serde_json::json!(secs);
        }
        let pairs = clock.timing_pairs();
        if !pairs.is_empty() {
            let mut timing = serde_json::Map::new();
            for (key, ms) in pairs {
                timing.insert(key.to_string(), serde_json::json!(ms));
            }
            message["streamTiming"] = serde_json::Value::Object(timing);
        }
    }
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: 0,
        },
        message,
    });
    if let Err(error) = store.append_session_entry(session_id, &entry).await {
        log::warn!(target: "xylitol::session", "persist agent message failed: {error}");
    }
}

pub(crate) async fn observe_script_hook(
    bus: &Arc<dyn XyHookBus>,
    event_type: &str,
    phase: &str,
    context: serde_json::Value,
) {
    if let XyHookOutcome::Blocked { reason } = bus.dispatch(event_type, phase, context).await {
        log::warn!(
            "Script hook blocked observe-only lifecycle event (fail-open) event={} phase={} reason={}",
            event_type,
            phase,
            reason
        );
    }
}

/// 单次模型连接尝试（投影 + generate_stream）。重试环在 react 生成器内联,
/// 以便在尝试之间 yield `AutoRetryStart` / `AutoRetryEnd` 事件。
pub(crate) async fn attempt_model_stream(
    model: &Arc<dyn XyModel>,
    llm_messages: Vec<crate::protocol::message::LlmMessage>,
    tool_schemas: &[XyToolSchema],
    options: &crate::protocol::ports::XyGenerateOptions,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, XyError> {
    model
        .generate_stream(llm_messages, tool_schemas, true, options.clone())
        .await
}
