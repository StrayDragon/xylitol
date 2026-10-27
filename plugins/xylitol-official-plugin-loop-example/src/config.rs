use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LoopExampleConfig {
    pub enabled: bool,
    pub max_settles: Option<u32>,
    pub deny_bash_patterns: Vec<String>,
}

impl LoopExampleConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
