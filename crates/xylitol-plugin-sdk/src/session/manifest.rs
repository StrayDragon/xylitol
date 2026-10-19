//! Session manifest and segment metadata.

use serde::{Deserialize, Serialize};

use super::entries::SESSION_VERSION;

/// A manifest reference to one immutable or active JSONL segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSegment {
    /// Relative path from the session directory.
    pub path: String,
    pub generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_entry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_entry_id: Option<String>,
    #[serde(default)]
    pub includes_header: bool,
    /// Optional immutable sidecar index path for sealed (cold) segments,
    /// relative to the session directory. Absent on old manifests; present
    /// only for sealed segments sealed by a v7.1+ writer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_path: Option<String>,
}

/// Immutable sidecar index for one sealed segment (v7.1+).
///
/// Pure derived data of the sealed JSONL: `entry_ids` carries every non-empty
/// entry id in the segment, `done_bash_ids` every done non-empty bash id. It is
/// only an optimization — a missing or corrupt sidecar MUST fall back to
/// scanning the sealed segment (no false negatives).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedIndex {
    #[serde(default)]
    pub entry_ids: Vec<String>,
    #[serde(default)]
    pub done_bash_ids: Vec<String>,
}

impl SealedIndex {
    /// Build the index from the exact entries being sealed (stable output).
    ///
    /// Entry ids are the non-empty `entry_id()` of each entry in order; done
    /// bash ids reuse the protocol-level bash parse (`done_bash_ids`), so the
    /// sidecar and the logical scan agree on what counts as done.
    pub fn from_entries(entries: &[super::SessionEntry]) -> Self {
        let mut entry_ids: Vec<String> = entries
            .iter()
            .filter_map(|e| e.entry_id().filter(|id| !id.is_empty()).map(str::to_owned))
            .collect();
        entry_ids.sort();
        entry_ids.dedup();
        let mut done_bash = super::done_bash_ids(entries)
            .into_iter()
            .collect::<Vec<_>>();
        done_bash.sort();
        Self {
            entry_ids,
            done_bash_ids: done_bash,
        }
    }

    /// Whether `entry_id` is a candidate in this segment (index-based screen).
    ///
    /// Returns `false` only when this index is intact AND reports absence; the
    /// caller MUST re-scan when the index is missing/corrupt/unusable. Linear
    /// probe keeps correctness even for an unsorted on-disk index.
    pub fn contains_entry(&self, entry_id: &str) -> bool {
        self.entry_ids.iter().any(|id| id == entry_id)
    }
}

/// The commit pointer for one session.
///
/// The manifest intentionally contains references and lightweight bounds only;
/// session facts remain in the referenced JSONL segments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionManifest {
    pub format_version: u32,
    pub session_id: String,
    pub active_segment: SessionSegment,
    #[serde(default)]
    pub sealed_segments: Vec<SessionSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaf_entry_id: Option<String>,
}

impl SessionManifest {
    pub fn new(session_id: impl Into<String>, active_segment: SessionSegment) -> Self {
        Self {
            format_version: SESSION_VERSION,
            session_id: session_id.into(),
            active_segment,
            sealed_segments: Vec::new(),
            leaf_entry_id: None,
        }
    }
}
