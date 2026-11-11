use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::log::{LogSessionStore, generate_branch_summary};
use crate::sync::{lock_rwlock_read, lock_rwlock_write};
use crate::types::*;
use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
use xylitol_plugin_sdk::session::{ForkPosition, SessionProfile, ThinkingLevelChangeEntry};
use xylitol_plugin_sdk::{SessionListEntry, XySessionStore};

#[derive(Clone)]
pub struct SessionManager {
    inner: Arc<LogSessionStore>,
    pub backend: SessionBackend,
    active_session: Arc<RwLock<Option<String>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(LogSessionStore::new(sessions_dir)),
            backend: SessionBackend::Persisted,
            active_session: Arc::new(RwLock::new(None)),
        }
    }

    pub fn in_memory() -> Self {
        Self {
            inner: Arc::new(LogSessionStore::in_memory()),
            backend: SessionBackend::InMemory,
            active_session: Arc::new(RwLock::new(None)),
        }
    }

    pub fn default_dir() -> PathBuf {
        LogSessionStore::default_dir()
    }

    pub async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        self.inner.create(id, cwd, parent).await
    }

    pub async fn append(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.inner.append(session_id, entry).await
    }

    pub async fn append_with_id(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.inner.append_with_id(session_id, entry).await
    }

    pub async fn load(&self, session_id: &str) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.load(session_id).await
    }

    pub async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.load_entries(session_id).await
    }

    pub async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.load_leaf_branch(session_id).await
    }

    pub async fn build_context_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.build_context_entries(session_id).await
    }

    pub async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError> {
        self.inner.build_session_context(session_id).await
    }

    pub async fn get_branch(
        &self,
        session_id: &str,
        leaf_id: Option<&str>,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.get_branch(session_id, leaf_id).await
    }

    pub async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.inner.commit_compaction(session_id, entry).await
    }

    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError> {
        self.inner
            .fork(parent_id, child_id, at_entry_id, position)
            .await
    }

    pub async fn list(&self) -> Result<Vec<String>, XySessionStoreError> {
        self.inner.list().await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        self.inner.delete_session(session_id).await
    }

    pub async fn exists(&self, session_id: &str) -> bool {
        self.inner.exists(session_id).await
    }

    pub fn exists_sync(&self, session_id: &str) -> bool {
        self.inner.exists_sync(session_id)
    }

    pub async fn session_profile(
        &self,
        session_id: &str,
    ) -> Result<SessionProfile, XySessionStoreError> {
        self.inner.session_profile(session_id).await
    }

    pub async fn set_session_profile(
        &self,
        session_id: &str,
        profile: SessionProfile,
    ) -> Result<(), XySessionStoreError> {
        self.inner.set_session_profile(session_id, profile).await
    }

    pub async fn flush_pending_to_disk(
        &self,
        _session_id: &str,
    ) -> Result<(), XySessionStoreError> {
        Ok(())
    }

    pub async fn append_model_change(
        &self,
        session_id: &str,
        provider: &str,
        model_id: &str,
    ) -> Result<(), XySessionStoreError> {
        let entry = SessionEntry::ModelChange(ModelChangeEntry {
            base: empty_base("modelChange"),
            provider: provider.to_string(),
            model_id: model_id.to_string(),
        });
        self.append(session_id, &entry).await
    }

    pub async fn append_thinking_level_change(
        &self,
        session_id: &str,
        thinking_level: &str,
    ) -> Result<(), XySessionStoreError> {
        let entry = SessionEntry::ThinkingLevelChange(ThinkingLevelChangeEntry {
            base: empty_base("thinkingLevelChange"),
            thinking_level: thinking_level.to_string(),
        });
        self.append(session_id, &entry).await
    }

    pub async fn append_label_change(
        &self,
        session_id: &str,
        target_id: &str,
        label: Option<&str>,
    ) -> Result<(), XySessionError> {
        let entry = SessionEntry::Label(LabelEntry {
            base: empty_base("label"),
            target_id: target_id.to_string(),
            label: label.map(str::to_string),
        });
        self.append(session_id, &entry)
            .await
            .map_err(XySessionError::from)
    }

    pub async fn get_label(
        &self,
        session_id: &str,
        target_id: &str,
    ) -> Result<Option<String>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        for entry in entries.iter().rev() {
            if let SessionEntry::Label(label) = entry
                && label.target_id == target_id
            {
                return Ok(label.label.clone());
            }
        }
        Ok(None)
    }

    pub fn generate_branch_summary(&self, skipped_entries: &[SessionEntry]) -> String {
        generate_branch_summary(skipped_entries)
    }

    pub fn set_leaf(&self, session_id: &str, entry_id: Option<String>) {
        LogSessionStore::set_leaf(&self.inner, session_id, entry_id);
    }

    pub fn get_leaf_id(&self, session_id: &str) -> Option<String> {
        self.inner.get_leaf(session_id)
    }

    pub fn branch(&self, session_id: &str, entry_id: &str) {
        self.set_leaf(session_id, Some(entry_id.to_string()));
    }

    pub fn reset_leaf(&self, session_id: &str) {
        self.set_leaf(session_id, None);
    }

    pub fn navigate_tree(&self, session_id: &str, target_id: Option<&str>) {
        match target_id {
            Some(id) => self.branch(session_id, id),
            None => self.reset_leaf(session_id),
        }
    }

    pub fn active_session_id(&self) -> Option<String> {
        lock_rwlock_read(&self.active_session).clone()
    }

    pub fn set_active_session(&self, id: &str) {
        *lock_rwlock_write(&self.active_session) = Some(id.to_string());
    }

    pub fn acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        self.inner.acquire_write_lock(session_id)
    }

    pub fn release_write_lock(&self, session_id: &str) {
        self.inner.release_write_lock(session_id);
    }

    pub fn holds_write_lock(&self, session_id: &str) -> bool {
        self.inner.holds_write_lock(session_id)
    }

    pub fn write_lock_path(&self, session_id: &str) -> PathBuf {
        self.inner.write_lock_path(session_id)
    }

    pub fn manifest_path(&self, session_id: &str) -> PathBuf {
        self.inner.manifest_path(session_id)
    }

    pub fn session_dir_path(&self, session_id: &str) -> PathBuf {
        self.inner.session_dir_path(session_id)
    }

    pub async fn load_validated(
        &self,
        session_id: &str,
        fallback_cwd: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        assert_session_cwd_exists(&entries, fallback_cwd)?;
        Ok(entries)
    }

    pub fn get_session_file(&self, session_id: &str) -> Result<PathBuf, XySessionStoreError> {
        if !self.inner.exists_sync(session_id) {
            return Err(XySessionStoreError::not_found(session_id));
        }
        Ok(self.inner.active_log_file(session_id))
    }

    pub fn legacy_session_path(&self, session_id: &str) -> PathBuf {
        self.inner.legacy_session_path(session_id)
    }
}

