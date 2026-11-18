use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HooksConfig {
    pub global: Vec<HookEntry>,
    pub project: Vec<HookEntry>,
    pub user: Vec<HookEntry>,
    pub default_timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct HookEntry {
    pub command: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub fail_open: Option<bool>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "hooks",
        schema: || serde_json::to_value(schemars::schema_for!(HooksConfig)).expect("hooks schema"),
        default: || serde_json::to_value(HooksConfig::default()).expect("hooks default"),
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn hooks_section_declares_key_schema_default() {
        let section = config_section();
        assert_eq!(section.key, "hooks");
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(HooksConfig::default()).expect("hooks default")
        );
    }
}
