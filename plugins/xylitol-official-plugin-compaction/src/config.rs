use xylitol_plugin_sdk::compaction_config::XyCompactionSettingsConfig;
use xylitol_plugin_sdk::config_section::SectionBinding;

pub fn config_section() -> xylitol_plugin_sdk::config_section::ConfigSection {
    <XyCompactionSettingsConfig as SectionBinding>::descriptor()
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn compaction_section_declares_key_schema_and_null_default() {
        assert_eq!(
            <XyCompactionSettingsConfig as SectionBinding>::KEY,
            "compaction"
        );
        let section = config_section();
        assert_eq!(
            section.key,
            <XyCompactionSettingsConfig as SectionBinding>::KEY
        );
        assert!((section.schema)().is_object());
        assert!((section.default)().is_null());
    }
}
