use crate::DapPluginConfig;
use xylitol_plugin_sdk::config_section::SectionBinding;

impl SectionBinding for DapPluginConfig {
    const KEY: &'static str = "dap";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(DapPluginConfig)).expect("dap schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::Value::Null
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <DapPluginConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn dap_section_declares_key_and_null_default() {
        assert_eq!(<DapPluginConfig as SectionBinding>::KEY, "dap");
        let section = config_section();
        assert_eq!(section.key, <DapPluginConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert!((section.default)().is_null());
    }

    #[test]
    fn dap_config_defaults_apply_on_parse() {
        let config: DapPluginConfig = serde_json::from_str("{}").expect("dap defaults");
        assert_eq!(config.backend, "python3 -m debugpy.adapter");
        assert_eq!(config.cwd, None);
        assert!(config.compression);
    }
}
