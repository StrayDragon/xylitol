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
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::{AgentMessage, LlmMessage, XyStopReason};
use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore};
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
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
        context_window: u64,
        fixed_context: Option<&FixedRequestContext>,
        obs_session: &xylitol_ai_bridge::ObsSessionContext,
    ) -> Result<(), CompactionError> {
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
            model,
            &force_settings,
            instructions.as_deref(),
            context_window,
            fixed_context,
            llm_parent,
            obs_session,
        )
        .await;

        if let Some(obs) = obs {
            obs.finish(
                false,
                false,
                result.as_ref().err().map(|e| e.to_string()).as_deref(),
            );
        }
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
            })
            .await;

        if result.is_ok() {
            emit_after_compaction_settlement(
                store,
                sid,
                event_sink,
                fixed_context,
                None,
                obs_session,
            )
            .await;
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
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        last_assistant: &AgentMessage,
        current_provider: &str,
        current_model_id: &str,
        overflow_recovery_attempted: bool,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        obs_session: &xylitol_ai_bridge::ObsSessionContext,
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
                    model,
                    event_sink,
                    "overflow",
                    false,
                    context_window,
                    &entries,
                    fixed_context,
                    turn_obs_parent,
                    obs_session,
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
                })
                .await;
            return Ok(OverflowCompactOutcome::FailedOnce);
        }

        self.run_auto_compaction(
            store,
            sid,
            model,
            event_sink,
            "overflow",
            true,
            context_window,
            &entries,
            fixed_context,
            turn_obs_parent,
            obs_session,
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
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
        last_assistant: Option<&AgentMessage>,
        precomputed: Option<&crate::protocol::model::ContextTokenEstimate>,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        obs_session: &xylitol_ai_bridge::ObsSessionContext,
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

        if !should_compact(estimate.tokens, context_window, &self.settings) {
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
            model,
            event_sink,
            &reason,
            false,
            context_window,
            &entries,
            fixed_context,
            turn_obs_parent,
            obs_session,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_auto_compaction(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        reason: &str,
        will_retry: bool,
        context_window: u64,
        entries: &[SessionEntry],
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        obs_session: &xylitol_ai_bridge::ObsSessionContext,
    ) -> Result<bool, CompactionError> {
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
            model,
            &self.settings,
            None,
            context_window,
            fixed_context,
            llm_parent,
            obs_session,
        )
        .await;
        let (ok_result, err_msg, summary, tokens_before) = match &result {
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
        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: ok_result,
                aborted: false,
                reason: end_reason,
                will_retry: will_retry_end,
                error_message: err_msg,
                summary,
                tokens_before,
            })
            .await;

        if result.is_ok() {
            emit_after_compaction_settlement(
                store,
                sid,
                event_sink,
                fixed_context,
                turn_obs_parent,
                obs_session,
            )
            .await;
        }

        match result {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }
}

async fn emit_after_compaction_settlement(
    store: &dyn XySessionStore,
    sid: &str,
    event_sink: &dyn XyEventSink,
    fixed_context: Option<&FixedRequestContext>,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    obs_session: &xylitol_ai_bridge::ObsSessionContext,
) {
    use crate::agent::compaction::settlement::{
        ContextTokenSettlementReason, settle_from_session_entries,
    };
    let Ok(fresh) = store.load_leaf_branch(sid).await else {
        return;
    };
    // c25: AfterCompaction placeholder — estimate of summary row + kept tail +
    // fixed request overhead (system prompt + tools), the next request's size.
    // The stale pre-compact Api anchor is dropped inside the estimator.
    let settled = settle_from_session_entries(
        &fresh,
        &EstimateOpts {
            fixed_context: fixed_context.cloned(),
            obs_parent: turn_obs_parent,
            obs_session: obs_session.clone(),
            ..Default::default()
        },
        ContextTokenSettlementReason::AfterCompaction,
    );
    event_sink
        .emit(&XyEvent::ContextTokenSettlement {
            estimate: settled.estimate,
            reason: settled.reason.as_str().to_string(),
            generation: settled.generation,
        })
        .await;
}

/// Check if compaction should trigger (pi-aligned reserve formula).
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: &CompactionSettings,
) -> bool {
    if !settings.enabled || context_window == 0 {
        return false;
    }
    context_tokens > context_window.saturating_sub(settings.reserve_tokens)
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

    use crate::protocol::error::{XyError, XySessionStoreError};
    use crate::protocol::model::XyToolSchema;
    use crate::protocol::ports::{XyGenerateOptions, XyStream};
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
        });
        let fixed = FixedRequestContext {
            system_prompt: Some("S".repeat(26_000)), // ≈6.5k chars/4 overhead
            tool_schemas: Vec::new(),
        };
        orch.compact(
            &mgr,
            sid,
            model.as_ref(),
            sink.as_ref(),
            None,
            32_768,
            Some(&fixed),
            &Default::default(),
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
                ..
            }
        )));
    }

    /// otel19: prepare early-exit MUST NOT export `agent.compaction`.
    /// `current_thread` so scoped gates / collect stay on the worker that entered them.
    #[tokio::test(flavor = "current_thread")]
    async fn prepare_fail_exports_no_compaction_span() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        let orch = CompactionOrchestrator::new(CompactionSettings::default());
        let err = orch
            .compact(
                &EmptyLeafStore,
                "sid",
                &PanicModel,
                &NoopSink,
                None,
                0,
                None,
                &Default::default(),
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
        assert!(!should_compact(100_000, 200_000, &s));
    }

    #[test]
    fn should_compact_window_zero() {
        let s = CompactionSettings::default();
        assert!(!should_compact(100_000, 0, &s));
    }

    #[test]
    fn should_compact_not_exceeded() {
        let s = CompactionSettings::default();
        assert!(!should_compact(50_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        assert!(should_compact(200_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exact_boundary_not_trigger() {
        let s = CompactionSettings::default();
        assert!(!should_compact(183_616, 200_000, &s));
    }

    #[test]
    fn should_compact_one_over_boundary() {
        let s = CompactionSettings::default();
        assert!(should_compact(183_617, 200_000, &s));
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
}
