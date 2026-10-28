use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct PlanConfig {
    pub enabled: bool,
    pub read_only: bool,
}

impl Default for PlanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            read_only: true,
        }
    }
}

impl PlanConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
