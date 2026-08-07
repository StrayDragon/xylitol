//! Shared helpers for ReAct setup, persistence, and model retry.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;

use super::super::obs;
use super::super::retry::{RetryState, is_retryable_error};
use super::super::state::{RunId, SharedRunCoordinator};
use crate::agent::llm_project::project_for_llm;
use crate::protocol::error::XyError;
use crate::protocol::message::AgentMessage;
use crate::protocol::model::{XyChunk, XyToolSchema};
use crate::protocol::ports::{XyHookBus, XyHookOutcome, XyModel, XySessionStore};
use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

pub(crate) fn prepare_turn_binding(
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    coordinator: &SharedRunCoordinator,
    run_id: RunId,
    system_prompt: &Option<String>,
    run_model: &mut Option<(String, Arc<dyn XyModel>)>,
) -> Result<(Arc<dyn XyModel>, crate::protocol::ports::XyGenerateOptions), XyError> {
    let mm = crate::utils::lock_mutex(model_manager);
    let meta = mm
        .current_model()
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let model_id = meta.id.clone();
    let thinking = mm.thinking_level();
    let levels = crate::agent::model::manager::ModelManager::levels_for_meta(meta);
    let binding = crate::agent::capabilities::ActiveTurnBinding {
        model_id: meta.id.clone(),
        display_name: if meta.display_name.is_empty() {
            meta.id.clone()
        } else {
            meta.display_name.clone()
        },
        thinking,
        omit_thinking: !crate::protocol::model::ThinkingLevel::is_adjustable(&levels),
    };
    let generate_options = crate::protocol::ports::XyGenerateOptions {
        thinking_level: thinking,
        level_map: meta.thinking_level_map.clone(),
        thinking_budgets: None,
        system_prompt: system_prompt.clone(),
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
    let Ok(message) = serde_json::to_value(message) else {
        return;
    };
    let entry = SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: String::new(),
            parent_id: None,
            timestamp: String::new(),
        },
        message,
    });
    let _ = store.append_session_entry(session_id, &entry).await;
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

/// Helper: call model with retry for transient errors.
pub(crate) async fn call_with_retry(
    model: &Arc<dyn XyModel>,
    messages: Vec<AgentMessage>,
    tool_schemas: &[XyToolSchema],
    retry_state: &RetryState,
    options: &crate::protocol::ports::XyGenerateOptions,
    turn_id: Option<&str>,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, XyError> {
    let llm_messages = project_for_llm(&messages);
    loop {
        match model
            .generate_stream(llm_messages.clone(), tool_schemas, true, options.clone())
            .await
        {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                let err_msg = e.to_string();
                if is_retryable_error(&err_msg) && retry_state.can_retry() {
                    log::warn!(
                        target: "xylitol::react",
                        "model.generate_stream retrying error.kind={} turn_id={} error={e}",
                        e.kind(),
                        turn_id.unwrap_or("")
                    );
                    let delay = retry_state.next_delay();
                    retry_state.backoff(delay).await;
                    continue;
                }
                obs::record_xy_error("model.generate_stream", &e, turn_id);
                return Err(e);
            }
        }
    }
}
