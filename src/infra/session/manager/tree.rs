use std::collections::HashMap;

use uuid::Uuid;

use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::{XySessionError, XySessionStoreError};
use crate::utils::{lock_rwlock_read, lock_rwlock_write};

impl SessionManager {
    /// Branch: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    pub fn branch(&self, session_id: &str, entry_id: &str) {
        self.set_leaf(session_id, Some(entry_id.to_string()));
    }

    /// Reset leaf to root (null).
    pub fn reset_leaf(&self, session_id: &str) {
        self.set_leaf(session_id, None);
    }

    /// Collect entries on the path from leaf_id to root.
    pub async fn get_branch(
        &self,
        session_id: &str,
        leaf_id: Option<&str>,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        let id_map: HashMap<&str, &SessionEntry> = entries
            .iter()
            .filter_map(|e| e.entry_id().map(|id| (id, e)))
            .collect();

        let effective_leaf = match leaf_id {
            Some(id) => id.to_string(),
            None => match entries.iter().rev().find_map(|e| e.entry_id()) {
                Some(id) => id.to_string(),
                None => return Ok(vec![]),
            },
        };

        // Collect path from leaf to root
        let mut path = Vec::new();
        let mut current = Some(effective_leaf.as_str());
        let mut visited = std::collections::HashSet::new();

        while let Some(id) = current {
            if !visited.insert(id) {
                break; // Cycle detection
            }
            if let Some(entry) = id_map.get(id) {
                path.push((*entry).clone());
                current = entry.parent_id();
            } else {
                break;
            }
        }

        path.reverse();
        Ok(path)
    }

    /// Get a range of branch entries between two entry IDs.
    /// Returns entries from `start_id` (inclusive) to `end_id` (exclusive).
    pub async fn get_branch_entries(
        &self,
        session_id: &str,
        start_id: &str,
        end_id: &str,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let leaf_id = self.get_leaf(session_id);
        let branch = self.get_branch(session_id, leaf_id.as_deref()).await?;

        let mut in_range = false;
        let mut result = Vec::new();
        for entry in &branch {
            let Some(eid) = entry.entry_id() else {
                continue;
            };
            if eid == start_id {
                in_range = true;
            }
            if in_range {
                result.push(entry.clone());
            }
            if eid == end_id {
                break;
            }
        }

        Ok(result)
    }

    /// Create a branched (forked) session from a parent session up to a given entry.
    /// This is a cleaner alias for `fork()` that creates the child explicitly.
    pub async fn create_branched_session(
        &self,
        parent_id: &str,
        child_id: &str,
        target_entry_id: &str,
    ) -> Result<(), XySessionError> {
        self.fork(
            parent_id,
            child_id,
            target_entry_id,
            crate::protocol::session::ForkPosition::At,
        )
        .await
    }

    // ── Fork ────────────────────────────────────────────────────

    /// Fork a session: create a child session from a parent branch path.
    ///
    /// Aligns with pi `createBranchedSession`: content is [`Self::get_branch`],
    /// `parent_id` values are re-chained; the parent JSONL is never mutated.
    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<(), XySessionError> {
        self.fork_inner(parent_id, child_id, at_entry_id, position)
            .await
    }

    /// Pi runtime guard when the parent JSONL is not on disk yet.
    pub const UNFLUSHED_FORK_ERR: &str = "This session has not been saved yet. Wait for the first assistant response before cloning or forking it.";

    /// Fork implementation (path-based; see [`crate::protocol::session::ForkPosition`]).
    ///
    /// Aligns with pi `createBranchedSession`:
    /// - Persisted parent with no file yet → reject ([`Self::UNFLUSHED_FORK_ERR`]).
    /// - Strip label entries from the path and re-chain; recreate labels for targets on the path.
    /// - Child path with no assistant → keep pending (no file) until first assistant append.
    /// - Child path with assistant → flush to disk immediately.
    pub async fn fork_inner(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: crate::protocol::session::ForkPosition,
    ) -> Result<(), XySessionError> {
        use crate::protocol::session::{ForkPosition, is_assistant_message, is_user_message};

        if matches!(&self.backend, SessionBackend::Persisted { .. })
            && !self.session_file_exists(parent_id)
        {
            return Err(XySessionStoreError::validation(Self::UNFLUSHED_FORK_ERR).into());
        }

        let parent_entries = self.load(parent_id).await?;
        let selected = parent_entries
            .iter()
            .find(|e| e.entry_id() == Some(at_entry_id))
            .ok_or_else(|| XySessionError::entry_not_found(at_entry_id))?;

        let path_leaf: Option<&str> = match position {
            ForkPosition::At => Some(at_entry_id),
            ForkPosition::Before => {
                if !is_user_message(selected) {
                    return Err(XySessionStoreError::validation(
                        "ForkPosition::Before requires a user message entry (pi /fork)",
                    )
                    .into());
                }
                selected.parent_id()
            }
        };

        let path = match path_leaf {
            None => Vec::new(),
            Some(leaf) => self.get_branch(parent_id, Some(leaf)).await?,
        };

        // Strip labels and re-chain (pi createBranchedSession).
        let mut rechanneled = Vec::with_capacity(path.len());
        let mut prev_id: Option<String> = None;
        for entry in &path {
            if entry.entry_type() == "label" {
                continue;
            }
            let Some(eid) = entry.entry_id() else {
                continue;
            };
            let ts = entry.base().map(|b| b.timestamp).unwrap_or(0);
            let rewritten = Self::clone_entry_with_ids(entry, eid, prev_id.as_deref(), ts);
            prev_id = Some(eid.to_string());
            rechanneled.push(rewritten);
        }

        let path_ids: std::collections::HashSet<&str> =
            rechanneled.iter().filter_map(|e| e.entry_id()).collect();
        // Last label wins per target (same as build_session_tree).
        let mut labels_by_target: HashMap<String, (Option<String>, u64)> = HashMap::new();
        for entry in &parent_entries {
            if let SessionEntry::Label(l) = entry
                && path_ids.contains(l.target_id.as_str())
            {
                if l.label.is_none() {
                    labels_by_target.remove(&l.target_id);
                } else {
                    labels_by_target
                        .insert(l.target_id.clone(), (l.label.clone(), l.base.timestamp));
                }
            }
        }
        let mut label_targets: Vec<_> = labels_by_target.into_iter().collect();
        label_targets.sort_by(|a, b| a.1.1.cmp(&b.1.1).then_with(|| a.0.cmp(&b.0)));

        let mut label_entries = Vec::with_capacity(label_targets.len());
        let mut label_parent = rechanneled
            .last()
            .and_then(|e| e.entry_id())
            .map(str::to_string);
        for (target_id, (label, timestamp)) in label_targets {
            let id = Uuid::new_v4().to_string();
            label_entries.push(SessionEntry::Label(LabelEntry {
                base: EntryBase {
                    entry_type: "label".into(),
                    id: id.clone(),
                    parent_id: label_parent.clone(),
                    timestamp,
                },
                target_id,
                label,
            }));
            label_parent = Some(id);
        }

        let mut child_body = rechanneled;
        child_body.extend(label_entries);

        let parent_cwd = parent_entries.iter().find_map(|e| {
            if let SessionEntry::Header(h) = e {
                Some(h.cwd.as_str())
            } else {
                None
            }
        });
        self.create(child_id, parent_cwd, Some(parent_id)).await?;

        let has_assistant = child_body.iter().any(is_assistant_message);

        match &self.backend {
            SessionBackend::Persisted { .. } => {
                if has_assistant {
                    // Path includes assistant → write immediately (pi _rewriteFile).
                    self.flush_pending_to_disk(child_id).await?;
                    for entry in &child_body {
                        self.append_with_id(child_id, entry).await?;
                    }
                } else {
                    // No assistant → stay pending until first assistant append (pi deferred).
                    let mut pending = lock_rwlock_write(&self.pending_store);
                    pending
                        .entry(child_id.to_string())
                        .or_default()
                        .extend(child_body.iter().cloned());
                }
            }
            SessionBackend::InMemory { .. } => {
                let mut store = lock_rwlock_write(&self.in_memory_store);
                store
                    .entry(child_id.to_string())
                    .or_default()
                    .extend(child_body.iter().cloned());
            }
        }

        if let Some(last) = child_body.last().and_then(|e| e.entry_id()) {
            self.set_leaf(child_id, Some(last.to_string()));
        } else {
            self.set_leaf(child_id, None);
        }

        Ok(())
    }

    // ── Tree operations ────────────────────────────────────────

    /// Get the active session id.
    pub fn active_session_id(&self) -> Option<String> {
        lock_rwlock_read(&self.active_session).clone()
    }

    /// Set the active session.
    pub fn set_active_session(&self, id: &str) {
        lock_rwlock_write(&self.active_session).replace(id.to_string());
    }

    /// Navigate tree: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    /// Alias for `branch()`.
    pub fn navigate_tree(&self, session_id: &str, target_id: Option<&str>) {
        match target_id {
            Some(id) => self.branch(session_id, id),
            None => self.reset_leaf(session_id),
        }
    }

    /// Switch the active session to a new file path.
    /// This loads entries from the new path and updates the active session.
    pub async fn switch_session(
        &self,
        new_session_id: &str,
        new_path: &str,
    ) -> Result<(), XySessionStoreError> {
        // Verify the new path exists
        let path = std::path::Path::new(new_path);
        if !path.exists() {
            return Err(XySessionStoreError::not_found(new_path));
        }
        // Load entries from the new path
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| XySessionStoreError::io("read session file", e))?;

        let mut entries: Vec<SessionEntry> = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: SessionEntry =
                serde_json::from_str(line).map_err(XySessionStoreError::from)?;
            entries.push(entry);
        }

        // Update active session
        self.set_active_session(new_session_id);

        // Update leaf tracking: last entry's id
        if let Some(last) = entries.last() {
            if let Some(id) = last.entry_id() {
                self.set_leaf(new_session_id, Some(id.to_string()));
            }
        } else {
            self.set_leaf(new_session_id, None);
        }

        Ok(())
    }

    /// Get the session as a tree structure.
    /// Builds a `Vec<SessionTreeNode>` with labels resolved from LabelEntries.
    pub async fn get_tree(
        &self,
        session_id: &str,
    ) -> Result<Vec<SessionTreeNode>, XySessionStoreError> {
        let entries = self.load(session_id).await?;
        Ok(crate::protocol::session::build_session_tree(&entries))
    }
}
