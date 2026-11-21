use xylitol_plugin_sdk::CodemodeConfig;
use xylitol_plugin_sdk::config_section::SectionBinding;

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <CodemodeConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn codemode_section_declares_key_schema_default() {
        assert_eq!(<CodemodeConfig as SectionBinding>::KEY, "codemode");
        let section = config_section();
        assert_eq!(section.key, <CodemodeConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(CodemodeConfig::default()).expect("codemode default")
        );
    }
}
