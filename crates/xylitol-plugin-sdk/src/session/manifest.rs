use serde::{Deserialize, Serialize};

use super::entries::SESSION_VERSION;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSegment {
    pub path: String,
    pub generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_entry_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_entry_id: Option<String>,
    #[serde(default)]
    pub includes_header: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedIndex {
    #[serde(default)]
    pub entry_ids: Vec<String>,
    #[serde(default)]
    pub done_bash_ids: Vec<String>,
}

impl SealedIndex {
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

    pub fn contains_entry(&self, entry_id: &str) -> bool {
        self.entry_ids.iter().any(|id| id == entry_id)
    }
}

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
