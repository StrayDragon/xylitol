#![allow(dead_code)]
use chrono::TimeDelta;

/// Strategy for pruning (garbage-collecting) snapshots.
#[derive(Debug, Clone)]
pub(crate) enum PruneStrategy {
    /// Keep at most N latest snapshots; remove the rest.
    KeepLatest(usize),
    /// Remove snapshots older than the given duration.
    OlderThan(TimeDelta),
    /// Remove all snapshots with a specific tag.
    Tagged(String),
}
