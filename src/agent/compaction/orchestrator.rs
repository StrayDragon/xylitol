//! CompactionOrchestrator — reserve-formula checks and compaction triggering.
//!
//! Extracted from [`AgentCapabilities`](crate::agent::session::AgentCapabilities) to isolate
//! compaction orchestration into a focused component.
//!
//! Trigger formula matches pi `shouldCompact` (coding-agent compaction.ts):
//! `enabled && contextTokens > contextWindow - reserveTokens`.

use crate::agent::compaction::token_estimator::{EstimateOpts, estimate_from_session_entries};
use crate::agent::compaction::{CompactionSettings, compact_session};
use crate::protocol::lifecycle::XyEvent;
use crate::protocol::ports::{XyEventSink, XyModel, XySessionStore};

/// Orchestrates session compaction — threshold checks and execution.
///
/// Used by [`AgentCapabilities`](crate::agent::session::AgentCapabilities) as a composed helper.
pub struct CompactionOrchestrator {
    /// Compaction tuning (reserve / keep-recent tokens, master toggle).
    settings: CompactionSettings,
}

impl CompactionOrchestrator {
    pub fn new(settings: CompactionSettings) -> Self {
        Self { settings }
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

    /// Check reserve formula and auto-compact if needed.
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

        if !should_compact(token_estimate, context_window, &self.settings) {
            return Ok(false);
        }

        event_sink
            .emit(&XyEvent::CompactionStart {
                reason: format!(
                    "auto: {} tokens over reserve of {}k window",
                    token_estimate,
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

/// Check if compaction should trigger (pi-aligned reserve formula).
///
/// `enabled && context_window > 0 && tokens > window.saturating_sub(reserve_tokens)`.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        // threshold = 200_000 - 16384 = 183_616; 50_000 is under
        assert!(!should_compact(50_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exceeded() {
        let s = CompactionSettings {
            reserve_tokens: 1000,
            ..Default::default()
        };
        // threshold = 200_000 - 1000 = 199_000
        assert!(should_compact(200_000, 200_000, &s));
    }

    #[test]
    fn should_compact_exact_boundary_not_trigger() {
        let s = CompactionSettings::default();
        // threshold = 200_000 - 16384 = 183_616; equal is NOT >
        assert!(!should_compact(183_616, 200_000, &s));
    }

    #[test]
    fn should_compact_one_over_boundary() {
        let s = CompactionSettings::default();
        assert!(should_compact(183_617, 200_000, &s));
    }

    #[test]
    fn should_compact_pi_examples() {
        // From pi compaction.test.ts
        let s = CompactionSettings {
            enabled: true,
            reserve_tokens: 10_000,
            keep_recent_tokens: 20_000,
        };
        assert!(should_compact(95_000, 100_000, &s));
        assert!(!should_compact(89_000, 100_000, &s));
        assert!(!should_compact(
            95_000,
            100_000,
            &CompactionSettings {
                enabled: false,
                ..s
            }
        ));
    }
}
