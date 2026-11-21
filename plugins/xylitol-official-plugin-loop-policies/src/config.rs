use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use xylitol_plugin_sdk::config_section::SectionBinding;

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LoopPoliciesConfig {
    pub enabled: bool,
    pub max_settles: Option<u32>,
    pub deny_bash_patterns: Vec<String>,
    pub self_heal: SelfHealConfig,
    pub generator_critic: GeneratorCriticConfig,
    pub tool_exposure: ToolExposureConfig,
}

impl LoopPoliciesConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

impl SectionBinding for LoopPoliciesConfig {
    const KEY: &'static str = "loop_policies";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(LoopPoliciesConfig))
            .expect("loop_policies schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(LoopPoliciesConfig::default()).expect("loop_policies default")
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <LoopPoliciesConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn loop_policies_section_declares_key_schema_default() {
        assert_eq!(<LoopPoliciesConfig as SectionBinding>::KEY, "loop_policies");
        let section = config_section();
        assert_eq!(section.key, <LoopPoliciesConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(LoopPoliciesConfig::default()).expect("loop_policies default")
        );
    }
}
