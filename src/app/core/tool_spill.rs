//! Process-wide tool spill directory (c2080).
//!
//! Composition roots / surfaces call [`install`] so `OutputAccumulator` writes
//! beside the session instead of a global system tmp. Surfaces MUST NOT import
//! `infra::*` for this — use this seam.

use std::path::PathBuf;

/// Install (or clear) the process spill root for tool hard-truncation.
pub fn install(dir: Option<PathBuf>) {
    crate::infra::tools::accumulator::set_process_spill_dir(dir);
}

/// Current process spill root (tests / diagnostics).
pub fn current() -> Option<PathBuf> {
    crate::infra::tools::accumulator::process_spill_dir()
}
