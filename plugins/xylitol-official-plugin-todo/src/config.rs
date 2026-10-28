use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TODO_REMINDER_STALE_AFTER: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TodoConfig {
    pub enabled: bool,
    pub progress: bool,
    pub reminder: bool,
    pub reminder_stale_after: u32,
    pub checkpoint: bool,
    pub max_settles: Option<u32>,
}

impl Default for TodoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            progress: false,
            reminder: false,
            reminder_stale_after: DEFAULT_TODO_REMINDER_STALE_AFTER,
            checkpoint: false,
            max_settles: None,
        }
    }
}

impl TodoConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
