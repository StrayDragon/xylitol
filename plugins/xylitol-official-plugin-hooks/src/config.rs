use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use xylitol_plugin_sdk::config_section::SectionBinding;

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

impl SectionBinding for HooksConfig {
    const KEY: &'static str = "hooks";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(HooksConfig)).expect("hooks schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(HooksConfig::default()).expect("hooks default")
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <HooksConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn hooks_section_declares_key_schema_default() {
        assert_eq!(<HooksConfig as SectionBinding>::KEY, "hooks");
        let section = config_section();
        assert_eq!(section.key, <HooksConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(HooksConfig::default()).expect("hooks default")
        );
    }
}
