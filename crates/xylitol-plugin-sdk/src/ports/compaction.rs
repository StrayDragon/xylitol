//! Compaction orchestration port — runtime calls this; the official plugin implements it.
#![allow(clippy::too_many_arguments)]

use crate::compaction_binding::CompactionSummaryBinding;
use crate::compaction_error::CompactionError;
use crate::compaction_settings::CompactionSettings;
use crate::message::AgentMessage;
use crate::model::ContextTokenEstimate;
use crate::token_estimator::{EstimateOpts, FixedRequestContext};
use crate::{XyEventSink, XySessionStore};

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

/// Trigger formula matches pi `shouldCompact`.
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
        > crate::compaction_settings::effective_trigger_threshold(
            context_window,
            settings,
            post_compact_floor,
        )
}

/// Compaction orchestration — threshold checks and execution.
#[async_trait::async_trait]
pub trait XyCompaction: Send + Sync {
    fn settings(&self) -> &CompactionSettings;

    #[allow(clippy::too_many_arguments)]
    async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        instructions: Option<String>,
        context_window: u64,
        fixed_context: Option<&FixedRequestContext>,
        fallback_notice_emitted: &mut bool,
    ) -> Result<(), CompactionError>;

    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<OverflowCompactOutcome, CompactionError>;

    #[allow(clippy::too_many_arguments)]
    async fn maybe_auto_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        summary: &CompactionSummaryBinding,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
        last_assistant: Option<&AgentMessage>,
        precomputed: Option<&ContextTokenEstimate>,
        fixed_context: Option<&FixedRequestContext>,
        turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        fallback_notice_emitted: &mut bool,
        floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, CompactionError>;
}

/// No-op compaction used when composition has not injected a plugin.
pub struct NoopCompaction {
    settings: CompactionSettings,
}

impl NoopCompaction {
    pub fn new(settings: CompactionSettings) -> Self {
        Self { settings }
    }
}

#[async_trait::async_trait]
impl XyCompaction for NoopCompaction {
    fn settings(&self) -> &CompactionSettings {
        &self.settings
    }

    async fn compact(
        &self,
        _store: &dyn XySessionStore,
        _sid: &str,
        _summary: &CompactionSummaryBinding,
        _event_sink: &dyn XyEventSink,
        _instructions: Option<String>,
        _context_window: u64,
        _fixed_context: Option<&FixedRequestContext>,
        _fallback_notice_emitted: &mut bool,
    ) -> Result<(), CompactionError> {
        Ok(())
    }

    async fn maybe_overflow_compact(
        &self,
        _store: &dyn XySessionStore,
        _sid: &str,
        _summary: &CompactionSummaryBinding,
        _event_sink: &dyn XyEventSink,
        _context_window: u64,
        _last_assistant: &AgentMessage,
        _current_provider: &str,
        _current_model_id: &str,
        _overflow_recovery_attempted: bool,
        _fixed_context: Option<&FixedRequestContext>,
        _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        _fallback_notice_emitted: &mut bool,
        _floor_notice_emitted: Option<&mut bool>,
    ) -> Result<OverflowCompactOutcome, CompactionError> {
        Ok(OverflowCompactOutcome::Skipped)
    }

    async fn maybe_auto_compact(
        &self,
        _store: &dyn XySessionStore,
        _sid: &str,
        _summary: &CompactionSummaryBinding,
        _event_sink: &dyn XyEventSink,
        _context_window: u64,
        _estimate_opts: &EstimateOpts,
        _last_assistant: Option<&AgentMessage>,
        _precomputed: Option<&ContextTokenEstimate>,
        _fixed_context: Option<&FixedRequestContext>,
        _turn_obs_parent: Option<fastrace::prelude::SpanContext>,
        _fallback_notice_emitted: &mut bool,
        _floor_notice_emitted: Option<&mut bool>,
    ) -> Result<bool, CompactionError> {
        Ok(false)
    }
}

/// Factory that turns settings into a live [`XyCompaction`] (composition injects the plugin).
pub type XyCompactionFactory =
    std::sync::Arc<dyn Fn(CompactionSettings) -> std::sync::Arc<dyn XyCompaction> + Send + Sync>;

/// Default factory: [`NoopCompaction`].
pub fn noop_compaction_factory() -> XyCompactionFactory {
    std::sync::Arc::new(|settings| {
        std::sync::Arc::new(NoopCompaction::new(settings)) as std::sync::Arc<dyn XyCompaction>
    })
}
