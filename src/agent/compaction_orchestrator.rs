//! CompactionOrchestrator — threshold checking and compaction triggering.
//!
//! Extracted from [`AgentSession`](super::session::AgentSession) to isolate
//! compaction orchestration into a focused component.

use crate::agent::compaction::{CompactionSettings, compact_session};
use crate::core::traits::XyModel;
use crate::infra::event::lifecycle::AgentLifecycleEvent;
use crate::infra::event::EventBus;
use crate::infra::session::manager::SessionManager;

/// Orchestrates session compaction — threshold checks and execution.
///
/// Used by [`AgentSession`](super::session::AgentSession) as a composed helper.
pub struct CompactionOrchestrator {
    /// Context window threshold for compaction (0.0–1.0).
    threshold: f64,
}

impl CompactionOrchestrator {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Manually compact the current session.
    pub async fn compact(
        &self,
        session_manager: &SessionManager,
        sid: &str,
        model: &dyn XyModel,
        event_bus: &EventBus,
    ) -> Result<(), String> {
        event_bus.emit_lifecycle(&AgentLifecycleEvent::CompactionStart {
            reason: "manual".to_string(),
        });

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        let result = compact_session(session_manager, sid, model, &settings)
            .await
            .map_err(|e| format!("compaction failed: {e}"));

        event_bus.emit_lifecycle(&AgentLifecycleEvent::CompactionEnd {
            result: result.as_ref().ok().map(|_| "ok".to_string()),
            aborted: false,
        });

        result?;
        Ok(())
    }

    /// Check threshold and auto-compact if needed.
    /// Returns true if compaction was performed.
    pub async fn maybe_auto_compact(
        &self,
        session_manager: &SessionManager,
        sid: &str,
        model: &dyn XyModel,
        event_bus: &EventBus,
        context_window: u64,
    ) -> Result<bool, String> {
        let session_ctx = session_manager.build_session_context(sid).await?;
        let token_estimate: u64 = session_ctx
            .messages
            .iter()
            .map(|m| (m.to_string().len() as u64).div_ceil(4))
            .sum();

        if !should_compact(token_estimate, context_window, self.threshold) {
            return Ok(false);
        }

        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 16384,
            keep_recent_tokens: 20000,
        };

        event_bus.emit_lifecycle(&AgentLifecycleEvent::CompactionStart {
            reason: format!(
                "auto: {:.1}% of {}k window",
                (token_estimate as f64 / context_window as f64) * 100.0,
                context_window / 1000,
            ),
        });

        let result = compact_session(session_manager, sid, model, &settings)
            .await
            .map_err(|e| format!("auto-compaction: {e}"));

        event_bus.emit_lifecycle(&AgentLifecycleEvent::CompactionEnd {
            result: result.as_ref().ok().map(|_| "ok".to_string()),
            aborted: false,
        });

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
