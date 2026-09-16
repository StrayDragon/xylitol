//! Session entry type re-exports plus infra-only storage backend.
//!
//! Prefer `crate::protocol::session` for entry types in new code.
//! [`SessionBackend`] is an infra storage detail used by `manager.rs`.

pub use crate::protocol::session::{
    BranchSummaryEntry, CompactionEntry, CustomEntry, CustomMessageEntry, EntryBase, LabelEntry,
    MessageEntry, ModelChangeEntry, SESSION_VERSION, SessionContext, SessionEntry, SessionHeader,
    SessionInfoEntry, ThinkingLevelChangeEntry,
};

/// Storage backend for a session.
#[derive(Debug, Clone)]
pub enum SessionBackend {
    /// Persisted to a manifest-backed session directory.
    Persisted,
    /// In-memory only (no disk writes).
    InMemory,
}
