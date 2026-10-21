use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HooksConfig {
    pub global: Vec<HookEntry>,
    pub project: Vec<HookEntry>,
    pub user: Vec<HookEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct HookEntry {
    pub command: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}
