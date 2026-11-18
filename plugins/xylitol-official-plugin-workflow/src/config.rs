use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "workflow",
        schema: || {
            serde_json::to_value(schemars::schema_for!(WorkflowConfig)).expect("workflow schema")
        },
        default: || serde_json::to_value(WorkflowConfig::default()).expect("workflow default"),
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn workflow_section_declares_key_schema_default() {
        let section = config_section();
        assert_eq!(section.key, "workflow");
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(WorkflowConfig::default()).expect("workflow default")
        );
    }
}
