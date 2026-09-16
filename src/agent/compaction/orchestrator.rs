//! CompactionOrchestrator — reserve-formula checks and compaction triggering.
//!
//! Trigger formula matches pi `shouldCompact` (coding-agent compaction.ts):
//! `enabled && contextTokens > contextWindow - reserveTokens`.
//!
//! Manual `compact` = force (pi `AgentSession.compact`); auto threshold = Case2;
//! overflow compact-and-retry = Case1 (c1660).

use crate::agent::compaction::obs::{AgentCompactionSpan, export_skipped};
use crate::agent::compaction::overflow::{assistant_same_model, is_context_overflow_assistant};
use crate::agent::compaction::token_estimator::{EstimateOpts, FixedRequestContext};
use crate::agent::compaction::{
    CompactionError, CompactionSettings, compact_session, prepare_compaction,
};
use crate::agent::model::task_model::CompactionSummaryBinding;
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason};
use crate::protocol::ports::{XyEventSink, XySessionStore};
use crate::protocol::session::SessionEntry;

/// Outcome of overflow Case1 auto-compact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowCompactOutcome {
    /// No overflow path taken (not overflow / skipped).
    Skipped,
    /// Compacted; `will_retry` means caller should continue the model loop.
    Ran { will_retry: bool },
    /// Second overflow after one recovery — failed with user-facing end event.
    FailedOnce,
}

const OVERFLOW_ONCE_MSG: &str = "Context overflow recovery failed after one compact-and-retry attempt. Try reducing context or switching to a larger-context model.";

