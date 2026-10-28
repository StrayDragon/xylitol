use crate::CompactionSummaryBinding;
use crate::obs::{AgentCompactionSpan, export_skipped};
use crate::overflow::{assistant_same_model, is_context_overflow_assistant};
use crate::token_estimator::{EstimateOpts, FixedRequestContext};
use crate::{CompactionError, CompactionSettings, compact_session, prepare_compaction};
use xylitol_plugin_sdk::XyCompaction;
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::message::{AgentMessage, XyLlmMessage, XyStopReason};
use xylitol_plugin_sdk::session::SessionEntry;
use xylitol_plugin_sdk::{XyEventSink, XySessionStore};

pub use xylitol_plugin_sdk::OverflowCompactOutcome;

const OVERFLOW_ONCE_MSG: &str = "Context overflow recovery failed after one compact-and-retry attempt. Try reducing context or switching to a larger-context model.";

pub struct CompactionOrchestrator {
    settings: CompactionSettings,
}

#[allow(clippy::too_many_arguments)]
impl CompactionOrchestrator {
    pub fn new(settings: CompactionSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &CompactionSettings {
        &self.settings
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
        reason: &str,
        context_window: u64,
        fixed_context: Option<&FixedRequestContext>,
        fallback_notice_emitted: &mut bool,
    ) -> Result<(), CompactionError> {
        let obs_session = &summary.generate_options.obs_session;
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: reason.to_string(),
            })
            .await;

