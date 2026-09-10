use std::collections::HashMap;

use uuid::Uuid;

use super::SessionManager;
use crate::infra::session::types::*;
use crate::protocol::error::{XySessionError, XySessionStoreError};
use crate::utils::{lock_rwlock_read, lock_rwlock_write};

impl SessionManager {
    // ── BDD @executable session-navigation surface ──────────────────

    /// Branch: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn branch(&self, session_id: &str, entry_id: &str) {
        self.set_leaf(session_id, Some(entry_id.to_string()));
    }

    /// Reset leaf to root (null).
    #[allow(dead_code)] // BDD @executable contract, not product-called
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

        // Anchor on a chain entry: a stored/absent leaf that lands on bookkeeping
        // (e.g. parent-less trailing modelChange from cold materialize) must not
        // truncate the branch to that row alone.
        let Some(effective_leaf) =
            crate::protocol::session::transcript_leaf_anchor(&entries, leaf_id)
        else {
            return Ok(vec![]);
        };

        // Collect path from leaf to root; splices across bookkeeping seams
        // (parent-less modelChange/thinkingLevelChange spliced into the chain).
        let path_ids = crate::protocol::session::transcript_ancestry_ids(
            &entries,
            Some(effective_leaf.as_str()),
        );
        let mut path = Vec::with_capacity(path_ids.len());
        for id in &path_ids {
            if let Some(entry) = id_map.get(id.as_str()) {
                path.push((*entry).clone());
            }
        }
        Ok(path)
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
            if matches!(entry, SessionEntry::Label(_)) {
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
        self.stamp_header_fork_at_entry_id(child_id, at_entry_id);

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
            SessionBackend::InMemory => {
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

    // ── Tree operations (BDD @executable session-navigation surface) ──

    /// Get the active session id.
    #[allow(dead_code)] // BDD @executable pair of `set_active_session`; reader unbound today
    pub fn active_session_id(&self) -> Option<String> {
        lock_rwlock_read(&self.active_session).clone()
    }

    /// Set the active session (`导航到` step target bookkeeping).
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn set_active_session(&self, id: &str) {
        lock_rwlock_write(&self.active_session).replace(id.to_string());
    }

    /// Navigate tree: change the current leaf to a different entry.
    /// Future appends will be children of this entry.
    #[allow(dead_code)] // BDD @executable contract, not product-called
    pub fn navigate_tree(&self, session_id: &str, target_id: Option<&str>) {
        match target_id {
            Some(id) => self.branch(session_id, id),
            None => self.reset_leaf(session_id),
        }
    }
}
