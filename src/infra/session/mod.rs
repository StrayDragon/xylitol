//! Session persistence — JSONL file-based session storage.
//!
//! Key behaviors:
//! - JSONL format (one JSON object per line, append-only)
//! - Stored in ~/.xylitol/sessions/`<id>`.jsonl
//! - Entry types: message, compaction, branch_summary, model_change, thinking_level_change, custom
//! - Session tree via parentSession header field
//! - Version gate: only current SESSION_VERSION loads; older files are refused (no migrate)
//! - Atomic appends with file locking

pub(crate) mod manager;
pub mod types;

pub use manager::SessionManager;
pub use types::*;
