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
pub struct SelfHealConfig {
    pub enabled: bool,
    pub signatures: Vec<String>,
    pub max_attempts: u32,
}

pub const DEFAULT_SELF_HEAL_ATTEMPTS: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct GeneratorCriticConfig {
    pub enabled: bool,
    pub rubric: Vec<String>,
    pub max_rounds: u32,
}

pub const DEFAULT_CRITIC_ROUNDS: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WorkflowConfig {
    pub enabled: bool,
    pub max_replans: u32,
}

pub const DEFAULT_MAX_REWORKFLOWS: u32 = 3;

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_replans: DEFAULT_MAX_REWORKFLOWS,
        }
    }
}

impl Default for GeneratorCriticConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rubric: Vec::new(),
            max_rounds: DEFAULT_CRITIC_ROUNDS,
        }
    }
}

impl Default for SelfHealConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            signatures: Vec::new(),
            max_attempts: DEFAULT_SELF_HEAL_ATTEMPTS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LoopExampleConfig {
    pub enabled: bool,
    pub max_settles: Option<u32>,
    pub deny_bash_patterns: Vec<String>,
    pub todo_progress: bool,
    pub todo_reminder: bool,
    pub todo_reminder_stale_after: u32,
    pub todo_checkpoint: bool,
    pub plan_mode: bool,
    pub self_heal: SelfHealConfig,
    pub generator_critic: GeneratorCriticConfig,
    pub workflow: WorkflowConfig,
    pub tool_exposure: ToolExposureConfig,
}

pub const DEFAULT_TODO_REMINDER_STALE_AFTER: u32 = 3;

impl Default for LoopExampleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_settles: None,
            deny_bash_patterns: Vec::new(),
            todo_progress: false,
            todo_reminder: false,
            todo_reminder_stale_after: DEFAULT_TODO_REMINDER_STALE_AFTER,
            todo_checkpoint: false,
            plan_mode: true,
            self_heal: SelfHealConfig::default(),
            generator_critic: GeneratorCriticConfig::default(),
            workflow: WorkflowConfig::default(),
            tool_exposure: ToolExposureConfig::default(),
        }
    }
}

impl LoopExampleConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
