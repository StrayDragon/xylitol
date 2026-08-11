//! Session entry type re-exports plus infra-only storage backend.
//!
//! Prefer `crate::protocol::session` for entry types in new code.
//! [`SessionBackend`] is an infra storage detail used by `manager.rs`.

use std::path::PathBuf;

pub use crate::protocol::session::{
    BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry, EntryBase, LabelEntry,
    MessageEntry, ModelChangeEntry, SESSION_VERSION, SessionContext, SessionEntry, SessionHeader,
    SessionInfoEntry, SessionTreeNode, ThinkingLevelChangeEntry,
};

/// Storage backend for a session.
#[derive(Debug, Clone)]
pub enum SessionBackend {
    /// Persisted to a JSONL file in a directory.
    Persisted { sessions_dir: PathBuf },
    /// In-memory only (no disk writes).
    InMemory { entries: Vec<SessionEntry> },
}
