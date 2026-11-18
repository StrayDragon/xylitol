pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "codemode",
        schema: || {
            serde_json::to_value(schemars::schema_for!(xylitol_plugin_sdk::CodemodeConfig))
                .expect("codemode schema")
        },
        default: || {
            serde_json::to_value(xylitol_plugin_sdk::CodemodeConfig::default())
                .expect("codemode default")
        },
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn codemode_section_declares_key_schema_default() {
        let section = config_section();
        assert_eq!(section.key, "codemode");
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(xylitol_plugin_sdk::CodemodeConfig::default())
                .expect("codemode default")
        );
    }
}
