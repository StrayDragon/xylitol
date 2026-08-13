//! Model metadata and context-token estimates.

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::protocol::model::config::XyModelConfig;
use crate::protocol::model::thinking::ThinkingLevelMap;

/// Where a context-token estimate came from (c1030 accounting provenance).
///
/// Naming SSOT: the **wire form is snake_case** (serde), while
/// [`Self::as_str`] is the legacy PascalCase obs key. Both forms live on the
/// enum via strum (`IntoStaticStr` with per-variant PascalCase serializes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TokenProvenance {
    #[strum(serialize = "Api")]
    Api,
    #[strum(serialize = "RemoteCount")]
    RemoteCount,
    #[strum(serialize = "LocalTokenizer")]
    LocalTokenizer,
    #[strum(serialize = "Heuristic")]
    Heuristic,
    #[strum(serialize = "Unknown")]
    Unknown,
}

impl TokenProvenance {
    /// Stable string for logs / fastrace (`Api`, `LocalTokenizer`, …).
    ///
    /// Legacy PascalCase key — wire form is snake_case (`api`, `remote_count`…).
    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

/// Context occupancy estimate with provenance (XyDriver / compaction seam).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTokenEstimate {
    pub tokens: u64,
    pub provenance: TokenProvenance,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Metadata describing a model variant available from a provider.
#[derive(Debug, Clone)]
pub struct XyModelMeta {
    pub id: String,
    pub config: XyModelConfig,
    pub display_name: String,
    pub thinking: bool,
    pub context_window: u64,
    pub api: String,
    pub provider: String,
    pub cost_input: f64,
    pub cost_output: f64,
    pub cost_cache_read: f64,
    pub cost_cache_write: f64,
    pub max_tokens: u64,
    pub thinking_levels: Vec<String>,
    /// Optional per-level provider effort/budget overrides (`null` = omit).
    pub thinking_level_map: ThinkingLevelMap,
}
