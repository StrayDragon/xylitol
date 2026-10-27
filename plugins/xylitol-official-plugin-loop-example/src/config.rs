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
    pub plan_state: bool,
    pub progress_reminder: bool,
    pub progress_reminder_stale_after: u32,
    pub plan_checkpoint: bool,
    pub self_heal: SelfHealConfig,
    pub generator_critic: GeneratorCriticConfig,
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
            plan_checkpoint: false,
            self_heal: SelfHealConfig::default(),
            generator_critic: GeneratorCriticConfig::default(),
            tool_exposure: ToolExposureConfig::default(),
        }
    }
}

impl LoopExampleConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
