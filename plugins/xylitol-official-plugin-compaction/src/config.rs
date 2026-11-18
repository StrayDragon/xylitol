pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "compaction",
        schema: || {
            serde_json::to_value(schemars::schema_for!(
                xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig
            ))
            .expect("compaction schema")
        },
        default: || serde_json::Value::Null,
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn compaction_section_declares_key_schema_and_null_default() {
        let section = config_section();
        assert_eq!(section.key, "compaction");
        assert!((section.schema)().is_object());
        assert!((section.default)().is_null());
    }
}