fn empty_base(entry_type: &str) -> EntryBase {
    EntryBase {
        entry_type: entry_type.into(),
        id: String::new(),
        parent_id: None,
        timestamp: 0,
    }
}

#[async_trait::async_trait]
impl XySessionStore for SessionManager {
    async fn exists(&self, session_id: &str) -> bool {
        self.inner.exists(session_id).await
    }

    async fn session_profile(
        &self,
        session_id: &str,
    ) -> Result<SessionProfile, XySessionStoreError> {
        self.inner.session_profile(session_id).await
    }

    async fn set_session_profile(
        &self,
        session_id: &str,
        profile: SessionProfile,
    ) -> Result<(), XySessionStoreError> {
        self.inner.set_session_profile(session_id, profile).await
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.load_entries(session_id).await
    }

    async fn load_entries_page(
        &self,
        session_id: &str,
        limit: Option<usize>,
        before_entry_id: Option<&str>,
    ) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
        self.inner
            .load_entries_page(session_id, limit, before_entry_id)
            .await
    }

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.load_leaf_branch(session_id).await
    }

    async fn build_context_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        self.inner.build_context_entries(session_id).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.inner.append_session_entry(session_id, entry).await
    }

    async fn load_done_bash_ids(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashSet<String>, XySessionStoreError> {
        self.inner.load_done_bash_ids(session_id).await
    }

    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.inner.commit_compaction(session_id, entry).await
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<SessionContext, XySessionStoreError> {
        self.inner.build_session_context(session_id).await
    }

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        self.inner.create(id, cwd, parent).await
    }

    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: ForkPosition,
    ) -> Result<(), XySessionError> {
        self.inner
            .fork(parent_id, child_id, at_entry_id, position)
            .await
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        self.inner
            .set_leaf(session_id, entry_id.map(str::to_string));
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        self.inner.leaf_id(session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionListEntry>, XySessionStoreError> {
        self.inner.list_sessions().await
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        self.inner.delete_session(session_id).await
    }

    fn try_acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        self.inner.try_acquire_write_lock(session_id)
    }

    fn release_write_lock(&self, session_id: &str) {
        self.inner.release_write_lock(session_id);
    }
}

fn assert_session_cwd_exists(
    entries: &[SessionEntry],
    fallback_cwd: &str,
) -> Result<(), XySessionStoreError> {
    let header = entries.iter().find_map(|entry| match entry {
        SessionEntry::Header(header) => Some(header),
        _ => None,
    });
    let Some(header) = header else {
        return Ok(());
    };
    let cwd = if header.cwd.is_empty() {
        "."
    } else {
        &header.cwd
    };
    let cwd_path = std::path::Path::new(cwd);
    if cwd_path.is_dir() {
        return Ok(());
    }
    let fallback_path = std::path::Path::new(fallback_cwd);
    if fallback_path.is_dir() {
        return Ok(());
    }
    Err(XySessionStoreError::validation(format!(
        "Session working directory '{cwd}' does not exist. Fallback '{fallback_cwd}' also not found."
    )))
}
