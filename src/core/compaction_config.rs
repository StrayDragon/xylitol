//! Compaction configuration vocabulary — load-time + settings representations.
//!
//! Pure serde types shared by `agent::compaction` (runtime settings conversion)
//! and `infra::{config, settings}` (file loading). Zero crate-internal deps.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Compaction configuration from YAML — load-time representation.
///
/// Mapped to the runtime `CompactionSettings` (in `agent::compaction::settings`)
/// via `From<CompactionConfig>`. Reserve tokens and keep-recent thresholds are
/// hardcoded in the runtime defaults.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct CompactionConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Settings controlling compaction behavior (file-loaded, all-optional form).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CompactionSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
}
