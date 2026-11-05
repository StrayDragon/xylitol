use std::collections::HashMap;

use uuid::Uuid;

use super::SessionManager;
use crate::sync::{lock_rwlock_read, lock_rwlock_write};
use crate::types::*;
use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
use xylitol_plugin_sdk::message::now_ms;
use xylitol_plugin_sdk::session::{Branch, EdgeKind, EdgeRecord};

impl SessionManager {
    #[allow(dead_code)]
    pub fn branch(&self, session_id: &str, entry_id: &str) {
        self.set_leaf(session_id, Some(entry_id.to_string()));
    }

    #[allow(dead_code)]
    pub fn reset_leaf(&self, session_id: &str) {
        self.set_leaf(session_id, None);
    }

    pub async fn get_branch(
        &self,
        session_id: &str,
        leaf_id: Option<&str>,
    ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
        let entries = self.load_branch_entries(session_id, leaf_id).await?;
        let id_map: HashMap<&str, &SessionEntry> = entries
            .iter()
            .filter_map(|e| e.entry_id().map(|id| (id, e)))
            .collect();

        let Some(effective_leaf) =
            xylitol_plugin_sdk::session::transcript_leaf_anchor(&entries, leaf_id)
        else {
            return Ok(vec![]);
        };

        let path_ids = xylitol_plugin_sdk::session::transcript_ancestry_ids(
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

    pub async fn fork(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<(), XySessionError> {
        self.fork_inner(parent_id, child_id, at_entry_id, position)
            .await
    }

    pub const UNFLUSHED_FORK_ERR: &str = "This session has not been saved yet. Wait for the first assistant response before cloning or forking it.";

    pub async fn fork_inner(
        &self,
        parent_id: &str,
        child_id: &str,
        at_entry_id: &str,
        position: xylitol_plugin_sdk::session::ForkPosition,
    ) -> Result<(), XySessionError> {
        use std::collections::HashSet;
        use xylitol_plugin_sdk::session::{
            BranchSummaryEntry, ForkPosition, ParentRef, is_user_message,
        };

        if matches!(&self.backend, SessionBackend::Persisted)
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
        let from_id = path_leaf.unwrap_or(at_entry_id).to_string();
        let path_ids: HashSet<&str> = path.iter().filter_map(|e| e.entry_id()).collect();
        let skipped: Vec<SessionEntry> = parent_entries
            .iter()
            .filter(|e| matches!(e, SessionEntry::Message(_)))
            .filter(|e| e.entry_id().is_some_and(|id| !path_ids.contains(id)))
            .cloned()
            .collect();
        let summary = self.generate_branch_summary(&skipped);
        let summary = if summary.is_empty() {
            format!("{child_id}: no skipped entries")
        } else {
            format!("{child_id}: {summary}")
        };

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

        let parent_cwd = parent_entries.iter().find_map(|e| {
            if let SessionEntry::Header(h) = e {
                Some(h.cwd.as_str())
            } else {
                None
            }
        });

        let parent_segments = if matches!(&self.backend, SessionBackend::Persisted) {
            Some(self.seal_active_for_fork(parent_id).await?)
        } else {
            None
        };

        self.create(child_id, parent_cwd, Some(parent_id)).await?;
        self.stamp_header_fork_at_entry_id(child_id, at_entry_id);

        let summary_id = Uuid::new_v4().to_string();
        let summary_entry = SessionEntry::BranchSummary(BranchSummaryEntry {
            base: EntryBase {
                entry_type: "branchSummary".into(),
                id: summary_id.clone(),
                parent_id: Some(from_id.clone()),
                timestamp: now_ms(),
            },
            from_id: from_id.clone(),
            summary,
            details: None,
            from_hook: None,
        });

        match &self.backend {
            SessionBackend::Persisted => {
                self.flush_pending_to_disk(child_id).await?;
                self.append_with_id(child_id, &summary_entry).await?;
                let mut label_parent = Some(summary_id.clone());
                for (target_id, (label, timestamp)) in label_targets {
                    let id = Uuid::new_v4().to_string();
                    let entry = SessionEntry::Label(LabelEntry {
                        base: EntryBase {
                            entry_type: "label".into(),
                            id: id.clone(),
                            parent_id: label_parent.clone(),
                            timestamp,
                        },
                        target_id,
                        label,
                    });
                    self.append_with_id(child_id, &entry).await?;
                    label_parent = Some(id);
                }
                let mut manifest = self.read_manifest(child_id).await?;
                manifest.parent = Some(ParentRef {
                    session_id: parent_id.to_string(),
                    fork_entry_id: at_entry_id.to_string(),
                    segments: parent_segments.unwrap_or_default(),
                });
                manifest.branches = vec![Branch {
                    id: child_id.to_string(),
                    head_entry_id: self.get_leaf(child_id),
                    parent_session: Some(parent_id.to_string()),
                    fork_entry_id: Some(at_entry_id.to_string()),
                    workspace_ref: None,
                }];
                self.write_manifest_atomically(child_id, &manifest).await?;
            }
            SessionBackend::InMemory => {
                {
                    let mut store = lock_rwlock_write(&self.in_memory_store);
                    store
                        .entry(child_id.to_string())
                        .or_default()
                        .push(summary_entry);
                }
                self.set_leaf(child_id, Some(summary_id.clone()));
                let mut label_parent = Some(summary_id);
                for (target_id, (label, timestamp)) in label_targets {
                    let id = Uuid::new_v4().to_string();
                    let entry = SessionEntry::Label(LabelEntry {
                        base: EntryBase {
                            entry_type: "label".into(),
                            id: id.clone(),
                            parent_id: label_parent.clone(),
                            timestamp,
                        },
                        target_id,
                        label,
                    });
                    {
                        let mut store = lock_rwlock_write(&self.in_memory_store);
                        store.entry(child_id.to_string()).or_default().push(entry);
                    }
                    self.set_leaf(child_id, Some(id.clone()));
                    label_parent = Some(id);
                }
                lock_rwlock_write(&self.in_memory_parents)
                    .insert(child_id.to_string(), parent_id.to_string());
            }
        }

        if let Some(leaf) = self.get_leaf(child_id) {
            self.append_edge(
                child_id,
                &EdgeRecord {
                    id: format!("fork:{child_id}"),
                    edge_kind: EdgeKind::Fork,
                    from: at_entry_id.to_string(),
                    to: leaf,
                    order: 0,
                    enabled: true,
                    timestamp: now_ms(),
                },
            )
            .await?;
        }

        Ok(())
    }
    #[allow(dead_code)]
    pub fn active_session_id(&self) -> Option<String> {
        lock_rwlock_read(&self.active_session).clone()
    }

    #[allow(dead_code)]
    pub fn set_active_session(&self, id: &str) {
        lock_rwlock_write(&self.active_session).replace(id.to_string());
    }

    #[allow(dead_code)]
    pub fn navigate_tree(&self, session_id: &str, target_id: Option<&str>) {
        match target_id {
            Some(id) => self.branch(session_id, id),
            None => self.reset_leaf(session_id),
        }
    }
}
