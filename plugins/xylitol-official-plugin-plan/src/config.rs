use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct PlanConfig {
    pub enabled: bool,
    pub read_only: bool,
    pub allowed_tools: Vec<String>,
}

impl Default for PlanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            read_only: true,
            allowed_tools: Vec::new(),
        }
    }
}

impl PlanConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
