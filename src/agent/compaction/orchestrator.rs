//! CompactionOrchestrator — threshold checking and compaction triggering.
//!
//! Extracted from [`AgentCapabilities`](crate::agent::session::AgentCapabilities) to isolate
//! compaction orchestration into a focused component.

use crate::agent::compaction::token_estimator::{EstimateOpts, estimate_from_session_entries};
use crate::agent::compaction::{CompactionSettings, compact_session};
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore};

/// Orchestrates session compaction — threshold checks and execution.
///
/// Used by [`AgentCapabilities`](crate::agent::session::AgentCapabilities) as a composed helper.
pub struct CompactionOrchestrator {
    /// Context window threshold for compaction (0.0–1.0).
    threshold: f64,
    /// Compaction tuning (reserve / keep-recent tokens, master toggle).
    settings: CompactionSettings,
}

impl CompactionOrchestrator {
    pub fn new(threshold: f64, settings: CompactionSettings) -> Self {
        Self {
            threshold,
            settings,
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Compaction tuning in use (reserve / keep-recent tokens, master toggle).
    pub fn settings(&self) -> &CompactionSettings {
        &self.settings
    }

    /// Manually compact the current session.
    pub async fn compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
    ) -> Result<(), String> {
        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: "manual".to_string(),
            })
            .await;

        let result = compact_session(store, sid, model, &self.settings)
            .await
            .map_err(|e| format!("compaction failed: {e}"));

        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: result.as_ref().ok().map(|_| "ok".to_string()),
                aborted: false,
            })
            .await;

        result?;
        Ok(())
    }

    /// Check threshold and auto-compact if needed.
    /// Returns true if compaction was performed.
    ///
    /// Threshold uses the same estimate path as the TUI footer (`estimate_from_session_entries`
    /// / paa1), not a separate `len/4` sum (c1420 / c2 / c16).
    pub async fn maybe_auto_compact(
        &self,
        store: &dyn XySessionStore,
        sid: &str,
        model: &dyn XyModel,
        event_sink: &dyn XyEventSink,
        context_window: u64,
        estimate_opts: &EstimateOpts,
    ) -> Result<bool, String> {
        let entries = store.load_entries(sid).await?;
        let token_estimate = estimate_from_session_entries(&entries, estimate_opts).tokens;

        if !should_compact(token_estimate, context_window, self.threshold) {
            return Ok(false);
        }

        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: format!(
                    "auto: {:.1}% of {}k window",
                    (token_estimate as f64 / context_window as f64) * 100.0,
                    context_window / 1000,
                ),
            })
            .await;

        let result = compact_session(store, sid, model, &self.settings)
            .await
            .map_err(|e| format!("auto-compaction: {e}"));

        event_sink
            .emit(&XyEvent::CompactionEnd {
                result: result.as_ref().ok().map(|_| "ok".to_string()),
                aborted: false,
            })
            .await;

        result?;
        Ok(true)
    }
}

/// Check if compaction should be triggered based on token usage.
pub fn should_compact(token_estimate: u64, context_window: u64, threshold: f64) -> bool {
    if context_window == 0 {
        return false;
    }
    let usage_ratio = token_estimate as f64 / context_window as f64;
    usage_ratio >= threshold
}
