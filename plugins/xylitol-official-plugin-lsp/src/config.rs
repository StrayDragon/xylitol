use crate::LspPluginConfig;

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    xylitol_plugin_sdk::config_section::ConfigSection {
        key: "lsp",
        schema: || {
            serde_json::to_value(schemars::schema_for!(LspPluginConfig)).expect("lsp schema")
        },
        default: || serde_json::Value::Null,
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn lsp_section_declares_key_and_null_default() {
        let section = config_section();
        assert_eq!(section.key, "lsp");
        assert!((section.schema)().is_object());
        assert!((section.default)().is_null());
    }

    #[test]
    fn lsp_config_defaults_apply_on_parse() {
        let config: LspPluginConfig = serde_json::from_str("{}").expect("lsp defaults");
        assert_eq!(config.backend, "basedpyright-langserver --stdio");
        assert_eq!(config.language, "python");
        assert_eq!(config.workspace_root, None);
        assert!(config.compression);
    }
}