/// Orchestrates session compaction — threshold checks and execution.
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

    /// Manual force compact (pi `compact(customInstructions?)`). Does **not** apply the reserve gate.
    /// OTel `agent.compaction` starts only after `prepare_compaction` succeeds (otel19).
    #[allow(clippy::too_many_arguments)]
    pub async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
        context_window: u64,
        fixed_context: Option<&FixedRequestContext>,
        fallback_notice_emitted: &mut bool,
    ) -> Result<(), CompactionError> {
        let obs_session = &summary.generate_options.obs_session;
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: "manual".to_string(),
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

        let obs = AgentCompactionSpan::start("manual", None, obs_session);

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
            // c26/c28: settle BEFORE the end event so CompactionEnd can carry
            // tokens_after; manual never carries a c28 floor notice.
            let settled =
                compute_after_compaction_settlement(store, sid, fixed_context, None, obs_session)
                    .await;
            let notice = take_fallback_notice(summary, fallback_notice_emitted, None);
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: result.as_ref().ok().map(|_| "ok".to_string()),
                    aborted: false,
                    reason: "manual".into(),
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

    /// Overflow Case1 (pi `_checkCompaction` overflow branch).
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
            AgentMessage::Llm(LlmMessage::AssistantMessage {
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

    /// Threshold auto-compact (pi Case2).
    ///
    /// When `precomputed` is `Some`, that estimate is used for the reserve gate
    /// (c1860 settlement share with footer). Otherwise estimates quietly from the leaf.
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
        precomputed: Option<&crate::protocol::model::ContextTokenEstimate>,
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
                use crate::agent::compaction::settlement::estimate_quiet;
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

        // c2: the floor is independent of the tokens estimate — it is decided by
        // settings + window + the leaf's measured summary, not by the estimate.
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
            // otel27: auto path stays event-silent but leaves an obs trace.
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

        // c26/c28: settle BEFORE the end event (quiet compute), so CompactionEnd
        // carries tokens_after + the one-shot c28 notice, then the settlement
        // event goes out (order preserved: end → settlement).
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

/// Quietly compute the AfterCompaction settlement (c25/c26) without emitting:
/// estimate of summary row + kept tail + fixed request overhead — the next main
/// request's size. The stale pre-compact Api anchor is dropped inside the estimator.
async fn compute_after_compaction_settlement(
    store: &dyn XySessionStore,
    sid: &str,
    fixed_context: Option<&FixedRequestContext>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
) -> Option<crate::agent::compaction::settlement::ContextTokenSettlement> {
    use crate::agent::compaction::settlement::{
        ContextTokenSettlementReason, settle_from_session_entries,
    };
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
    settled: &crate::agent::compaction::settlement::ContextTokenSettlement,
) {
    event_sink
        .emit(&XyEvent::ContextTokenSettlement {
            estimate: settled.estimate.clone(),
            reason: settled.reason.as_str().to_string(),
            generation: settled.generation,
        })
        .await;
}

/// c28 one-shot actionable diagnostic copy (product scroll-notice register).
fn floor_diagnostic_notice(tokens_after: u64, context_window: u64) -> String {
    format!(
        "Context still ~{tokens_after} tokens after compaction (window {context_window}) — lower keepRecentTokens, raise contextWindow, or trim tool surface"
    )
}

/// Check if compaction should trigger (c2 floor-aware threshold).
///
/// `post_compact_floor` is the [`crate::agent::compaction::projected_post_compact_tokens`]
/// projection; `0` degenerates to the pi reserve formula `window − reserve`.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
    post_compact_floor: u64,
) -> bool {
    if !settings.enabled || context_window == 0 {
        return false;
    }
    context_tokens
        > super::effective_trigger_threshold(context_window, settings, post_compact_floor)
}

fn assistant_is_aborted(assistant: Option<&AgentMessage>) -> bool {
    matches!(
        assistant,
        Some(AgentMessage::Llm(LlmMessage::AssistantMessage {
            stop_reason: Some(XyStopReason::Aborted),
            ..
        }))
    )
}

fn assistant_timestamp_ms(assistant: &AgentMessage) -> Option<u64> {
    match assistant {
        AgentMessage::Llm(LlmMessage::AssistantMessage { timestamp, .. }) if *timestamp > 0 => {
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
        if let AgentMessage::Llm(LlmMessage::AssistantMessage {
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

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use xylitol_ai_bridge::provider::trace::{ObsGateScope, ObsGateState, SpanCollectScope};

    use std::sync::Arc;

    use crate::agent::model::task_model::CompactionSummaryBinding;
    use crate::protocol::error::{XyError, XySessionStoreError};
    use crate::protocol::model::XyToolSchema;
    use crate::protocol::ports::{XyGenerateOptions, XyModel, XyStream};
    use crate::protocol::session::{ForkPosition, SessionContext};

    /// Empty leaf → `prepare_compaction` early-exit; model MUST NOT be touched.
    struct EmptyLeafStore;

    #[async_trait]
    impl XySessionStore for EmptyLeafStore {
        async fn exists(&self, _: &str) -> bool {
            true
        }
        async fn load_entries(&self, _: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(Vec::new())
        }
        async fn append_session_entry(
            &self,
            _: &str,
            _: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            unreachable!("prepare-fail path must not append")
        }
        async fn build_session_context(
            &self,
            _: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            unreachable!("prepare-fail path must not build context")
        }
        async fn create(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }
        async fn fork(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: ForkPosition,
        ) -> Result<(), crate::protocol::error::XySessionError> {
            unreachable!("prepare-fail path must not fork")
        }
        fn set_leaf(&self, _: &str, _: Option<&str>) {}
        fn leaf_id(&self, _: &str) -> Option<String> {
            None
        }
    }

    struct PanicModel;

    #[async_trait]
    impl XyModel for PanicModel {
        fn name(&self) -> &str {
            "panic-model"
        }
        async fn generate_stream(
            &self,
            _: Vec<LlmMessage>,
            _: &[XyToolSchema],
            _: bool,
            _: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            panic!("prepare-fail path must not call the model");
        }
    }

    struct NoopSink;

    #[async_trait]
    impl XyEventSink for NoopSink {
        async fn emit(&self, _: &XyEvent) {}
    }

    struct RecordingSink(std::sync::Mutex<Vec<XyEvent>>);

    #[async_trait]
    impl XyEventSink for RecordingSink {
        async fn emit(&self, event: &XyEvent) {
            self.0.lock().unwrap().push(event.clone());
        }
    }

    /// c25/c26: force compact on a 92fa9adf-shaped session — the AfterCompaction
    /// settlement MUST carry the post-cut placeholder (summary + kept tail),
    /// never the stale pre-compact usage anchor, with no extra model traffic.
    #[tokio::test(flavor = "current_thread")]
    async fn after_compaction_settlement_reports_post_cut_placeholder() {
        use crate::infra::provider::{ScenarioStep, fake_xy_model};
        use crate::infra::session::SessionManager;
        use crate::protocol::lifecycle::XyEvent as Ev;
        use crate::protocol::message::{AgentPart, XyUsage};

        let mgr = SessionManager::in_memory();
        let sid = "after-compact-settle";
        mgr.create(sid, Some("."), None).await.unwrap();
        let usage = XyUsage {
            input: 90_000,
            output: 0,
            total_tokens: 90_000,
            ..Default::default()
        };
        let stale = crate::protocol::message::AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![AgentPart::text("pre-compact anchor")],
            stop_reason: Some(XyStopReason::Stop),
            usage: Some(usage),
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        });
        mgr.append(
            sid,
            &crate::protocol::session::SessionEntry::Message(
                crate::protocol::session::MessageEntry {
                    base: crate::protocol::session::EntryBase {
                        entry_type: "message".into(),
                        id: "a_stale".into(),
                        parent_id: None,
                        timestamp: 1,
                    },
                    message: serde_json::to_value(&stale).unwrap(),
                },
            ),
        )
        .await
        .unwrap();
        // 60 turns ≈ 12k chars/4: below keep(20k) — only the window clamp frees it.
        for i in 0..60 {
            for (id, role, body) in [
                (format!("u{i}"), "user", "x".repeat(400)),
                (format!("a{i}"), "assistant", "y".repeat(400)),
            ] {
                let msg = if role == "user" {
                    crate::protocol::message::AgentMessage::user(body)
                } else {
                    crate::protocol::message::AgentMessage::assistant(body)
                };
                mgr.append(
                    sid,
                    &crate::protocol::session::SessionEntry::Message(
                        crate::protocol::session::MessageEntry {
                            base: crate::protocol::session::EntryBase {
                                entry_type: "message".into(),
                                id,
                                parent_id: None,
                                timestamp: 0,
                            },
                            message: serde_json::to_value(&msg).unwrap(),
                        },
                    ),
                )
                .await
                .unwrap();
            }
        }

        let model = fake_xy_model("sum", vec![ScenarioStep::text("## Goal\nsummarized")]);
        let sink = std::sync::Arc::new(RecordingSink(std::sync::Mutex::new(Vec::new())));
        let orch = CompactionOrchestrator::new(CompactionSettings {
            enabled: true,
            reserve_tokens: 16_384,
            keep_recent_tokens: 20_000,
            ..Default::default()
        });
        let fixed = FixedRequestContext {
            system_prompt: Some("S".repeat(26_000)), // ≈6.5k chars/4 overhead
            tool_schemas: Vec::new(),
        };
        let mut fallback_notice = false;
        orch.compact(
            &mgr,
            sid,
            &CompactionSummaryBinding::for_test(model, "sum"),
            sink.as_ref(),
            None,
            32_768,
            Some(&fixed),
            &mut fallback_notice,
        )
        .await
        .expect("force compact succeeds under clamp");

        let events = sink.0.lock().unwrap().clone();
        let settlement = events
            .iter()
            .find_map(|e| match e {
                Ev::ContextTokenSettlement {
                    estimate, reason, ..
                } if reason == "after_compaction" => Some(estimate.tokens),
                _ => None,
            })
            .expect("AfterCompaction settlement emitted");
        // Steady-state placeholder ≈ kept tail (≈ clamped budget) + overhead;
        // decisively NOT the stale pre-compact 90k anchor.
        assert!(
            settlement < 32_768,
            "placeholder must track the post-cut context, below the window: {settlement}"
        );
        assert!(
            settlement < 90_000 / 4,
            "stale pre-compact anchor must be dropped: {settlement}"
        );
        // CompactionEnd carries the real payload for the wire (pa-wire3 sender side).
        assert!(events.iter().any(|e| matches!(
            e,
            Ev::CompactionEnd {
                tokens_before: Some(_),
                summary: Some(_),
                tokens_after: Some(_),
                notice: None,
                ..
            }
        )));
    }

    /// c28: threshold-path compact whose post-compact projection sits at/over the
    /// window emits the actionable diagnostic exactly once per session flag; a
    /// second qualifying compact does not repeat it.
    #[tokio::test(flavor = "current_thread")]
    async fn floor_notice_fires_once_and_manual_exempt() {
        use crate::infra::provider::{ScenarioStep, fake_xy_model};
        use crate::infra::session::SessionManager;
        use crate::protocol::lifecycle::XyEvent as Ev;

        // window 3000 / reserve 512 / keep 2000 / overhead 3000 tokens.
        // clamp saturates to 0; floor = 3000+0+2048 = 5048; threshold ≈ 6310.
        // Post-compact settlement ≈ overhead + small rows ≈ 3.2k ≥ window → the
        // c28 diagnostic condition holds for both qualifying compacts.
        let window: u64 = 3_000;
        let overhead_tokens: u64 = 3_000;
        let big_summary = format!("## Goal\n{}", "x".repeat(12_000));

        let mgr = SessionManager::in_memory();
        let sid = "floor-notice";
        mgr.create(sid, Some("."), None).await.unwrap();
        let sink = std::sync::Arc::new(RecordingSink(std::sync::Mutex::new(Vec::new())));
        let orch = CompactionOrchestrator::new(CompactionSettings {
            enabled: true,
            reserve_tokens: 512,
            keep_recent_tokens: 2_000,
            ..Default::default()
        });
        let fixed = FixedRequestContext {
            system_prompt: Some("S".repeat((overhead_tokens * 4) as usize)),
            tool_schemas: Vec::new(),
        };
        let opts = EstimateOpts {
            fixed_context: Some(fixed.clone()),
            ..Default::default()
        };
        let mut flag = false;
        let mut fallback_notice = false;

        for i in 0..100 {
            append_turn(&mgr, sid, i).await;
        }
        let model = fake_xy_model("sum", vec![ScenarioStep::text(big_summary.clone())]);
        orch.maybe_auto_compact(
            &mgr,
            sid,
            &CompactionSummaryBinding::for_test(model, "sum"),
            sink.as_ref(),
            window,
            &opts,
            None,
            None,
            Some(&fixed),
            None,
            &mut fallback_notice,
            Some(&mut flag),
        )
        .await
        .expect("first auto compact");

        for i in 0..100 {
            append_turn(&mgr, sid, i).await;
        }
        let model = fake_xy_model("sum", vec![ScenarioStep::text(big_summary.clone())]);
        orch.maybe_auto_compact(
            &mgr,
            sid,
            &CompactionSummaryBinding::for_test(model, "sum"),
            sink.as_ref(),
            window,
            &opts,
            None,
            None,
            Some(&fixed),
            None,
            &mut fallback_notice,
            Some(&mut flag),
        )
        .await
        .expect("second auto compact");

        let events = sink.0.lock().unwrap().clone();
        let ends: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Ev::CompactionEnd {
                    result: Some(_),
                    tokens_after,
                    notice,
                    ..
                } => Some((*tokens_after, notice.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(ends.len(), 2, "two successful auto compacts: {ends:?}");
        let noticed = ends.iter().filter(|(_, n)| n.is_some()).count();
        assert_eq!(noticed, 1, "diagnostic must fire exactly once: {ends:?}");
        assert!(flag, "once flag must be set after the first notice");
        assert!(
            ends.iter().all(|(m, _)| m.is_some_and(|v| v >= window)),
            "post-compact projection must sit at/over the window: {ends:?}"
        );
    }

    /// c2 anti-churn: 32k-shape small window — appended turns must NOT compact
    /// every turn-end; each compact must buy at least the hysteresis band.
    #[tokio::test(flavor = "current_thread")]
    async fn floor_threshold_bounds_compaction_frequency() {
        use crate::infra::provider::{ScenarioStep, fake_xy_model};
        use crate::infra::session::SessionManager;

        let window: u64 = 32_768;
        let overhead_tokens: u64 = 9_000;
        let mgr = SessionManager::in_memory();
        let sid = "floor-churn";
        mgr.create(sid, Some("."), None).await.unwrap();

        let sink = std::sync::Arc::new(RecordingSink(std::sync::Mutex::new(Vec::new())));
        let orch = CompactionOrchestrator::new(CompactionSettings {
            enabled: true,
            reserve_tokens: 16_384,
            keep_recent_tokens: 20_000,
            ..Default::default()
        });
        let fixed = FixedRequestContext {
            system_prompt: Some("S".repeat((overhead_tokens * 4) as usize)),
            tool_schemas: Vec::new(),
        };
        let opts = EstimateOpts {
            fixed_context: Some(fixed.clone()),
            ..Default::default()
        };
        let mut flag = false;
        let mut fallback_notice = false;

        let mut compactions = 0usize;
        for i in 0..240 {
            append_turn(&mgr, sid, i).await;
            let model = fake_xy_model("sum", vec![ScenarioStep::text("## Goal\ns")]);
            let ran = orch
                .maybe_auto_compact(
                    &mgr,
                    sid,
                    &CompactionSummaryBinding::for_test(model, "sum"),
                    sink.as_ref(),
                    window,
                    &opts,
                    None,
                    None,
                    Some(&fixed),
                    None,
                    &mut fallback_notice,
                    Some(&mut flag),
                )
                .await
                .expect("auto compact");
            compactions += usize::from(ran);
        }
        // 240 turns ≈ 48k tokens + 9k overhead. The old reserve-only formula
        // compacted on essentially every turn-end once over threshold (200+).
        // The floor-aware threshold buys a ≈10-turn cycle here (≈24 rounds) —
        // every compact ≥ the hysteresis band instead of a no-gain rewrite.
        assert!(
            compactions > 1 && compactions <= 40,
            "floor threshold must bound churn, got {compactions} compactions in 240 turns"
        );
    }

    /// otel19: prepare early-exit MUST NOT export `agent.compaction`.
    /// `current_thread` so scoped gates / collect stay on the worker that entered them.
    #[tokio::test(flavor = "current_thread")]
    async fn prepare_fail_exports_no_compaction_span() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        let orch = CompactionOrchestrator::new(CompactionSettings::default());
        let mut fallback_notice = false;
        let err = orch
            .compact(
                &EmptyLeafStore,
                "sid",
                &CompactionSummaryBinding::for_test(Arc::new(PanicModel), "panic-model"),
                &NoopSink,
                None,
                0,
                None,
                &mut fallback_notice,
            )
            .await
            .expect_err("empty session must fail prepare");
        assert!(
            err.to_string().contains("Nothing to compact"),
            "unexpected prepare error: {err}"
        );

        fastrace::flush();

        let spans = collect.records();
        assert!(
            spans.iter().all(|s| s.name != "agent.compaction"),
            "prepare early-exit must not export agent.compaction; got: {:?}",
            spans.iter().map(|s| s.name.as_ref()).collect::<Vec<_>>()
        );
        // otel27: the early exit is visible under its own name instead.
        assert!(
            spans.iter().any(|s| s.name == "agent.compaction.skipped"),
            "prepare early-exit must export agent.compaction.skipped; got: {:?}",
            spans.iter().map(|s| s.name.as_ref()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn should_compact_disabled() {
        let s = CompactionSettings {
            enabled: false,
            ..Default::default()
        };
        assert!(!should_compact(100_000, 200_000, &s, 0));
    }

    #[test]
    fn should_compact_window_zero() {
        let s = CompactionSettings::default();
        assert!(!should_compact(100_000, 0, &s, 0));
    }

    #[test]
    fn should_compact_not_exceeded() {
        let s = CompactionSettings::default();
        assert!(!should_compact(50_000, 200_000, &s, 0));
    }

    #[test]
    fn should_compact_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        assert!(should_compact(200_000, 200_000, &s, 0));
    }

    #[test]
    fn should_compact_exact_boundary_not_trigger() {
        let s = CompactionSettings::default();
        assert!(!should_compact(183_616, 200_000, &s, 0));
    }

    #[test]
    fn should_compact_one_over_boundary() {
        let s = CompactionSettings::default();
        assert!(should_compact(183_617, 200_000, &s, 0));
    }

    /// c2 32k regression: real-measured shape (window 32768 / reserve 16384 /
    /// keep 20000 / overhead ≈9k, no prior summary) — the old reserve formula
    /// triggered below the floor and compacted every turn (40/40). The
    /// floor-aware threshold must hold below 23,040 and fire above it.
    #[test]
    fn floor_threshold_holds_32k_shape() {
        let s = CompactionSettings {
            enabled: true,
            reserve_tokens: 16_384,
            keep_recent_tokens: 20_000,
            ..Default::default()
        };
        let floor = crate::agent::compaction::projected_post_compact_tokens(
            &s,
            32_768,
            9_000,
            crate::agent::compaction::summary_placeholder_tokens(&[]),
        );
        assert_eq!(floor, 9_000 + 7_384 + 2_048);
        let threshold = crate::agent::compaction::effective_trigger_threshold(32_768, &s, floor);
        assert_eq!(threshold, 23_040);
        assert!(!should_compact(20_000, 32_768, &s, floor));
        assert!(should_compact(23_041, 32_768, &s, floor));
    }

    #[test]
    fn floor_zero_degenerates_to_reserve_formula() {
        let s = CompactionSettings {
            enabled: true,
            reserve_tokens: 16_384,
            keep_recent_tokens: 20_000,
            ..Default::default()
        };
        assert!(!should_compact(16_384, 32_768, &s, 0));
        assert!(should_compact(16_385, 32_768, &s, 0));
    }

    #[test]
    fn summary_placeholder_measures_latest_entry() {
        use crate::protocol::session::{CompactionEntry, EntryBase};
        let entry = CompactionEntry {
            base: EntryBase {
                entry_type: "compaction".into(),
                id: "c1".into(),
                parent_id: None,
                timestamp: 1,
            },
            summary: "x".repeat(8_192),
            first_kept_entry_id: "m1".into(),
            tokens_before: 1_000,
            details: None,
            from_hook: None,
            policy: None,
        };
        let entries = vec![SessionEntry::Compaction(entry)];
        assert_eq!(
            crate::agent::compaction::summary_placeholder_tokens(&entries),
            2_048
        );
        assert_eq!(
            crate::agent::compaction::summary_placeholder_tokens(&[]),
            2_048
        );
    }

    #[test]
    fn aborted_assistant_detected() {
        let msg = AgentMessage::Llm(LlmMessage::AssistantMessage {
            content: vec![crate::protocol::message::AgentPart::text("x")],
            stop_reason: Some(XyStopReason::Aborted),
            usage: None,
            api: String::new(),
            provider: String::new(),
            model: String::new(),
            response_id: None,
            error_message: None,
            timestamp: 1,
            diagnostics: Vec::new(),
        });
        assert!(assistant_is_aborted(Some(&msg)));
    }

    /// Two 400-char messages ≈ 200 tokens per appended turn (heuristic chars/4).
    async fn append_turn(mgr: &crate::infra::session::SessionManager, sid: &str, i: usize) {
        for (id, role, body) in [
            (format!("u{i}"), "user", "x".repeat(400)),
            (format!("a{i}"), "assistant", "y".repeat(400)),
        ] {
            let msg = if role == "user" {
                AgentMessage::user(body)
            } else {
                AgentMessage::assistant(body)
            };
            let _ = mgr
                .append(
                    sid,
                    &SessionEntry::Message(crate::protocol::session::MessageEntry {
                        base: crate::protocol::session::EntryBase {
                            entry_type: "message".into(),
                            id,
                            parent_id: None,
                            timestamp: 0,
                        },
                        message: serde_json::to_value(&msg).unwrap(),
                    }),
                )
                .await;
        }
    }
}
