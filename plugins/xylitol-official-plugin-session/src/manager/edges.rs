use std::path::PathBuf;

use tokio::io::AsyncWriteExt;

use super::SessionManager;
use crate::sync::{lock_rwlock_read, lock_rwlock_write};
use crate::types::*;
use xylitol_plugin_sdk::error::XySessionStoreError;
use xylitol_plugin_sdk::session::{EdgeKind, EdgeRecord};

fn derived_parent_edges(entries: &[SessionEntry]) -> Vec<EdgeRecord> {
    entries
        .iter()
        .filter_map(|entry| {
            let id = entry.entry_id()?;
            let parent = entry.parent_id()?;
            Some(EdgeRecord {
                id: format!("parent:{id}"),
                edge_kind: EdgeKind::Parent,
                from: parent.to_string(),
                to: id.to_string(),
                order: 0,
                enabled: true,
                timestamp: entry.base().map(|base| base.timestamp).unwrap_or(0),
            })
        })
        .collect()
}

impl SessionManager {
    pub(super) fn graph_path(&self, session_id: &str) -> PathBuf {
        self.session_dir_path(session_id).join("graph.jsonl")
    }

    pub(super) async fn read_edges(
        &self,
        session_id: &str,
        entries: &[SessionEntry],
    ) -> Result<Vec<EdgeRecord>, XySessionStoreError> {
        let mut edges = derived_parent_edges(entries);
        match &self.backend {
            SessionBackend::Persisted => {
                let path = self.graph_path(session_id);
                if path.exists() {
                    let content = tokio::fs::read_to_string(&path)
                        .await
                        .map_err(|e| XySessionStoreError::io("read session graph", e))?;
                    for line in content.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<EdgeRecord>(line) {
                            Ok(edge) => edges.push(edge),
                            Err(error) => log::warn!(
                                target: "xylitol::session",
                                "skip session graph edge line: {error}"
                            ),
                        }
                    }
                }
            }
            SessionBackend::InMemory => {
                let store = lock_rwlock_read(&self.in_memory_edges);
                if let Some(list) = store.get(session_id) {
                    edges.extend(list.iter().cloned());
                }
            }
        }
        Ok(edges)
    }

    pub(super) async fn append_edge(
        &self,
        session_id: &str,
        edge: &EdgeRecord,
    ) -> Result<(), XySessionStoreError> {
        match &self.backend {
            SessionBackend::Persisted => {
                self.acquire_write_lock(session_id)?;
                let path = self.graph_path(session_id);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| XySessionStoreError::io("create session graph dir", e))?;
                }
                let line = serde_json::to_string(edge)
                    .map_err(|e| XySessionStoreError::validation(format!("serialize edge: {e}")))?;
                let mut file = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await
                    .map_err(|e| XySessionStoreError::io("open session graph", e))?;
                file.write_all(format!("{line}\n").as_bytes())
                    .await
                    .map_err(|e| XySessionStoreError::io("write session graph", e))?;
                file.flush()
                    .await
                    .map_err(|e| XySessionStoreError::io("flush session graph", e))?;
            }
            SessionBackend::InMemory => {
                lock_rwlock_write(&self.in_memory_edges)
                    .entry(session_id.to_string())
                    .or_default()
                    .push(edge.clone());
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(super) fn in_memory_edges(&self, session_id: &str) -> Vec<EdgeRecord> {
        lock_rwlock_read(&self.in_memory_edges)
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub(super) fn clear_in_memory_edges(&self, session_id: &str) {
        lock_rwlock_write(&self.in_memory_edges).remove(session_id);
    }
}
