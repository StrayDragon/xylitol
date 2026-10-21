use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model_entry::XyModelEntryConfig;

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
