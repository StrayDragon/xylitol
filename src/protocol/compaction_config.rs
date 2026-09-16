//! Compaction configuration vocabulary — file-loaded + runtime representations.
//!
//! Pure serde types shared by `agent::compaction` (runtime settings conversion)
//! and `infra::{config, settings}` (file loading). Zero crate-internal deps.
//!
//! JSON Schema is derived on this type in place (c2720, landing c510's
//! single-field-table intent): infra config/settings DTOs reference it
//! directly, with no hand-written schema twin.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::protocol::model_entry::XyModelEntryConfig;

/// Compaction settings as loaded from YAML/JSON files (all-optional form).
///
/// Mapped to the runtime `CompactionSettings` (in `agent::compaction::settings`)
/// via `From<XyCompactionSettingsConfig>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct XyCompactionSettingsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<XyModelEntryConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
}
