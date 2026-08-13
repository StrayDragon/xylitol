//! CompactionOrchestrator — reserve-formula checks and compaction triggering.
//!
//! Trigger formula matches pi `shouldCompact` (coding-agent compaction.ts):
//! `enabled && contextTokens > contextWindow - reserveTokens`.
//!
//! Manual `compact` = force (pi `AgentSession.compact`); auto threshold = Case2;
//! overflow compact-and-retry = Case1 (c1660).

use crate::agent::compaction::obs::AgentCompactionSpan;
use crate::agent::compaction::overflow::{assistant_same_model, is_context_overflow_assistant};
use crate::agent::compaction::token_estimator::EstimateOpts;
use crate::agent::compaction::{CompactionSettings, compact_session, prepare_compaction};
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
    pub async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
    ) -> Result<(), String> {
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: "manual".to_string(),
            })
            .await;

        let entries = store.load_leaf_branch(sid).await?;
        if let Some(err) = prepare_compaction(&entries, &self.settings).err() {
            event_sink
                .emit(&XyEvent::CompactionEnd {
                    result: None,
                    aborted: false,
                    reason: "manual".into(),
                    will_retry: false,
                    error_message: Some(err.clone()),
                    summary: None,
                    tokens_before: None,
                })
                .await;
            return Err(err);
        }

        let obs = AgentCompactionSpan::start("manual", None);

        let mut force_settings = self.settings.clone();
        force_settings.enabled = true;

        let llm_parent = obs.as_ref().and_then(|s| s.span_context());
        let result = compact_session(
            store,
            sid,
            model,
            &force_settings,
            instructions.as_deref(),
            llm_parent,
        )
        .await
        .map_err(|e| format!("compaction failed: {e}"));

        if let Some(obs) = obs {
            obs.finish(false, false, result.as_ref().err().map(String::as_str));
        }
        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: result.as_ref().ok().map(|_| "ok".to_string()),
                aborted: false,
                reason: "manual".into(),
                will_retry: false,
                error_message: result.as_ref().err().cloned(),
                summary: result.as_ref().ok().map(|e| e.summary.clone()),
                tokens_before: result.as_ref().ok().map(|e| e.tokens_before),
            })
            .await;

        if result.is_ok() {
            emit_after_compaction_settlement(store, sid, event_sink, None).await;
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
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    ) -> Result<OverflowCompactOutcome, String> {
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
                    &entries,
                    turn_obs_parent,
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
            &entries,
            turn_obs_parent,
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
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    ) -> Result<bool, String> {
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
                estimate_quiet(&entries, estimate_opts)
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
            &entries,
            turn_obs_parent,
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
        entries: &[SessionEntry],
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
    ) -> Result<bool, String> {
        if prepare_compaction(entries, &self.settings).is_err() {
            return Ok(false);
        }

        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: reason.to_string(),
            })
            .await;
        let obs = AgentCompactionSpan::start(reason, turn_obs_parent);

        let llm_parent = obs.as_ref().and_then(|s| s.span_context());
        let result = compact_session(store, sid, model, &self.settings, None, llm_parent).await;
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
            emit_after_compaction_settlement(store, sid, event_sink, turn_obs_parent).await;
        }

        match result {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("auto-compaction: {e}")),
        }
    }
}

async fn emit_after_compaction_settlement(
    store: &dyn XySessionStore,
    sid: &str,
    event_sink: &dyn XyEventSink,
    turn_obs_parent: Option<fastrace::prelude::SpanContext>,
) {
    use crate::agent::compaction::settlement::{
        ContextTokenSettlementReason, settle_from_session_entries,
    };
    let Ok(fresh) = store.load_leaf_branch(sid).await else {
        return;
    };
    let settled = settle_from_session_entries(
        &fresh,
        &EstimateOpts {
            obs_parent: turn_obs_parent,
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
        SessionEntry::Compaction(c) => parse_rfc3339_ms(&c.base.timestamp),
        _ => None,
    })
}

fn parse_rfc3339_ms(ts: &str) -> Option<u64> {
    time::OffsetDateTime::parse(ts, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|dt| (dt.unix_timestamp() * 1000 + dt.millisecond() as i64).max(0) as u64)
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
            if let Some(entry_ms) = entry.base().and_then(|b| parse_rfc3339_ms(&b.timestamp)) {
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

    use crate::protocol::error::XyError;
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
        async fn load_entries(&self, _: &str) -> Result<Vec<SessionEntry>, String> {
            Ok(Vec::new())
        }
        async fn append_session_entry(&self, _: &str, _: &SessionEntry) -> Result<(), String> {
            unreachable!("prepare-fail path must not append")
        }
        async fn build_session_context(&self, _: &str) -> Result<SessionContext, String> {
            unreachable!("prepare-fail path must not build context")
        }
        async fn create(&self, _: &str, _: Option<&str>, _: Option<&str>) -> Result<(), String> {
            Ok(())
        }
        async fn fork(&self, _: &str, _: &str, _: &str, _: ForkPosition) -> Result<(), String> {
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

    /// otel19: prepare early-exit MUST NOT export `agent.compaction`.
    /// `current_thread` so scoped gates / collect stay on the worker that entered them.
    #[tokio::test(flavor = "current_thread")]
    async fn prepare_fail_exports_no_compaction_span() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        let orch = CompactionOrchestrator::new(CompactionSettings::default());
        let err = orch
            .compact(&EmptyLeafStore, "sid", &PanicModel, &NoopSink, None)
            .await
            .expect_err("empty session must fail prepare");
        assert!(
            err.contains("Nothing to compact"),
            "unexpected prepare error: {err}"
        );

        fastrace::flush();

        let spans = collect.records();
        assert!(
            spans.iter().all(|s| s.name != "agent.compaction"),
            "prepare early-exit must not export agent.compaction; got: {:?}",
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
