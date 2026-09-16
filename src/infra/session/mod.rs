//! Session persistence — manifest-backed JSONL segment storage.
//!
//! Key behaviors:
//! - JSONL segments (one JSON object per line, append-only active segment)
//! - Stored in ~/.xylitol/sessions/`<id>/`
//! - Entry types: message, compaction, branch_summary, model_change, thinking_level_change, custom
//! - Session tree via parentSession header field
//! - Version gate: current manifests load; the supported legacy file is migrated once
//! - Atomic appends with file locking

pub(crate) mod manager;
pub mod types;

pub use manager::SessionManager;
// Test-support re-export (in-crate tests import via this facade).
#[cfg(test)]
pub use types::*;
