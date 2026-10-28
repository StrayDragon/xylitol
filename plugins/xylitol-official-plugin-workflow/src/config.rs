use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_REWORKFLOWS: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WorkflowConfig {
    pub enabled: bool,
    pub max_replans: u32,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_replans: DEFAULT_MAX_REWORKFLOWS,
        }
    }
}

impl WorkflowConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
