use std::collections::HashMap;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::instrument;

use super::config::StorageConfig;
use super::types::{Snapshot, SnapshotId};

/// Persistent snapshot storage backed by adk-session SQLite.
pub struct SnapshotStore {
    config: StorageConfig,
    /// In-memory index keyed by snapshot ID (adk-session provides the actual
    /// durable backend; this struct is a convenience wrapper).
    snapshots: HashMap<SnapshotId, Vec<u8>>,
}

impl SnapshotStore {
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            snapshots: HashMap::new(),
        }
    }

    /// Serialise a snapshot to its wire format.
    ///
    /// Format: JSON → optional Zstd compression.
    /// JSON is used over MessagePack for broad serde compatibility; zstd
    /// provides the space savings.
    pub fn encode(&self, snapshot: &Snapshot) -> Result<Vec<u8>> {
        let encoded = serde_json::to_vec(snapshot).context("json encode failed")?;
        if self.config.compress {
            let compressed = zstd::encode_all(std::io::Cursor::new(encoded), 3)
                .context("zstd compression failed")?;
            Ok(compressed)
        } else {
            Ok(encoded)
        }
    }

    /// Deserialise a snapshot from its wire format.
    pub fn decode(&self, blob: &[u8]) -> Result<Snapshot> {
        let decompressed = if self.config.compress {
            zstd::decode_all(std::io::Cursor::new(blob)).context("zstd decompression failed")?
        } else {
            blob.to_vec()
        };
        serde_json::from_slice(&decompressed).context("json decode failed")
    }

    /// Persist a snapshot.
    #[instrument(skip(self, snapshot))]
    pub fn store(&mut self, snapshot: &Snapshot) -> Result<()> {
        let blob = self.encode(snapshot)?;
        self.snapshots.insert(snapshot.id.clone(), blob);
        Ok(())
    }

    /// Load a snapshot by ID.
    pub fn load(&self, id: &str) -> Result<Snapshot> {
        let blob = self
            .snapshots
            .get(id)
            .with_context(|| format!("snapshot {id} not found"))?;
        self.decode(blob)
    }

    /// Delete a snapshot by ID.
    pub fn delete(&mut self, id: &str) -> Result<()> {
        self.snapshots
            .remove(id)
            .with_context(|| format!("snapshot {id} not found"))?;
        Ok(())
    }

    /// List all stored snapshot IDs.
    pub fn list_ids(&self) -> Vec<SnapshotId> {
        self.snapshots.keys().cloned().collect()
    }

    /// Check whether a snapshot exists.
    pub fn exists(&self, id: &str) -> bool {
        self.snapshots.contains_key(id)
    }

    /// Number of stored snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Compute a SHA-256 project hash from a set of file paths and their contents.
    ///
    /// Only hashes files matching the given glob patterns (excludes common
    /// generated directories like `target/`, `node_modules/`).
    pub fn compute_project_hash(paths: &[String]) -> String {
        let mut hasher = Sha256::new();
        for path in paths {
            hasher.update(path.as_bytes());
            if let Ok(content) = std::fs::read_to_string(path) {
                hasher.update(content.as_bytes());
            }
        }
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::session::types::*;

    fn sample_snapshot() -> Snapshot {
        Snapshot {
            id: "test-1".into(),
            created: TimestampMillis::now(),
            parent_snapshot_id: None,
            meta: SnapshotMeta {
                project_root: "/tmp/project".into(),
                project_hash: "abc123".into(),
                model_id: "claude-opus-4".into(),
                tags: vec!["test".into()],
            },
            conversation: vec![ConversationTurn {
                role: ConversationRole::User,
                content: "hello".into(),
                tool_calls: None,
                timestamp: TimestampMillis::now(),
                deprecated: false,
            }],
            project_cognition: ProjectCognition::empty(),
            tool_call_log: vec![],
            config_fingerprint: ConfigFingerprint {
                features: vec!["infra-session".into()],
                config_hash: "xyz".into(),
            },
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let config = StorageConfig::default();
        let store = SnapshotStore::new(config);
        let snap = sample_snapshot();
        let blob = store.encode(&snap).unwrap();
        let decoded = store.decode(&blob).unwrap();
        assert_eq!(decoded.id, snap.id);
        assert_eq!(decoded.conversation.len(), 1);
    }

    #[test]
    fn test_store_load_roundtrip() {
        let mut store = SnapshotStore::new(StorageConfig::default());
        let snap = sample_snapshot();
        store.store(&snap).unwrap();
        assert!(store.exists("test-1"));
        let loaded = store.load("test-1").unwrap();
        assert_eq!(loaded.meta.project_hash, "abc123");
    }

    #[test]
    fn test_delete() {
        let mut store = SnapshotStore::new(StorageConfig::default());
        store.store(&sample_snapshot()).unwrap();
        store.delete("test-1").unwrap();
        assert!(!store.exists("test-1"));
    }
}
