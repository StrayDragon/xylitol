use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub(crate) const LOG_FORMAT: &str = "log-v1";
pub(crate) const ANCHOR_EVERY: u64 = 64;
pub(crate) const INITIAL_THRESHOLD_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MIN_THRESHOLD_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_THRESHOLD_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RotationPolicy {
    pub(crate) policy: String,
    pub(crate) threshold_bytes: u64,
    pub(crate) min_bytes: u64,
    pub(crate) max_bytes: u64,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            policy: "adaptive-turn-aligned".into(),
            threshold_bytes: INITIAL_THRESHOLD_BYTES,
            min_bytes: MIN_THRESHOLD_BYTES,
            max_bytes: MAX_THRESHOLD_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexState {
    pub(crate) anchor_every: u64,
    pub(crate) built_at: u64,
    pub(crate) files: Vec<u32>,
}

impl Default for IndexState {
    fn default() -> Self {
        Self {
            anchor_every: ANCHOR_EVERY,
            built_at: 0,
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Manifest {
    pub(crate) format: String,
    pub(crate) session_id: String,
    pub(crate) active_file: u32,
    pub(crate) tail_bytes: u64,
    pub(crate) line_count: u64,
    pub(crate) next_seq: u64,
    pub(crate) leaf_id: Option<String>,
    pub(crate) rotation: RotationPolicy,
    pub(crate) index: IndexState,
    pub(crate) updated_at: u64,
}

pub(crate) struct CachedManifest {
    pub(crate) manifest: Manifest,
    pub(crate) dirty: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Layout {
    pub(crate) sessions_dir: PathBuf,
}

impl Layout {
    pub(crate) fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    pub(crate) fn legacy_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{session_id}.jsonl"))
    }

    pub(crate) fn log_dir(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("log")
    }

    pub(crate) fn log_file(&self, session_id: &str, file: u32) -> PathBuf {
        self.log_dir(session_id).join(format!("{file:06}.jsonl"))
    }

    pub(crate) fn manifest_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("manifest.json")
    }

    pub(crate) fn index_dir(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("index")
    }

    pub(crate) fn index_path(&self, session_id: &str, file: u32) -> PathBuf {
        self.index_dir(session_id).join(format!("{file:06}.idx"))
    }

    pub(crate) fn graph_path(&self, session_id: &str) -> PathBuf {
        self.session_dir(session_id).join("graph.jsonl")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Anchor {
    pub(crate) seq: u64,
    pub(crate) entry_id: String,
    pub(crate) offset: u64,
}
