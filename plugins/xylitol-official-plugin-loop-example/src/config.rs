use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ToolExposureConfig {
    pub enabled: bool,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub read_only: bool,
    pub write_tools: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LoopExampleConfig {
    pub enabled: bool,
    pub max_settles: Option<u32>,
    pub deny_bash_patterns: Vec<String>,
    pub plan_state: bool,
    pub progress_reminder: bool,
    pub progress_reminder_stale_after: u32,
    pub tool_exposure: ToolExposureConfig,
}

pub const DEFAULT_PROGRESS_STALE_AFTER: u32 = 3;

impl Default for LoopExampleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_settles: None,
            deny_bash_patterns: Vec::new(),
            plan_state: false,
            progress_reminder: false,
            progress_reminder_stale_after: DEFAULT_PROGRESS_STALE_AFTER,
            tool_exposure: ToolExposureConfig::default(),
        }
    }
}

impl LoopExampleConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
