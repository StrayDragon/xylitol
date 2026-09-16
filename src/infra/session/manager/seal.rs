//! Compaction seal transaction for segmented sessions.

use super::SessionManager;
use crate::infra::session::types::{SessionBackend, SessionEntry};
use crate::protocol::error::XySessionStoreError;
use crate::utils::lock_rwlock_read;

impl SessionManager {
    /// Append a session entry without applying compaction-specific storage policy.
    pub async fn append_session_entry(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        self.append(session_id, entry).await
    }

    /// Commit a compaction entry, sealing its summarized prefix when possible.
    pub async fn commit_compaction(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        if !matches!(entry, SessionEntry::Compaction(_)) {
            return Err(XySessionStoreError::validation(
                "compaction commit requires a compaction entry",
            ));
        }

        if matches!(&self.backend, SessionBackend::InMemory) {
            return self.append(session_id, entry).await;
        }
        self.append_compaction_with_seal(session_id, entry).await
    }

    async fn append_compaction_with_seal(
        &self,
        session_id: &str,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        if self.legacy_session_path(session_id).exists() && !self.manifest_path(session_id).exists()
        {
            self.migrate_legacy_session(session_id).await?;
        }
        if !self.manifest_path(session_id).exists() {
            // Compaction normally follows an assistant flush. Keep the seam
            // robust for imported/pending sessions as well.
            self.create(session_id, Some("."), None).await?;
            if lock_rwlock_read(&self.pending_store).contains_key(session_id) {
                self.flush_pending_to_disk(session_id).await?;
            }
        }

        let manifest = self.read_manifest(session_id).await?;
        let active_entries = self
            .read_segment(session_id, &manifest.active_segment)
            .await?;
        let compaction = self.inject_ids(session_id, entry);
        let SessionEntry::Compaction(compaction_ref) = &compaction else {
            unreachable!("append_compaction_with_seal received non-compaction entry")
        };
        let first_kept_index = active_entries.iter().position(|candidate| {
            candidate.entry_id() == Some(compaction_ref.first_kept_entry_id.as_str())
        });

        let Some(first_kept_index) = first_kept_index else {
            self.append_active_entry(session_id, &manifest, &compaction)
                .await?;
            self.set_leaf_from_entry(session_id, &compaction);
            return Ok(());
        };

        let prefix = &active_entries[..first_kept_index];
        let has_summarized_history = prefix
            .iter()
            .any(|candidate| !matches!(candidate, SessionEntry::Header(_)));
        if !has_summarized_history {
            self.append_active_entry(session_id, &manifest, &compaction)
                .await?;
            self.set_leaf_from_entry(session_id, &compaction);
            return Ok(());
        }

        let generation = Self::next_generation(&manifest);
        let cold_path = format!("segments/{generation:020}-sealed.jsonl");
        let active_path = format!("active-{generation}.jsonl");
        let cold_segment = Self::segment_descriptor(cold_path, generation, prefix);

        let mut active_entries_after = active_entries[first_kept_index..].to_vec();
        active_entries_after.push(compaction.clone());
        let active_segment =
            Self::segment_descriptor(active_path, generation, &active_entries_after);

        let mut next_manifest = manifest.clone();
        next_manifest.sealed_segments.push(cold_segment.clone());
        next_manifest.active_segment = active_segment.clone();
        next_manifest.leaf_entry_id = compaction.entry_id().map(str::to_owned);

        let cold_result = self
            .write_segment_atomically(session_id, &cold_segment, prefix)
            .await;
        if let Err(error) = cold_result {
            return Err(error);
        }
        if let Err(error) = self
            .write_segment_atomically(session_id, &active_segment, &active_entries_after)
            .await
        {
            self.remove_segment_if_present(session_id, &cold_segment)
                .await;
            return Err(error);
        }
        if let Err(error) = self
            .write_manifest_atomically(session_id, &next_manifest)
            .await
        {
            self.remove_segment_if_present(session_id, &cold_segment)
                .await;
            self.remove_segment_if_present(session_id, &active_segment)
                .await;
            return Err(error);
        }

        // The manifest is the commit point. The old active remains recoverable
        // until this point and is safe to remove afterwards.
        self.remove_segment_if_present(session_id, &manifest.active_segment)
            .await;
        self.set_leaf_from_entry(session_id, &compaction);
        Ok(())
    }

    async fn append_active_entry(
        &self,
        session_id: &str,
        manifest: &crate::protocol::session::SessionManifest,
        entry: &SessionEntry,
    ) -> Result<(), XySessionStoreError> {
        let path = self.resolve_segment_path(session_id, &manifest.active_segment.path)?;
        let line = serde_json::to_string(entry).map_err(XySessionStoreError::from)?;
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .await
            .map_err(|e| XySessionStoreError::io("open active session segment", e))?;
        file.write_all(format!("{line}\n").as_bytes())
            .await
            .map_err(|e| XySessionStoreError::io("append active session segment", e))?;
        file.flush()
            .await
            .map_err(|e| XySessionStoreError::io("flush active session segment", e))?;
        file.sync_all()
            .await
            .map_err(|e| XySessionStoreError::io("sync active session segment", e))?;
        Ok(())
    }

    async fn remove_segment_if_present(
        &self,
        session_id: &str,
        segment: &crate::protocol::session::SessionSegment,
    ) {
        if let Ok(path) = self.resolve_segment_path(session_id, &segment.path) {
            let _ = tokio::fs::remove_file(path).await;
        }
    }

    fn set_leaf_from_entry(&self, session_id: &str, entry: &SessionEntry) {
        if let Some(id) = entry.entry_id() {
            self.set_leaf(session_id, Some(id.to_string()));
        }
    }
}
