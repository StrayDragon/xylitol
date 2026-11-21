use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use xylitol_plugin_sdk::config_section::SectionBinding;

pub const DEFAULT_MAX_REWORKFLOWS: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct WorkflowConfig {
    pub enabled: bool,
    pub max_replans: u32,
    pub max_depth: u32,
    pub max_total_steps: usize,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_replans: DEFAULT_MAX_REWORKFLOWS,
            max_depth: xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            max_total_steps: xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
        }
    }
}

impl WorkflowConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

impl SectionBinding for WorkflowConfig {
    const KEY: &'static str = "workflow";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(WorkflowConfig)).expect("workflow schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(WorkflowConfig::default()).expect("workflow default")
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <WorkflowConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn workflow_section_declares_key_schema_default() {
        assert_eq!(<WorkflowConfig as SectionBinding>::KEY, "workflow");
        let section = config_section();
        assert_eq!(section.key, <WorkflowConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(WorkflowConfig::default()).expect("workflow default")
        );
    }
}
