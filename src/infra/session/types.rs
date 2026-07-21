//! Session entry types — shared vocabulary relocated to `domain::session_types`.
//!
//! This file remains as a thin re-export so `crate::infra::session::types`
//! references (session manager) keep resolving during the import migration.
//! New code should import from `crate::protocol::session`.
//!
//! The `SessionBackend` enum is an infra-only storage implementation detail
//! (used solely by `manager.rs`) and is defined here.

use std::path::PathBuf;

pub use crate::protocol::session::{
    BashExecutionEntry, BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry,
    EntryBase, LabelEntry, MessageEntry, ModelChangeEntry, SESSION_VERSION, SessionContext,
    SessionEntry, SessionHeader, SessionInfoEntry, SessionTreeNode, ThinkingLevelChangeEntry,
};

/// Storage backend for a session.
#[derive(Debug, Clone)]
pub enum SessionBackend {
    /// Persisted to a JSONL file in a directory.
    Persisted { sessions_dir: PathBuf },
    /// In-memory only (no disk writes).
    InMemory { entries: Vec<SessionEntry> },
}
