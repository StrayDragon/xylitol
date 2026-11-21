use crate::LspPluginConfig;
use xylitol_plugin_sdk::config_section::SectionBinding;

impl SectionBinding for LspPluginConfig {
    const KEY: &'static str = "lsp";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(LspPluginConfig)).expect("lsp schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::Value::Null
    }
}

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <LspPluginConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn lsp_section_declares_key_and_null_default() {
        assert_eq!(<LspPluginConfig as SectionBinding>::KEY, "lsp");
        let section = config_section();
        assert_eq!(section.key, <LspPluginConfig as SectionBinding>::KEY);
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
