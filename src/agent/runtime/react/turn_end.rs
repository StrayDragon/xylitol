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

/// How this iteration closes (c2820). Exhaustive: do not add a third "quiet skip".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IterationClose {
    /// Tool calls remain; the inner loop will generate again. Pair `TurnEnd` only.
    ContinueTools,
    /// No further tools (or overflow-error recovery). Settlement + precheck + should_stop.
    Settle,
}

/// Close one iteration. [`IterationClose::Settle`] is the historical `finish_turn` body.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn close_iteration(
    store: &Arc<dyn XySessionStore>,
    session_id: &str,
    model_manager: &Arc<Mutex<crate::agent::model::manager::ModelManager>>,
    event_sink: &Arc<dyn crate::protocol::ports::XyEventSink>,
    compaction_settings: &crate::agent::compaction::CompactionSettings,
    fixed_context: &crate::agent::compaction::FixedRequestContext,
    history: &mut Vec<AgentMessage>,
    overflow_recovery_attempted: &mut bool,
    floor_notice_emitted: &mut bool,
    hooks: &AgentHooks,
    hook_bus: &Option<Arc<dyn XyHookBus>>,
    turn: usize,
    run_baseline: usize,
    assistant: Option<AgentMessage>,
    tool_results: Vec<AgentMessage>,
    steer_queue: &Arc<Mutex<PendingMessageQueue>>,
    follow_up_queue: &Arc<Mutex<PendingMessageQueue>>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
    cwd: &str,
    kind: IterationClose,
) -> FinishTurnResult {
    let turn_index = turn as u32;
    let settlement = match kind {
        IterationClose::Settle => {
            settle_turn_context(
                store,
                session_id,
                model_manager,
                fixed_context,
                turn_obs_parent,
                obs_session,
            )
            .await
        }
        IterationClose::ContinueTools => None,
    };
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
    if kind == IterationClose::ContinueTools {
        return FinishTurnResult {
            events,
            outcome: FinishTurnOutcome::Advanced {
                pending: Vec::new(),
                queue_update: None,
            },
        };
    }
    let will_continue = try_turn_end_compaction(
        store,
        session_id,
        model_manager,
        event_sink,
        compaction_settings,
        fixed_context,
        history,
        overflow_recovery_attempted,
        floor_notice_emitted,
        settlement.as_ref().map(|s| &s.estimate),
        turn_obs_parent,
        obs_session,
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
    fixed_context: &crate::agent::compaction::FixedRequestContext,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
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
            // c25: footer / gate share the overhead-aware estimate (c16).
            fixed_context: Some(fixed_context.clone()),
            obs_parent: turn_obs_parent,
            obs_session: obs_session.clone(),
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
    fixed_context: &crate::agent::compaction::FixedRequestContext,
    history: &mut Vec<AgentMessage>,
    overflow_recovery_attempted: &mut bool,
    floor_notice_emitted: &mut bool,
    precomputed: Option<&crate::protocol::model::ContextTokenEstimate>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
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

    let (summary, ctx_window, model_id, provider) = {
        let mm = crate::utils::lock_mutex(model_manager);
        let meta = match mm.current_model() {
            Some(m) => m,
            None => return false,
        };
        let ctx_window = meta.context_window;
        let model_id = meta.config.model.clone();
        let provider = meta.config.provider_name().to_string();
        let summary = match crate::agent::model::task_model::resolve_compaction_summary(
            settings,
            &mm,
            obs_session,
        ) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("turn-end compaction: no model: {e}");
                return false;
            }
        };
        (summary, ctx_window, model_id, provider)
    };

    let orch = CompactionOrchestrator::new(settings.clone());
    let mut fallback_notice_emitted = false;

    match orch
        .maybe_overflow_compact(
            store.as_ref(),
            session_id,
            &summary,
            event_sink.as_ref(),
            ctx_window,
            &last_assistant,
            &provider,
            &model_id,
            *overflow_recovery_attempted,
            Some(fixed_context),
            turn_obs_parent,
            &mut fallback_notice_emitted,
            Some(&mut *floor_notice_emitted),
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
        fixed_context: Some(fixed_context.clone()),
        obs_parent: turn_obs_parent,
        obs_session: obs_session.clone(),
        ..Default::default()
    };
    if let Err(e) = orch
        .maybe_auto_compact(
            store.as_ref(),
            session_id,
            &summary,
            event_sink.as_ref(),
            ctx_window,
            &opts,
            Some(&last_assistant),
            precomputed,
            Some(fixed_context),
            turn_obs_parent,
            &mut fallback_notice_emitted,
            Some(&mut *floor_notice_emitted),
        )
        .await
    {
        log::warn!("turn-end compaction failed: {e}");
    }
    false
}
