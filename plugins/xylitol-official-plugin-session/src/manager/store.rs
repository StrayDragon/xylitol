use super::SessionManager;
use super::load::session_display_name_from_entries;
use crate::types::{SessionBackend, SessionEntry};
use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};

#[async_trait::async_trait]
impl XySessionStore for SessionManager {
    async fn exists(&self, session_id: &str) -> bool {
        self.exists(session_id)
    }

    async fn load_entries(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        SessionManager::read_entries(self, session_id).await
    }

    async fn load_entries_page(
        &self,
        session_id: &str,
        limit: Option<usize>,
        before_entry_id: Option<&str>,
    ) -> Result<(Vec<SessionEntry>, bool), XySessionStoreError> {
        SessionManager::read_entries_page(self, session_id, limit, before_entry_id).await
    }

    async fn load_leaf_branch(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let leaf_id = self.get_leaf(session_id);
        self.get_branch(session_id, leaf_id.as_deref()).await
    }

    async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        SessionManager::append_session_entry(self, session_id, entry).await
    }

    async fn load_done_bash_ids(
        &self,
        session_id: &str,
    ) -> Result<std::collections::HashSet<String>, XySessionStoreError> {
        if matches!(&self.backend, SessionBackend::InMemory) {
            let entries = XySessionStore::load_entries(self, session_id).await?;
            return Ok(xylitol_plugin_sdk::session::done_bash_ids(&entries));
        }
        if !self.manifest_path(session_id).exists() {
            if self.legacy_session_path(session_id).exists() {
                return Err(super::segments::legacy_unsupported_error(session_id));
            }
            let entries = self.read_entries(session_id).await?;
            return Ok(xylitol_plugin_sdk::session::done_bash_ids(&entries));
        }
        let manifest = self.read_manifest(session_id).await?;
        let mut done = self
            .read_segment(session_id, &manifest.active_segment)
            .await
            .map(|entries| xylitol_plugin_sdk::session::done_bash_ids(&entries))?;
        for segment in &manifest.sealed_segments {
            let segment_done: std::collections::HashSet<String> = match segment
                .index_path
                .as_deref()
            {
                Some(index_path) => {
                    match self.read_sealed_index_opt(session_id, index_path).await {
                        Some(index) => index.done_bash_ids.into_iter().collect(),
                        None => self
                            .read_segment(session_id, segment)
                            .await
                            .map(|entries| xylitol_plugin_sdk::session::done_bash_ids(&entries))?
                            .into_iter()
                            .collect(),
                    }
                }
                None => self
                    .read_segment(session_id, segment)
                    .await
                    .map(|entries| xylitol_plugin_sdk::session::done_bash_ids(&entries))?
                    .into_iter()
                    .collect(),
            };
            done.extend(segment_done);
        }
        Ok(done)
    }

    async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        SessionManager::commit_compaction(self, session_id, entry).await
    }

    async fn build_session_context(
        &self,
        session_id: &str,
    ) -> Result<xylitol_plugin_sdk::session::SessionContext, XySessionStoreError> {
        SessionManager::build_session_context(self, session_id).await
    }

    async fn create(
        &self,
        id: &str,
        cwd: Option<&str>,
        parent: Option<&str>,
    ) -> Result<(), XySessionStoreError> {
        SessionManager::create(self, id, cwd, parent).await
    }

    async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<(), XySessionError> {
        SessionManager::fork(self, parent_id, child_id, at_entry_id, position).await
    }

    fn set_leaf(&self, session_id: &str, entry_id: Option<&str>) {
        <SessionManager>::set_leaf(self, session_id, entry_id.map(str::to_string));
    }

    fn leaf_id(&self, session_id: &str) -> Option<String> {
        SessionManager::get_leaf_id(self, session_id)
    }

    async fn list_sessions(
        &self,
    ) -> Result<Vec<xylitol_plugin_sdk::SessionListEntry>, XySessionStoreError> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use xylitol_plugin_sdk::session::{SessionEntry, is_user_message, message_text};

        let ids = SessionManager::list(self).await?;
        let mut out = Vec::with_capacity(ids.len());
        let mut skipped = 0usize;
        let mut skip_examples: Vec<String> = Vec::new();
        for id in ids {
            let path = if self.manifest_path(&id).exists() {
                self.session_dir_path(&id)
            } else {
                self.legacy_session_path(&id)
            };
            let path_str = if path.exists() {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            };
            let entries = match SessionManager::load_entries_for_list(self, &id).await {
                Ok(e) => e,
                Err(e) => {
                    skipped += 1;
                    if skip_examples.len() < 3 {
                        skip_examples.push(format!("{id}: {e}"));
                    }
                    log::debug!(
                        target: "xylitol::session",
                        "list_sessions skip {id}: {e}"
                    );
                    continue;
                }
            };
            let name = session_display_name_from_entries(&entries);
            let mut message_count = 0usize;
            let mut first_message = None;
            let mut parent_session_id = None;
            let mut modified_unix = None;
            let mut cwd = None;
            for entry in &entries {
                if let SessionEntry::Header(h) = entry {
                    parent_session_id = h.parent_session.clone();
                    if !h.cwd.is_empty() {
                        cwd = Some(h.cwd.clone());
                    }
                    modified_unix = Some(h.timestamp / 1000);
                    continue;
                }
                if matches!(entry, SessionEntry::Message(_)) {
                    message_count += 1;
                }
                if first_message.is_none()
                    && is_user_message(entry)
                    && let SessionEntry::Message(m) = entry
                {
                    let text = message_text(&m.message);
                    let cleaned = text
                        .chars()
                        .map(|c| if c.is_control() { ' ' } else { c })
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !cleaned.is_empty() {
                        first_message = Some(cleaned);
                    }
                }
            }
            if modified_unix.is_none() {
                let path = if self.manifest_path(&id).exists() {
                    self.session_dir_path(&id)
                } else {
                    self.legacy_session_path(&id)
                };
                if let Ok(meta) = tokio::fs::metadata(&path).await
                    && let Ok(modified) = meta.modified()
                    && let Ok(dur) = modified.duration_since(UNIX_EPOCH)
                {
                    modified_unix = Some(dur.as_secs());
                } else {
                    modified_unix = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs());
                }
            }
            out.push(xylitol_plugin_sdk::SessionListEntry {
                id,
                name,
                first_message,
                message_count,
                modified_unix,
                parent_session_id,
                tree_prefix: String::new(),
                cwd,
                path: path_str,
            });
        }
        if skipped > 0 {
            log::warn!(
                target: "xylitol::session",
                "list_sessions skipped {skipped} unreadable/legacy session(s); examples: {}",
                skip_examples.join("; ")
            );
        }
        out.sort_by(|a, b| {
            b.modified_unix
                .unwrap_or(0)
                .cmp(&a.modified_unix.unwrap_or(0))
        });
        Ok(out)
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        SessionManager::delete_session(self, session_id).await
    }

    fn try_acquire_write_lock(&self, session_id: &str) -> Result<(), XySessionStoreError> {
        SessionManager::acquire_write_lock(self, session_id)
    }

    fn release_write_lock(&self, session_id: &str) {
        SessionManager::release_write_lock(self, session_id);
    }
}
