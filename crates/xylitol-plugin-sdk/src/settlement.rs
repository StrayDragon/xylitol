use std::sync::atomic::{AtomicU64, Ordering};

use strum::IntoStaticStr;

use crate::model::ContextTokenEstimate;
use crate::session::SessionEntry;

use crate::token_estimator::{
    EstimateOpts, emit_token_estimate_obs, estimate_from_session_entries,
};

static SETTLEMENT_GENERATION: AtomicU64 = AtomicU64::new(1);

pub fn next_settlement_generation() -> u64 {
    SETTLEMENT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ContextTokenSettlementReason {
    TurnSettled,
    AfterCompaction,
    MidTurnUsage,
    LeafChanged,
    ManualRefresh,
}

impl ContextTokenSettlementReason {
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    pub fn emit_obs(self) -> bool {
        matches!(self, Self::TurnSettled | Self::AfterCompaction)
    }
}

#[derive(Debug, Clone)]
pub struct ContextTokenSettlement {
    pub estimate: ContextTokenEstimate,
    pub reason: ContextTokenSettlementReason,
    pub generation: u64,
}

pub fn estimate_quiet(entries: &[SessionEntry], opts: &EstimateOpts) -> ContextTokenEstimate {
    let mut quiet = opts.clone();
    quiet.emit_obs = false;
    estimate_from_session_entries(entries, &quiet)
}

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
