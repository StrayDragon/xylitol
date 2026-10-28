use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use super::types::*;
use crate::sync::{lock_rwlock_read, lock_rwlock_write};

mod context;
mod load;
mod persist;
mod seal;
mod segments;
mod store;
mod tree;
mod write_lock;

pub use write_lock::SESSION_WRITE_LOCK_FILENAME;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions_dir: PathBuf,
    backend: SessionBackend,
    leaf_ids: Arc<RwLock<HashMap<String, Option<String>>>>,
    active_session: Arc<RwLock<Option<String>>>,
    in_memory_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
    pending_store: Arc<RwLock<HashMap<String, Vec<SessionEntry>>>>,
    write_locks: Arc<Mutex<HashMap<String, write_lock::SessionWriteLock>>>,
    #[cfg(test)]
    segment_reads: Arc<std::sync::atomic::AtomicUsize>,
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
            write_locks: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(test)]
            segment_reads: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir: sessions_dir.clone(),
            backend: SessionBackend::Persisted,
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
            write_locks: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(test)]
            segment_reads: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            sessions_dir: PathBuf::from("."),
            backend: SessionBackend::InMemory,
            leaf_ids: Arc::new(RwLock::new(HashMap::new())),
            active_session: Arc::new(RwLock::new(None)),
            in_memory_store: Arc::new(RwLock::new(HashMap::new())),
            pending_store: Arc::new(RwLock::new(HashMap::new())),
            write_locks: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(test)]
            segment_reads: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".xylitol")
            .join("sessions")
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<String>) {
        lock_rwlock_write(&self.leaf_ids).insert(session_id.to_string(), entry_id);
    }

    fn get_leaf(&self, session_id: &str) -> Option<String> {
        lock_rwlock_read(&self.leaf_ids)
            .get(session_id)
            .cloned()
            .unwrap_or(None)
    }

    #[cfg(test)]
    pub(super) fn reset_segment_reads(&self) {
        self.segment_reads
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(super) fn segment_reads(&self) -> usize {
        self.segment_reads.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn session_file_exists(&self, session_id: &str) -> bool {
        matches!(&self.backend, SessionBackend::Persisted)
            && (self.manifest_path(session_id).exists()
                || self.legacy_session_path(session_id).exists())
    }

    pub(super) fn session_path(&self, id: &str) -> PathBuf {
        match &self.backend {
            SessionBackend::Persisted => self.current_active_path(id),
            SessionBackend::InMemory => PathBuf::from("/dev/null"),
        }
    }

    #[allow(dead_code)]
    pub fn get_session_file(&self, id: &str) -> Option<PathBuf> {
        match &self.backend {
            SessionBackend::Persisted => Some(self.session_path(id)),
            SessionBackend::InMemory => None,
        }
    }

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
