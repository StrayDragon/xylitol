//! Context token **settlement** — one estimate snapshot shared by compact + footer (c1860).
//!
//! ## Extension (maintainers)
//!
//! Add a new [`ContextTokenSettlementReason`] variant when a product surface needs to
//! invalidate / recompute context tokens (e.g. future cache-policy or provider-hint
//! stale signals). Consumers (footer, compact, future hint UI) MUST subscribe to the
//! settlement snapshot / [`crate::protocol::lifecycle::XyEvent::ContextTokenSettlement`]
//! and MUST NOT each call [`super::token_estimator::estimate_from_session_entries`]
//! with OTel emit. Do **not** pre-build CacheHint UI stubs here.

use std::sync::atomic::{AtomicU64, Ordering};

use strum::IntoStaticStr;

use crate::protocol::model::ContextTokenEstimate;
use crate::protocol::session::SessionEntry;

use super::token_estimator::{
    EstimateOpts, emit_token_estimate_obs, estimate_from_session_entries,
};

static SETTLEMENT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Allocate the next settlement generation id (monotonic process-local).
pub fn next_settlement_generation() -> u64 {
    SETTLEMENT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// Why a context-token settlement was produced.
///
/// Future: `CachePolicyChanged` / `ProviderHintStale` — only add the variant +
/// invalidator mapping; keep one settle → many consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ContextTokenSettlementReason {
    /// ReAct turn settled (threshold / overflow pre-check path).
    TurnSettled,
    /// Leaf changed after a successful compaction.
    AfterCompaction,
    /// Mid-turn Api usage refresh (footer throttle).
    MidTurnUsage,
    /// Session tree travel / resume / import.
    LeafChanged,
    /// Explicit refresh (slash / tests).
    ManualRefresh,
}

impl ContextTokenSettlementReason {
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    /// Whether this settlement should export a `token.estimate` span when the
    /// provider-trace gate is on.
    pub fn emit_obs(self) -> bool {
        matches!(self, Self::TurnSettled | Self::AfterCompaction)
    }
}

/// One settled estimate + reason (generation is assigned by the surface/driver).
#[derive(Debug, Clone)]
pub struct ContextTokenSettlement {
    pub estimate: ContextTokenEstimate,
    pub reason: ContextTokenSettlementReason,
    /// Monotonic id for “already applied this run” checks (stream-close skip).
    pub generation: u64,
}

/// Estimate from session entries without OTel (`emit_obs = false`).
pub fn estimate_quiet(entries: &[SessionEntry], opts: &EstimateOpts) -> ContextTokenEstimate {
    let mut quiet = opts.clone();
    quiet.emit_obs = false;
    estimate_from_session_entries(entries, &quiet)
}

/// Settle once: quiet compute, then optionally emit `token.estimate` for the reason.
pub fn settle_from_session_entries(
    entries: &[SessionEntry],
    opts: &EstimateOpts,
    reason: ContextTokenSettlementReason,
) -> ContextTokenSettlement {
    let generation = next_settlement_generation();
    let estimate = estimate_quiet(entries, opts);
    if reason.emit_obs() {
        emit_token_estimate_obs(&estimate, opts);
    }
    ContextTokenSettlement {
        estimate,
        reason,
        generation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use fastrace::prelude::*;
    use xylitol_ai_bridge::provider::trace::{ObsGateScope, ObsGateState, SpanCollectScope};

    use crate::protocol::message::AgentMessage;
    use crate::protocol::session::{EntryBase, MessageEntry, SessionEntry};

    fn sample_entries() -> Vec<SessionEntry> {
        vec![SessionEntry::Message(MessageEntry {
            base: EntryBase {
                entry_type: "message".into(),
                id: "u1".into(),
                parent_id: None,
                timestamp: String::new(),
            },
            message: serde_json::to_value(AgentMessage::user("hi")).unwrap(),
        })]
    }

    #[test]
    fn turn_settled_emits_one_token_estimate_under_turn() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();

        {
            let turn = Span::root("agent.turn", SpanContext::random());
            let turn_ctx = SpanContext::from_span(&turn);
            let _s = settle_from_session_entries(
                &sample_entries(),
                &EstimateOpts {
                    obs_parent: turn_ctx,
                    ..Default::default()
                },
                ContextTokenSettlementReason::TurnSettled,
            );
            // Mid-path quiet must not add a second span.
            let _ = estimate_quiet(&sample_entries(), &EstimateOpts::default());
            drop(turn);
        }
        fastrace::flush();

        let spans = collect.records();
        let estimates: Vec<_> = spans
            .iter()
            .filter(|s| s.name == "token.estimate")
            .collect();
        assert_eq!(
            estimates.len(),
            1,
            "TurnSettled must emit exactly one token.estimate, got {estimates:?}"
        );
        let turn = spans.iter().find(|s| s.name == "agent.turn").expect("turn");
        assert_eq!(estimates[0].trace_id, turn.trace_id);
        assert_eq!(estimates[0].parent_id, turn.span_id);
    }

    #[test]
    fn mid_turn_usage_does_not_emit_obs() {
        let _g = ObsGateScope::enter(ObsGateState::active_none_io());
        let collect = SpanCollectScope::enter();
        {
            let _s = settle_from_session_entries(
                &sample_entries(),
                &EstimateOpts::default(),
                ContextTokenSettlementReason::MidTurnUsage,
            );
        }
        fastrace::flush();
        let spans = collect.records();
        assert!(
            !spans.iter().any(|s| s.name == "token.estimate"),
            "MidTurnUsage must not emit token.estimate"
        );
    }
}
