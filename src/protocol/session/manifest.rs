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
