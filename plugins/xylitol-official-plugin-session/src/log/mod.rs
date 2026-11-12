mod append;
mod layout;
mod lock;
mod maintain;
mod page;
mod project;
mod store;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

use crate::sync::{lock_mutex, lock_rwlock_read, lock_rwlock_write};
use crate::types::SessionEntry;
use layout::{CachedManifest, Layout};
pub use lock::SESSION_WRITE_LOCK_FILENAME;
use lock::SessionWriteLock;
pub(crate) use store::generate_branch_summary;
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::SessionProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoreMode {
    Persisted,
    InMemory,
}

#[derive(Default)]
pub(crate) struct MemorySession {
    pub(crate) entries: Vec<SessionEntry>,
    pub(crate) edges: Vec<xylitol_plugin_sdk::session::EdgeRecord>,
    pub(crate) parent: Option<String>,
}

pub struct LogSessionStore {
    pub(crate) layout: Layout,
    pub(crate) mode: StoreMode,
    pub(crate) locks: Mutex<HashMap<String, SessionWriteLock>>,
    pub(crate) leaf_ids: RwLock<HashMap<String, Option<String>>>,
    pub(crate) memory: RwLock<HashMap<String, MemorySession>>,
    pub(crate) manifest_cache: RwLock<HashMap<String, CachedManifest>>,
}

impl Default for LogSessionStore {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl LogSessionStore {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            layout: Layout { sessions_dir },
            mode: StoreMode::Persisted,
            locks: Mutex::new(HashMap::new()),
            leaf_ids: RwLock::new(HashMap::new()),
            memory: RwLock::new(HashMap::new()),
            manifest_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            layout: Layout {
                sessions_dir: PathBuf::from("."),
            },
            mode: StoreMode::InMemory,
            locks: Mutex::new(HashMap::new()),
            leaf_ids: RwLock::new(HashMap::new()),
            memory: RwLock::new(HashMap::new()),
            manifest_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".xylitol")
            .join("sessions")
    }

    pub(crate) fn set_leaf(&self, session_id: &str, entry_id: Option<String>) {
        lock_rwlock_write(&self.leaf_ids).insert(session_id.to_string(), entry_id);
    }

    pub(crate) fn get_leaf(&self, session_id: &str) -> Option<String> {
        lock_rwlock_read(&self.leaf_ids)
            .get(session_id)
            .cloned()
            .flatten()
    }

    pub(crate) fn cache_manifest(&self, session_id: &str, manifest: layout::Manifest, dirty: bool) {
        lock_rwlock_write(&self.manifest_cache)
            .insert(session_id.to_string(), CachedManifest { manifest, dirty });
    }

    pub(crate) fn cached_manifest(&self, session_id: &str) -> Option<layout::Manifest> {
        lock_rwlock_read(&self.manifest_cache)
            .get(session_id)
            .map(|cached| cached.manifest.clone())
    }

    pub(crate) fn drop_cached_manifest(&self, session_id: &str) {
        lock_rwlock_write(&self.manifest_cache).remove(session_id);
    }

    pub(crate) fn acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        if matches!(self.mode, StoreMode::InMemory) {
            return Ok(());
        }
        let mut map = lock_mutex(&self.locks);
        if map.contains_key(session_id) {
            return Ok(());
        }
        let dir = self.layout.session_dir(session_id);
        let lock = SessionWriteLock::try_acquire(&dir, session_id)?;
        map.insert(session_id.to_string(), lock);
        Ok(())
    }

    pub(crate) fn release_write_lock(&self, session_id: &str) {
        lock_mutex(&self.locks).remove(session_id);
    }

    pub async fn append(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        append::append(self, session_id, entry, false).await
    }

    pub async fn append_with_id(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        append::append(self, session_id, entry, true).await
    }

    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        project::load_all(self, session_id).await
    }

    pub async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        project::load_all(self, session_id).await
    }

    pub async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        project::load_branch(self, session_id, None).await
    }

    pub async fn get_branch(
        &self,
        session_id: &str,
        leaf_id: Option<&str>,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        project::load_branch(self, session_id, leaf_id).await
    }

    pub async fn build_context_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        project::build_context_entries(self, session_id).await
    }

    pub async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<xylitol_plugin_sdk::session::SessionContext, XySessionStoreError> {
        project::build_session_context(self, session_id).await
    }

    pub async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        append::commit_compaction(self, session_id, entry).await
    }

    pub async fn list(&self) -> Result<Vec<String>, XySessionStoreError> {
        project::list_session_ids(self).await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        XySessionStore::delete_session(self, session_id).await
    }

    pub async fn exists(&self, session_id: &str) -> bool {
        XySessionStore::exists(self, session_id).await
    }

    pub async fn session_profile(
        &self,
        session_id: &str,
    ) -> Result<SessionProfile, XySessionStoreError> {
        XySessionStore::session_profile(self, session_id).await
    }

    pub async fn set_session_profile(
        &self,
        session_id: &str,
        profile: SessionProfile,
    ) -> Result<(), XySessionStoreError> {
        XySessionStore::set_session_profile(self, session_id, profile).await
    }

    pub fn holds_write_lock(&self, session_id: &str) -> bool {
        lock_mutex(&self.locks).contains_key(session_id)
    }

    pub fn write_lock_path(&self, session_id: &str) -> PathBuf {
        self.layout
            .session_dir(session_id)
            .join(SESSION_WRITE_LOCK_FILENAME)
    }

    pub fn manifest_path(&self, session_id: &str) -> PathBuf {
        self.layout.manifest_path(session_id)
    }

    pub fn session_dir_path(&self, session_id: &str) -> PathBuf {
        self.layout.session_dir(session_id)
    }

    pub fn legacy_session_path(&self, session_id: &str) -> PathBuf {
        self.layout.legacy_path(session_id)
    }

    pub(crate) fn exists_sync(&self, session_id: &str) -> bool {
        match self.mode {
            StoreMode::InMemory => lock_rwlock_read(&self.memory).contains_key(session_id),
            StoreMode::Persisted => self.layout.log_dir(session_id).exists(),
        }
    }

    pub(crate) fn active_log_file(&self, session_id: &str) -> PathBuf {
        if let Some(manifest) = self.cached_manifest(session_id) {
            return self.layout.log_file(session_id, manifest.active_file);
        }
        if let Ok(raw) = std::fs::read(self.layout.manifest_path(session_id))
            && let Ok(manifest) = serde_json::from_slice::<layout::Manifest>(&raw)
        {
            return self.layout.log_file(session_id, manifest.active_file);
        }
        self.layout.log_file(session_id, 1)
    }

    pub fn backend(&self) -> crate::types::SessionBackend {
        match self.mode {
            StoreMode::Persisted => crate::types::SessionBackend::Persisted,
            StoreMode::InMemory => crate::types::SessionBackend::InMemory,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionHealthReport {
    pub id: String,
    pub files: usize,
    pub bytes: u64,
    pub entries: usize,
    pub suggested_threshold_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MaintenanceReport {
    pub sessions: Vec<SessionHealthReport>,
}

pub async fn maintain_sessions(
    sessions_dir: PathBuf,
) -> Result<MaintenanceReport, XySessionStoreError> {
    let store = LogSessionStore::new(sessions_dir);
    let report = maintain::maintain_all(&store).await?;
    Ok(MaintenanceReport {
        sessions: report
            .sessions
            .into_iter()
            .map(|health| SessionHealthReport {
                id: health.id,
                files: health.files,
                bytes: health.bytes,
                entries: health.entries,
                suggested_threshold_bytes: health.suggested_threshold_bytes,
            })
            .collect(),
    })
}
