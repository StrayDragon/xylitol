use crate::DapPluginConfig;

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "dap",
        schema: || {
            serde_json::to_value(schemars::schema_for!(DapPluginConfig)).expect("dap schema")
        },
        default: || serde_json::Value::Null,
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn dap_section_declares_key_and_null_default() {
        let section = config_section();
        assert_eq!(section.key, "dap");
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