        let entries = store.load_leaf_branch(sid).await?;
        let overhead = fixed_context.map_or(0, FixedRequestContext::overhead_tokens);
        if let Some(err) =
            prepare_compaction(&entries, &self.settings, context_window, overhead).err()
        {
            let error_message = err.to_string();
            export_skipped(&error_message, None, obs_session);
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: None,
                    aborted: false,
                    reason: "manual".into(),
                    will_retry: false,
                    error_message: Some(error_message),
                    summary: None,
                    tokens_before: None,
                    tokens_after: None,
                    notice: None,
                })
                .await;
            return Err(err);
        }

        let obs = AgentCompactionSpan::start(reason, None, obs_session);

        let mut force_settings = self.settings.clone();
        force_settings.enabled = true;

        let llm_parent = obs.as_ref().and_then(|s| s.span_context());
        let result = compact_session(
            store,
            sid,
            summary,
            &force_settings,
            instructions.as_deref(),
            context_window,
            fixed_context,
            llm_parent,
        )
        .await;

        if let Some(obs) = obs {
            obs.finish(
                false,
                false,
                result.as_ref().err().map(|e| e.to_string()).as_deref(),
            );
        }
        if result.is_ok() {
            let settled =
                compute_after_compaction_settlement(store, sid, fixed_context, None, obs_session)
                    .await;
            let notice = take_fallback_notice(summary, fallback_notice_emitted, None);
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: result.as_ref().ok().map(|_| "ok".to_string()),
                    aborted: false,
                    reason: reason.to_string(),
                    will_retry: false,
                    error_message: result
                        .as_ref()
                        .err()
                        .map(|e| format!("compaction failed: {e}")),
                    summary: result.as_ref().ok().map(|e| e.summary.clone()),
                    tokens_before: result.as_ref().ok().map(|e| e.tokens_before),
                    tokens_after: settled.as_ref().map(|s| s.estimate.tokens),
                    notice,
                })
                .await;
            if let Some(settled) = settled {
                emit_settlement_event(event_sink, &settled).await;
            }
        }

        result?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn maybe_overflow_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        last_assistant: &AgentMessage,
        current_provider: &str,
        current_model_id: &str,
        overflow_recovery_attempted: bool,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<OverflowCompactOutcome, CompactionError> {
        if !self.settings.enabled {
            return Ok(OverflowCompactOutcome::Skipped);
        }
        if assistant_is_aborted(Some(last_assistant)) {
            return Ok(OverflowCompactOutcome::Skipped);
        }

        let entries = store.load_leaf_branch(sid).await?;
        if assistant_is_stale_vs_compaction(Some(last_assistant), &entries) {
            return Ok(OverflowCompactOutcome::Skipped);
        }

        if !assistant_same_model(last_assistant, current_provider, current_model_id) {
            return Ok(OverflowCompactOutcome::Skipped);
        }
        if !is_context_overflow_assistant(last_assistant, context_window) {
            return Ok(OverflowCompactOutcome::Skipped);
        }

        let will_retry = !matches!(
            last_assistant,
            AgentMessage::Llm(XyLlmMessage::AssistantMessage {
                stop_reason: Some(XyStopReason::Stop),
                ..
            })
        );

        if !will_retry {
            return self
                .run_auto_compaction(
                    store,
                    sid,
                    summary,
                    event_sink,
                    "overflow",
                    false,
                    context_window,
                    &entries,
                    fixed_context,
                    turn_obs_parent,
                    fallback_notice_emitted,
                    floor_notice_emitted,
                )
                .await
                .map(|ran| {
                    if ran {
                        OverflowCompactOutcome::Ran { will_retry: false }
                    } else {
                        OverflowCompactOutcome::Skipped
                    }
                });
        }

        if overflow_recovery_attempted {
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: None,
                    aborted: false,
                    reason: "overflow".into(),
                    will_retry: false,
                    error_message: Some(OVERFLOW_ONCE_MSG.into()),
                    summary: None,
                    tokens_before: None,
                    tokens_after: None,
                    notice: None,
                })
                .await;
            return Ok(OverflowCompactOutcome::FailedOnce);
        }

        self.run_auto_compaction(
            store,
            sid,
            summary,
            event_sink,
            "overflow",
            true,
            context_window,
            &entries,
            fixed_context,
            turn_obs_parent,
            fallback_notice_emitted,
            floor_notice_emitted,
        )
        .await
        .map(|ran| {
            if ran {
                OverflowCompactOutcome::Ran { will_retry: true }
            } else {
                OverflowCompactOutcome::Skipped
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn maybe_auto_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
        last_assistant: Option<&AgentMessage>,
        precomputed: Option<&xylitol_plugin_sdk::model::ContextTokenEstimate>,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, CompactionError> {
        if !self.settings.enabled {
            return Ok(false);
        }

        if assistant_is_aborted(last_assistant) {
            return Ok(false);
        }

        let entries = store.load_leaf_branch(sid).await?;

        if assistant_is_stale_vs_compaction(last_assistant, &entries) {
            return Ok(false);
        }

        let estimate = match precomputed {
            Some(e) => e.clone(),
            None => {
                use crate::settlement::estimate_quiet;
                let mut quiet_opts = estimate_opts.clone();
                quiet_opts.fixed_context = fixed_context.cloned();
                estimate_quiet(&entries, &quiet_opts)
            }
        };
        if estimate.tokens == 0 {
            return Ok(false);
        }

        if usage_anchor_stale_vs_compaction(&entries) {
            return Ok(false);
        }

        let overhead = fixed_context.map_or(0, FixedRequestContext::overhead_tokens);
        let floor = super::projected_post_compact_tokens(
            &self.settings,
            context_window,
            overhead,
            super::summary_placeholder_tokens(&entries),
        );
        if !should_compact(estimate.tokens, context_window, &self.settings, floor) {
            return Ok(false);
        }

        let reason = format!(
            "threshold: {} tokens over reserve of {}k window",
            estimate.tokens,
            context_window / 1000,
        );
        self.run_auto_compaction(
            store,
            sid,
            summary,
            event_sink,
            &reason,
            false,
            context_window,
            &entries,
            fixed_context,
            turn_obs_parent,
            fallback_notice_emitted,
            floor_notice_emitted,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_auto_compaction(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary_binding: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        reason: &str,
        will_retry: bool,
        context_window: u64,
        entries: &[SessionEntry],
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, CompactionError> {
        let obs_session = &summary_binding.generate_options.obs_session;
        let overhead = fixed_context.map_or(0, FixedRequestContext::overhead_tokens);
        if let Err(err) = prepare_compaction(entries, &self.settings, context_window, overhead) {
            export_skipped(&err.to_string(), turn_obs_parent, obs_session);
            return Ok(false);
        }

        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: reason.to_string(),
            })
            .await;
        let obs = AgentCompactionSpan::start(reason, turn_obs_parent, obs_session);

        let llm_parent = obs.as_ref().and_then(|s| s.span_context());
        let result = compact_session(
            store,
            sid,
            summary_binding,
            &self.settings,
            None,
            context_window,
            fixed_context,
            llm_parent,
        )
        .await;
        let (ok_result, err_msg, summary_text, tokens_before) = match &result {
            Ok(entry) => (
                Some("ok".to_string()),
                None,
                Some(entry.summary.clone()),
                Some(entry.tokens_before),
            ),
            Err(e) => (
                None,
                Some(if reason.starts_with("overflow") {
                    format!("Context overflow recovery failed: {e}")
                } else {
                    format!("Auto-compaction failed: {e}")
                }),
                None,
                None,
            ),
        };

        let end_reason = if reason.starts_with("overflow") {
            "overflow".into()
        } else if reason.starts_with("threshold") {
            "threshold".into()
        } else {
            reason.to_string()
        };
        let will_retry_end = will_retry && result.is_ok();
        if let Some(obs) = obs {
            obs.finish(will_retry_end, false, err_msg.as_deref());
        }

        let mut tokens_after = None;
        let mut floor_notice = None;
        let mut settled_event = None;
        if result.is_ok()
            && let Some(settled) = compute_after_compaction_settlement(
                store,
                sid,
                fixed_context,
                turn_obs_parent,
                obs_session,
            )
            .await
        {
            tokens_after = Some(settled.estimate.tokens);
            if context_window > 0
                && settled.estimate.tokens >= context_window
                && let Some(flag) = floor_notice_emitted
                && !*flag
            {
                *flag = true;
                floor_notice = Some(floor_diagnostic_notice(
                    settled.estimate.tokens,
                    context_window,
                ));
            }
            settled_event = Some(settled);
        }
        let notice = take_fallback_notice(summary_binding, fallback_notice_emitted, floor_notice);

        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: ok_result,
                aborted: false,
                reason: end_reason,
                will_retry: will_retry_end,
                error_message: err_msg,
                summary: summary_text,
                tokens_before,
                tokens_after,
                notice,
            })
            .await;

        if let Some(settled) = settled_event {
            emit_settlement_event(event_sink, &settled).await;
        }

        match result {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}

fn take_fallback_notice(
    summary_binding: &CompactionSummaryBinding,
    fallback_notice_emitted: &mut bool,
    floor_notice: Option<String>,
) -> Option<String> {
    let fallback = if !*fallback_notice_emitted {
        summary_binding.attribution.notice_message()
    } else {
        None
    };
    if fallback.is_some() {
        *fallback_notice_emitted = true;
    }
    match (fallback, floor_notice) {
        (Some(fb), Some(fl)) => Some(format!("{fb}\n{fl}")),
        (Some(fb), None) => Some(fb),
        (None, fl) => fl,
    }
}

async fn compute_after_compaction_settlement(
    store: &dyn XySessionStore,
    sid: &str,
    fixed_context: Option<&FixedRequestContext>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_plugin_sdk::ObsSessionContext,
) -> Option<crate::settlement::ContextTokenSettlement> {
    use crate::settlement::{ContextTokenSettlementReason, settle_from_session_entries};
    let Ok(fresh) = store.load_leaf_branch(sid).await else {
        return None;
    };
    Some(settle_from_session_entries(
        &fresh,
        &EstimateOpts {
            fixed_context: fixed_context.cloned(),
            obs_parent: turn_obs_parent,
            obs_session: obs_session.clone(),
            ..Default::default()
        },
        ContextTokenSettlementReason::AfterCompaction,
    ))
}

async fn emit_settlement_event(
    event_sink: &dyn XyEventSink,
    settled: &crate::settlement::ContextTokenSettlement,
) {
    event_sink
        .emit(&XyEvent::ContextTokenSettlement {
            estimate: settled.estimate.clone(),
            reason: settled.reason.as_str().to_string(),
            generation: settled.generation,
        })
        .await;
}

fn floor_diagnostic_notice(tokens_after: u64, context_window: u64) -> String {
    format!(
        "Context still ~{tokens_after} tokens after compaction (window {context_window}) — lower keepRecentTokens, raise contextWindow, or trim tool surface"
    )
}

pub use xylitol_plugin_sdk::should_compact;

fn assistant_is_aborted(assistant: Option<&AgentMessage>) -> bool {
    matches!(
        assistant,
        Some(AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            stop_reason: Some(XyStopReason::Aborted),
            ..
        }))
    )
}

fn assistant_timestamp_ms(assistant: &AgentMessage) -> Option<u64> {
    match assistant {
        AgentMessage::Llm(XyLlmMessage::AssistantMessage { timestamp, .. }) if *timestamp > 0 => {
            Some(*timestamp)
        }
        _ => None,
    }
}

fn latest_compaction_ms(entries: &[SessionEntry]) -> Option<u64> {
    entries.iter().rev().find_map(|e| match e {
        SessionEntry::Compaction(c) => Some(c.base.timestamp),
        _ => None,
    })
}

fn assistant_is_stale_vs_compaction(
    assistant: Option<&AgentMessage>,
    entries: &[SessionEntry],
) -> bool {
    let (Some(asst), Some(comp_ms)) = (assistant, latest_compaction_ms(entries)) else {
        return false;
    };
    match assistant_timestamp_ms(asst) {
        Some(ts) => ts <= comp_ms,
        None => false,
    }
}

fn usage_anchor_stale_vs_compaction(entries: &[SessionEntry]) -> bool {
    let Some(comp_ms) = latest_compaction_ms(entries) else {
        return false;
    };
    for entry in entries.iter().rev() {
        let Some(msg) = entry.as_agent_message() else {
            continue;
        };
        if let AgentMessage::Llm(XyLlmMessage::AssistantMessage {
            usage: Some(_),
            timestamp,
            ..
        }) = &msg
        {
            if *timestamp > 0 {
                return *timestamp <= comp_ms;
            }
            if let Some(entry_ms) = entry.base().map(|b| b.timestamp) {
                return entry_ms <= comp_ms;
            }
            return false;
        }
    }
    false
}

#[async_trait::async_trait]
impl XyCompaction for CompactionOrchestrator {
    fn settings(&self) -> &CompactionSettings {
        &self.settings
    }

    async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
        reason: &str,
        context_window: u64,
        fixed_context: Option<&FixedRequestContext>,
        fallback_notice_emitted: &mut bool,
    ) -> Result<(), CompactionError> {
        CompactionOrchestrator::compact(
            self,
            store,
            sid,
            summary,
            event_sink,
            instructions,
            reason,
            context_window,
            fixed_context,
            fallback_notice_emitted,
        )
        .await
    }

    async fn maybe_overflow_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        last_assistant: &AgentMessage,
        current_provider: &str,
        current_model_id: &str,
        overflow_recovery_attempted: bool,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<OverflowCompactOutcome, CompactionError> {
        CompactionOrchestrator::maybe_overflow_compact(
            self,
            store,
            sid,
            summary,
            event_sink,
            context_window,
            last_assistant,
            current_provider,
            current_model_id,
            overflow_recovery_attempted,
            fixed_context,
            turn_obs_parent,
            fallback_notice_emitted,
            floor_notice_emitted,
        )
        .await
    }

    async fn maybe_auto_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
        last_assistant: Option<&AgentMessage>,
        precomputed: Option<&xylitol_plugin_sdk::model::ContextTokenEstimate>,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, CompactionError> {
        CompactionOrchestrator::maybe_auto_compact(
            self,
            store,
            sid,
            summary,
            event_sink,
            context_window,
            estimate_opts,
            last_assistant,
            precomputed,
            fixed_context,
            turn_obs_parent,
            fallback_notice_emitted,
            floor_notice_emitted,
        )
        .await
    }
}
