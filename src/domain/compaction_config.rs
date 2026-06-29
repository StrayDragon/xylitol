//! Compaction configuration vocabulary — file-loaded + runtime representations.
//!
//! Pure serde types shared by `agent::compaction` (runtime settings conversion)
//! and `infra::{config, settings}` (file loading). Zero crate-internal deps.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Compaction settings as loaded from YAML/JSON files (all-optional form).
///
/// Mapped to the runtime `CompactionSettings` (in `agent::compaction::settings`)
/// via `From<XyCompactionSettingsConfig>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct XyCompactionSettingsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
}
