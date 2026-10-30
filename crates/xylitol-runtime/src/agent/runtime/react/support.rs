use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;

use super::super::state::{RunId, SharedRunCoordinator};
use crate::utils::StreamNodeClock;
use xylitol_plugin_sdk::error::XyError;
use xylitol_plugin_sdk::message::{AgentMessage, AgentPart, XyLlmMessage};
use xylitol_plugin_sdk::model::{XyChunk, XyModality, XyToolSchema};
use xylitol_plugin_sdk::session::{EntryBase, MessageEntry, SessionEntry};
use xylitol_plugin_sdk::{XyHookBus, XyHookOutcome, XyModel, XySessionStore};

fn output_budget_from_meta(meta: &xylitol_plugin_sdk::XyModelMeta) -> Option<u32> {
    (meta.max_tokens > 0).then(|| meta.max_tokens.min(u64::from(u32::MAX)) as u32)
}

pub(crate) fn prepare_turn_binding(
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    coordinator: &SharedRunCoordinator,
    run_id: RunId,
    system_prompt: &Option<String>,
    run_model: &mut Option<(String, Arc<dyn XyModel>)>,
) -> Result<(Arc<dyn XyModel>, xylitol_plugin_sdk::XyGenerateOptions), XyError> {
    let mm = crate::utils::lock_mutex(model_manager);
    let thinking = mm.thinking_level();
    let binding = crate::agent::capabilities::ActiveTurnBinding::from_manager(&mm)
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let meta = mm
        .current_model()
        .ok_or_else(|| XyError::Config("no model configured".into()))?;
    let model_id = meta.id.clone();
    let generate_options = xylitol_plugin_sdk::XyGenerateOptions {
        thinking_level: thinking,
        level_map: meta.thinking_level_map.clone(),
        thinking_budgets: mm.thinking_budgets().cloned(),
        system_prompt: system_prompt.clone(),
        max_output_tokens: output_budget_from_meta(meta),
        obs_parent: None,
        obs_session: xylitol_plugin_sdk::ObsSessionContext::default(),
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

#[must_use]
pub(crate) fn outbound_has_image(messages: &[XyLlmMessage]) -> bool {
    messages.iter().any(|message| {
        message
            .content()
            .iter()
            .any(|part| matches!(part, AgentPart::Image(_)))
    })
}

#[must_use]
pub(crate) fn current_model_modality_support(
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    modality: XyModality,
) -> Option<(String, bool)> {
    let mm = crate::utils::lock_mutex(model_manager);
    let meta = mm.current_model()?;
    let supported = mm.registry().supports_modality(&meta.id, modality);
    Some((meta.id.clone(), supported))
}

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

pub(crate) async fn attempt_model_stream(
    model: &Arc<dyn XyModel>,
    llm_messages: Vec<xylitol_plugin_sdk::message::XyLlmMessage>,
    tool_schemas: &[XyToolSchema],
    options: &xylitol_plugin_sdk::XyGenerateOptions,
) -> Result<Pin<Box<dyn Stream<Item = Result<XyChunk, XyError>> + Send>>, XyError> {
    model
        .generate_stream(llm_messages, tool_schemas, true, options.clone())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_budget_maps_zero_to_none() {
        let mut meta = xylitol_plugin_sdk::XyModelMeta {
            id: "m".into(),
            config: xylitol_plugin_sdk::XyModelConfig {
                kind: xylitol_plugin_sdk::XyModelKind::OpenAi,
                api_key: String::new(),
                model: "m".into(),
                base_url: None,
                api: None,
                compat: None,
                prompt_cache: None,
            },
            display_name: "m".into(),
            thinking: false,
            context_window: 1,
            api: String::new(),
            provider: "openai".into(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        };
        assert_eq!(output_budget_from_meta(&meta), None);
        meta.max_tokens = 4096;
        assert_eq!(output_budget_from_meta(&meta), Some(4096));
    }

    #[test]
    fn outbound_has_image_detects_image_parts() {
        let text = XyLlmMessage::user("hello");
        assert!(!outbound_has_image(&[text]));
        let image = XyLlmMessage::user_parts(vec![AgentPart::image("image/png", "abc")]);
        assert!(outbound_has_image(&[image]));
    }
}
