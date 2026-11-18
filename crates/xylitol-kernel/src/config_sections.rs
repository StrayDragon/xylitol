use xylitol_plugin_sdk::config_section::ConfigSection;

pub fn all() -> Vec<ConfigSection> {
    let mut sections: Vec<ConfigSection> =
        xylitol_official_plugin_config::sections::core_sections().to_vec();
    sections.extend([
        xylitol_official_plugin_hooks::config::config_section(),
        xylitol_official_plugin_loop_policies::config::config_section(),
        xylitol_official_plugin_todo::config::config_section(),
        xylitol_official_plugin_workflow::config::config_section(),
        xylitol_official_plugin_mcp::config::config_section(),
        xylitol_official_plugin_lsp::config_section(),
        xylitol_official_plugin_dap::config_section(),
    ]);
    debug_assert!(
        {
            let mut keys: Vec<&str> = sections.iter().map(|s| s.key).collect();
            keys.sort_unstable();
            keys.dedup();
            keys.len() == sections.len()
        },
        "config section keys must be unique"
    );
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_plugin_section_defaults_are_stable() {
        let sections = all();
        let todo = sections
            .iter()
            .find(|section| section.key == "todo")
            .expect("todo section declared");
        assert_eq!(
            (todo.default)()["enabled"],
            serde_json::json!(true),
            "todo plugin is default-on"
        );
        let workflow = sections
            .iter()
            .find(|section| section.key == "workflow")
            .expect("workflow section declared");
        assert_eq!(
            (workflow.default)()["enabled"],
            serde_json::json!(false),
            "workflow plugin is opt-in"
        );
    }

    #[test]
    fn checked_in_schema_matches_aggregated_registry() {
        let expected = include_str!("../../../configs/config.schema.json");
        assert_eq!(
            expected,
            xylitol_official_plugin_config::validate::render_app_config_schema(&all())
        );
    }
}
