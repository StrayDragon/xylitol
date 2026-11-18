use xylitol_plugin_sdk::config_section::ConfigSection;

pub fn all() -> Vec<ConfigSection> {
    let mut sections: Vec<ConfigSection> =
        xylitol_official_plugin_config::sections::core_sections().to_vec();
    sections.extend([
        xylitol_official_plugin_compaction::config::config_section(),
        xylitol_official_plugin_codemode::config::config_section(),
        xylitol_official_plugin_context_inject::config::config_section(),
        xylitol_official_plugin_obs::config::config_section(),
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

    fn load_with_sections(yaml: &str) -> xylitol_official_plugin_config::types::AppConfig {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(global.join("config.yaml"), yaml).unwrap();
        let home_s = home.path().to_string_lossy().into_owned();
        let global_s = global.to_string_lossy().into_owned();
        let entries = [
            ("HOME", home_s),
            ("XYLITOL_CONFIG_DIR", global_s),
            (
                "XYLITOL_PROJECT_DIR",
                home.path().to_string_lossy().into_owned(),
            ),
        ];
        let env = move |key: &str| {
            entries
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.clone())
        };
        xylitol_official_plugin_config::loader::load_app_config_with(None, env, None, &all())
            .expect("load")
    }

    #[test]
    fn codemode_section_defaults_and_rejects_unknown_runtime() {
        let cfg = load_with_sections("codemode:\n  enabled: true\n  fuel: 42\n");
        let codemode: xylitol_plugin_sdk::CodemodeConfig =
            cfg.section("codemode").expect("section").expect("present");
        assert!(codemode.enabled);
        assert_eq!(codemode.fuel, 42);
        assert_eq!(codemode.max_nested_calls, 100);

        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(
            global.join("config.yaml"),
            "codemode:\n  runtime: wasmtime2\n",
        )
        .unwrap();
        let entries = [
            ("HOME", home.path().to_string_lossy().into_owned()),
            ("XYLITOL_CONFIG_DIR", global.to_string_lossy().into_owned()),
            (
                "XYLITOL_PROJECT_DIR",
                home.path().to_string_lossy().into_owned(),
            ),
        ];
        let err = xylitol_official_plugin_config::loader::load_app_config_with(
            None,
            |key: &str| {
                entries
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, value)| value.clone())
            },
            None,
            &all(),
        )
        .expect_err("runtime");
        assert!(err.to_string().contains("codemode"), "got: {err}");
    }

    #[test]
    fn otel_config_otlp_http_parses() {
        let cfg = load_with_sections(
            "otel:\n  exporter: otlp-http\n  endpoint: \"http://127.0.0.1:3000/api/public/otel\"\n  protocol: http-binary\n  environment: dev\n  service_name: XYlitol\n",
        );
        let otel: xylitol_official_plugin_obs::config::OtelConfig =
            cfg.section("otel").expect("section").expect("present");
        assert!(otel.wants_otlp_http());
        assert_eq!(
            otel.endpoint.as_deref(),
            Some("http://127.0.0.1:3000/api/public/otel")
        );
        assert_eq!(
            otel.protocol,
            xylitol_official_plugin_obs::config::OtelHttpProtocol::HttpBinary
        );
        assert_eq!(otel.environment.as_deref(), Some("dev"));
        assert_eq!(otel.service_name.as_deref(), Some("XYlitol"));
        assert_eq!(otel.export_timeout_secs, 60);
    }

    #[test]
    fn otel_section_rejects_unknown_exporter() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(global.join("config.yaml"), "otel:\n  exporter: grpc\n").unwrap();
        let entries = [
            ("HOME", home.path().to_string_lossy().into_owned()),
            ("XYLITOL_CONFIG_DIR", global.to_string_lossy().into_owned()),
            (
                "XYLITOL_PROJECT_DIR",
                home.path().to_string_lossy().into_owned(),
            ),
        ];
        let err = xylitol_official_plugin_config::loader::load_app_config_with(
            None,
            |key: &str| {
                entries
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, value)| value.clone())
            },
            None,
            &all(),
        )
        .expect_err("unknown exporter");
        assert!(err.to_string().contains("otel"), "got: {err}");
    }

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
