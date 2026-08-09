//! Turn-end settlement, compaction, and finish_turn control flow.

use std::sync::{Arc, Mutex};

use crate::agent::capabilities::{PendingMessageQueue, observe_hook};
use crate::agent::runtime::hooks::{AgentHooks, ShouldStopAfterTurnCtx};
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, LlmMessage};
use crate::protocol::ports::{XyHookBus, XySessionStore};

pub(crate) fn should_stop_after_turn(hooks: &AgentHooks, ctx: &ShouldStopAfterTurnCtx) -> bool {
    hooks
        .should_stop_after_turn
        .as_ref()
        .is_some_and(|hook| hook(ctx))
}

pub(crate) fn drain_queue(queue: &Arc<Mutex<PendingMessageQueue>>) -> Vec<AgentMessage> {
    crate::utils::lock_mutex(queue).drain()
}

pub(crate) fn queue_counts(
    steer: &Arc<Mutex<PendingMessageQueue>>,
    follow_up: &Arc<Mutex<PendingMessageQueue>>,
) -> (usize, usize) {
    let steer_count = crate::utils::lock_mutex(steer).len();
    let follow_up_count = crate::utils::lock_mutex(follow_up).len();
    (steer_count, follow_up_count)
}

/// Events + control-flow decision after a turn settles (text-only or post-tools).
pub(crate) enum FinishTurnOutcome {
    ContinueOuterForCompaction,
    StopRun,
    Advanced {
        pending: Vec<AgentMessage>,
        queue_update: Option<(usize, usize)>,
    },
}

pub(crate) struct FinishTurnResult {
    pub(crate) events: Vec<XyEvent>,
    pub(crate) outcome: FinishTurnOutcome,
}

/// Shared turn-end: settle → TurnEnd → hook → compaction → stop/steer poll.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn finish_turn(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    event_sink: &Arc<dyn crate::protocol::ports::XyEventSink>,
    compaction_settings: &crate::agent::compaction::CompactionSettings,
    history: &mut Vec<AgentMessage>,
    overflow_recovery_attempted: &mut bool,
    hooks: &AgentHooks,
    hook_bus: &Option<Arc<dyn XyHookBus>>,
    turn: usize,
    run_baseline: usize,
    assistant: Option<AgentMessage>,
    tool_results: Vec<AgentMessage>,
    steer_queue: &Arc<Mutex<PendingMessageQueue>>,
    follow_up_queue: &Arc<Mutex<PendingMessageQueue>>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    cwd: &str,
) -> FinishTurnResult {
    let turn_index = turn as u32;
    let settlement = settle_turn_context(store, session_id, model_manager, turn_obs_parent).await;
    let mut events = Vec::new();
    if let Some(s) = &settlement {
        events.push(XyEvent::ContextTokenSettlement {
            estimate: s.estimate.clone(),
            reason: s.reason.as_str().to_string(),
            generation: s.generation,
        });
    }
    events.push(XyEvent::TurnEnd { turn_index });
    if let Some(bus) = hook_bus {
        let (ty, phase, ctx) = crate::agent::runtime::script_hook_ctx::turn_end(turn as u32);
        observe_hook(bus, ty, phase, ctx).await;
    }
    let will_continue = try_turn_end_compaction(
        store,
        session_id,
        model_manager,
        event_sink,
        compaction_settings,
        history,
        overflow_recovery_attempted,
        settlement.as_ref().map(|s| &s.estimate),
        turn_obs_parent,
        cwd,
    )
    .await;
    if will_continue {
        return FinishTurnResult {
            events,
            outcome: FinishTurnOutcome::ContinueOuterForCompaction,
        };
    }
    let stop_ctx = ShouldStopAfterTurnCtx {
        turn_index,
        assistant,
        tool_results,
        history: history.clone(),
        new_messages: history[run_baseline..].to_vec(),
    };
    if should_stop_after_turn(hooks, &stop_ctx) {
        return FinishTurnResult {
            events,
            outcome: FinishTurnOutcome::StopRun,
        };
    }
    let pending = drain_queue(steer_queue);
    let queue_update = if pending.is_empty() {
        None
    } else {
        Some(queue_counts(steer_queue, follow_up_queue))
    };
    FinishTurnResult {
        events,
        outcome: FinishTurnOutcome::Advanced {
            pending,
            queue_update,
        },
    }
}

