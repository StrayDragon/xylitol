//! SessionManager — manifest-backed JSONL segment storage.
//!
//! Handles create, append, load, list, exists, tree navigation,
//! and build_session_context for sessions (latest SESSION_VERSION only).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::types::*;
use crate::utils::{lock_rwlock_read, lock_rwlock_write};

mod context;
mod load;
mod persist;
mod seal;
mod segments;
mod store;
mod tree;

#[cfg(test)]
mod tests;

/// Manages session persistence using JSONL files or in-memory storage.
///
/// [`Clone`] shares interior stores via [`Arc`] so handles remain coherent after
/// deferred (pre-flush) creates — deep-copying pending would drop sessions on
/// `mgr.clone()` + mutate patterns used by tests and thin wrappers.
#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions_dir: PathBuf,
    /// Storage backend.
    backend: SessionBackend,
    /// Per-session leaf node tracking (in-memory).
    /// session_id -> current leaf entry id (None = root).
    leaf_ids: Arc<RwLock<HashMap<String, Option<String>>>>,
    /// Active session tracking (BDD @executable session-navigation surface).
    active_session: Arc<RwLock<Option<String>>>,
    /// In-memory entry storage (used when backend is InMemory).
    in_memory_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
    /// Pending entries for persisted sessions not yet flushed to disk.
    pending_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            backend: SessionBackend::Persisted,
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SessionManager {
    /// Create a new SessionManager with the given sessions directory.
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir: sessions_dir.clone(),
            backend: SessionBackend::Persisted,
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create an in-memory SessionManager (no disk writes).
    /// All entries are stored in a Vec, suitable for ephemeral sessions.
    pub fn in_memory() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            backend: SessionBackend::InMemory,
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Default sessions directory: ~/.xylitol/sessions/
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".xylitol")
            .join("sessions")
    }

    // ── Leaf tracking ───────────────────────────────────────────

    fn set_leaf(&self, session_id: &str, entry_id: Option<String>) {
        lock_rwlock_write(&self.leaf_ids).insert(session_id.to_string(), entry_id);
    }

    fn get_leaf(&self, session_id: &str) -> Option<String> {
        lock_rwlock_read(&self.leaf_ids)
            .get(session_id)
            .cloned()
            .unwrap_or(None)
    }

    fn session_file_exists(&self, session_id: &str) -> bool {
        matches!(&self.backend, SessionBackend::Persisted)
            && (self.manifest_path(session_id).exists()
                || self.legacy_session_path(session_id).exists())
    }

    // ── CRUD ────────────────────────────────────────────────────

    /// Get the active JSONL path for a persisted session.
    pub(super) fn session_path(&self, id: &str) -> PathBuf {
        match &self.backend {
            SessionBackend::Persisted => self.current_active_path(id),
            SessionBackend::InMemory => PathBuf::from("/dev/null"),
        }
    }

    /// Get the session file, if persisted.
    ///
    /// BDD @executable contract (`会话 JSONL 文件存在` steps) + driver/unit tests;
    /// no product caller today.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn get_session_file(&self, id: &str) -> Option<PathBuf> {
        match &self.backend {
            SessionBackend::Persisted => Some(self.session_path(id)),
            SessionBackend::InMemory => None,
        }
    }

    /// Check if a session exists.
    pub fn exists(&self, id: &str) -> bool {
        match &self.backend {
            SessionBackend::Persisted => {
                self.session_file_exists(id)
                    || lock_rwlock_read(&self.pending_store).contains_key(id)
            }
            SessionBackend::InMemory => lock_rwlock_read(&self.in_memory_store).contains_key(id),
        }
    }
}