/// Settle context tokens for turn-end (c1860) — emit via caller `yield`.
pub(crate) async fn settle_turn_context(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
) -> Option<crate::agent::compaction::ContextTokenSettlement> {
    use crate::agent::compaction::{
        ContextTokenSettlementReason, EstimateOpts, settle_from_session_entries,
    };
    let entries = match store.load_leaf_branch(session_id).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("turn-end settlement: load leaf failed: {e}");
            return None;
        }
    };
    let model_id = {
        let mm = crate::utils::lock_mutex(model_manager);
        mm.current_model().map(|m| m.config.model.clone())
    };
    Some(settle_from_session_entries(
        &entries,
        &EstimateOpts {
            model_id,
            obs_parent: turn_obs_parent,
            ..Default::default()
        },
        ContextTokenSettlementReason::TurnSettled,
    ))
}

/// Turn-end compaction: Case1 overflow then Case2 threshold (c1640/c1660).
///
/// Returns `true` when overflow recovery asks the ReAct loop to continue
/// (compact succeeded with willRetry).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn try_turn_end_compaction(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    event_sink: &Arc<dyn crate::protocol::ports::XyEventSink>,
    settings: &crate::agent::compaction::CompactionSettings,
    history: &mut Vec<AgentMessage>,
    overflow_recovery_attempted: &mut bool,
    precomputed: Option<&crate::protocol::model::ContextTokenEstimate>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    cwd: &str,
) -> bool {
    use crate::agent::compaction::{CompactionOrchestrator, EstimateOpts, OverflowCompactOutcome};

    let last_assistant = history
        .iter()
        .rev()
        .find(|m| m.role_name() == "assistant")
        .cloned();
    let Some(last_assistant) = last_assistant else {
        return false;
    };

    let (model, ctx_window, model_id, provider) = {
        let mm = crate::utils::lock_mutex(model_manager);
        let meta = match mm.current_model() {
            Some(m) => m,
            None => return false,
        };
        let ctx_window = meta.context_window;
        let model_id = meta.config.model.clone();
        let provider = meta.config.provider_name().to_string();
        let model = match mm.build_current_model() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("turn-end compaction: no model: {e}");
                return false;
            }
        };
        (model, ctx_window, model_id, provider)
    };

    let orch = CompactionOrchestrator::new(settings.clone());

    match orch
        .maybe_overflow_compact(
            store.as_ref(),
            session_id,
            model.as_ref(),
            event_sink.as_ref(),
            ctx_window,
            &last_assistant,
            &provider,
            &model_id,
            *overflow_recovery_attempted,
            turn_obs_parent,
        )
        .await
    {
        Ok(OverflowCompactOutcome::Ran { will_retry }) => {
            if will_retry {
                *overflow_recovery_attempted = true;
                // Reload compaction-aware leaf context (pi: rebuild after compact).
                match store.load_leaf_branch(session_id).await {
                    Ok(entries) => {
                        let cut = crate::protocol::session::build_context_entries(&entries);
                        *history = cut.iter().filter_map(|e| e.as_agent_message()).collect();
                        // c1906: cut may drop early session_env — ensure before retry generate.
                        let env_snap = crate::agent::prompt::snapshot_for_cwd(cwd);
                        if crate::agent::prompt::ensure_session_env_in_history(history, &env_snap) {
                            super::support::persist_agent_message(
                                store,
                                session_id,
                                history.last().expect("session_env"),
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "overflow retry: reload history failed ({e}); falling back to pop"
                        );
                        if matches!(
                            history.last(),
                            Some(AgentMessage::Llm(LlmMessage::AssistantMessage {
                                stop_reason: Some(crate::protocol::message::XyStopReason::Error),
                                ..
                            }))
                        ) {
                            history.pop();
                        }
                    }
                }
                return true;
            }
            return false;
        }
        Ok(OverflowCompactOutcome::FailedOnce) => {
            return false;
        }
        Ok(OverflowCompactOutcome::Skipped) => {}
        Err(e) => {
            log::warn!("turn-end overflow compaction failed: {e}");
        }
    }

    let opts = EstimateOpts {
        model_id: Some(model_id),
        obs_parent: turn_obs_parent,
        ..Default::default()
    };
    if let Err(e) = orch
        .maybe_auto_compact(
            store.as_ref(),
            session_id,
            model.as_ref(),
            event_sink.as_ref(),
            ctx_window,
            &opts,
            Some(&last_assistant),
            precomputed,
            turn_obs_parent,
        )
        .await
    {
        log::warn!("turn-end compaction failed: {e}");
    }
    false
}
