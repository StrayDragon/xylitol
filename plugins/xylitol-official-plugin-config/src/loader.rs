use std::path::Path;

use serde_json::Value;

use super::error::LoadError;
use super::paths::ConfigPaths;
use super::secret_env::SecretMap;
use super::types::AppConfig;

#[derive(Debug, Clone)]
pub struct LoadedAppConfig {
    pub config: AppConfig,
    pub from_yaml_layers: bool,
}

pub fn load_app_config(cli_config: Option<&Path>) -> Result<AppConfig, LoadError> {
    Ok(load_app_config_detailed(cli_config)?.config)
}

pub fn load_app_config_detailed(cli_config: Option<&Path>) -> Result<LoadedAppConfig, LoadError> {
    load_app_config_detailed_with(
        cli_config,
        |k| {
            if let Ok(v) = std::env::var(k) {
                return Some(v);
            }
            if k == "HOME" {
                return dirs::home_dir().map(|p| p.to_string_lossy().into_owned());
            }
            None
        },
        std::env::current_dir().ok().as_deref(),
    )
}

pub fn load_app_config_with(
    cli_config: Option<&Path>,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<AppConfig, LoadError> {
    Ok(load_app_config_detailed_with(cli_config, get_env, cwd)?.config)
}

pub fn load_app_config_detailed_with(
    cli_config: Option<&Path>,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<LoadedAppConfig, LoadError> {
    let paths = ConfigPaths::discover_with(get_env, cwd);
    load_from_paths(&paths, cli_config)
}

fn load_from_paths(
    paths: &ConfigPaths,
    cli_config: Option<&Path>,
) -> Result<LoadedAppConfig, LoadError> {
    let (secrets, injected) = super::secret_env::load_secret_env_files(paths);
    if injected > 0 {
        log::debug!(
            target: "xylitol::config",
            "loaded secret.env into process environment injected={injected}"
        );
    }

    let mut merged = Value::Null;
    let mut from_yaml_layers = false;

    if let Some(global_base) = first_existing(&[
        paths.global_dir.join("config.yaml"),
        paths.global_dir.join("config.yml"),
    ]) {
        log::info!(
            target: "xylitol::config",
            "loading global config {}",
            global_base.display()
        );
        let val = load_and_render(&global_base, &secrets)?;
        deep_merge(&mut merged, val);
        from_yaml_layers = true;
    }

    if let Some(ref proj_dir) = paths.project_dir
        && let Some(proj_base) =
            first_existing(&[proj_dir.join("config.yaml"), proj_dir.join("config.yml")])
    {
        let val = load_and_render(&proj_base, &secrets)?;
        deep_merge(&mut merged, val);
        from_yaml_layers = true;
    }

    if let Some(cli_path) = cli_config
        && cli_path.exists()
    {
        let val = load_and_render(cli_path, &secrets)?;
        deep_merge(&mut merged, val);
        from_yaml_layers = true;
    }

    if merged.is_null() {
        return Ok(LoadedAppConfig {
            config: AppConfig::default(),
            from_yaml_layers: false,
        });
    }

    reject_legacy_loop_example_section(&merged)?;
    reject_legacy_loop_policies_keys(&merged)?;
    let mut config: AppConfig = serde_json::from_value(merged.clone())?;
    super::validate::validate_merged_config(&merged)?;
    config.validate_thinking_levels()?;
    config.validate_session_max_turns()?;
    expand_model_config_expressions(&mut config)?;
    Ok(LoadedAppConfig {
        config,
        from_yaml_layers,
    })
}

fn reject_legacy_loop_example_section(merged: &Value) -> Result<(), LoadError> {
    if merged.get("loop_example").is_some() {
        return Err(LoadError::validation(
            "loop_example is no longer supported; use `loop_policies`; \
             todo/workflow/plan keys moved to top-level `todo`/`workflow`/`plan` \
             (see configs/example.yaml)",
        ));
    }
    Ok(())
}

fn reject_legacy_loop_policies_keys(merged: &Value) -> Result<(), LoadError> {
    let Some(loop_policies) = merged.get("loop_policies").and_then(Value::as_object) else {
        return Ok(());
    };
    let legacy = [
        ("todo_progress", "todo.progress"),
        ("todo_reminder", "todo.reminder"),
        ("todo_reminder_stale_after", "todo.reminder_stale_after"),
        ("todo_checkpoint", "todo.checkpoint"),
        ("plan_mode", "plan.enabled"),
        ("workflow", "workflow"),
    ];
    for (old, new) in legacy {
        if loop_policies.contains_key(old) {
            return Err(LoadError::validation(format!(
                "loop_policies.{old} is no longer supported; use `{new}` (see configs/example.yaml)"
            )));
        }
    }
    Ok(())
}

fn expand_model_config_expressions(config: &mut AppConfig) -> Result<(), LoadError> {
    for (alias, entry) in config.model.models.iter_mut() {
        if let Some(v) = entry.api_key.as_mut() {
            *v = expand_config_value(alias, "api_key", v)?;
        }
        let model_raw = entry.model.clone();
        entry.model = expand_config_value(alias, "model", &model_raw)?;
        if let Some(v) = entry.base_url.as_mut() {
            *v = expand_config_value(alias, "base_url", v)?;
        }
        if let Some(v) = entry.api.as_mut() {
            *v = expand_config_value(alias, "api", v)?;
        }
        if let Some(v) = entry.compat.as_mut() {
            *v = expand_config_value(alias, "compat", v)?;
        }
    }
    Ok(())
}

fn expand_config_value(alias: &str, field: &str, raw: &str) -> Result<String, LoadError> {
    let resolved = super::resolver::resolve_from_env(raw)
        .map_err(|e| LoadError::validation(format!("models.{alias}.{field}: {e}")))?;
    Ok(resolved)
}

fn first_existing(candidates: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    candidates.iter().find(|p| p.is_file()).cloned()
}

fn load_and_render(path: &Path, secrets: &SecretMap) -> Result<Value, LoadError> {
    let raw = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        path: path.to_string_lossy().to_string(),
        source: e,
    })?;

    let rendered = super::template::render_config_template(&raw, path, secrets)?;

    let value: Value = yaml_serde::from_str(&rendered).map_err(|e| LoadError::Yaml {
        path: path.to_string_lossy().to_string(),
        source: e,
    })?;

    Ok(value)
}

pub(crate) fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (base @ &mut Value::Object(_), Value::Object(map)) => {
            let base_map = base
                .as_object_mut()
                .expect("matched &mut Value::Object guard");
            for (k, v) in map {
                if let Some(existing) = base_map.get_mut(&k) {
                    deep_merge(existing, v);
                } else {
                    base_map.insert(k, v);
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use std::collections::HashMap;

    fn env_map<'a>(entries: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, &str> = entries.iter().copied().collect();
        move |k| map.get(k).map(|v| (*v).to_string())
    }

    #[test]
    fn test_deep_merge_object() {
        let mut base = json!({
            "a": 1,
            "b": { "c": 2, "d": 3 },
        });
        let overlay = json!({
            "b": { "d": 99, "e": 4 },
            "f": 5,
        });
        deep_merge(&mut base, overlay);
        assert_eq!(base["a"], json!(1));
        assert_eq!(base["b"]["c"], json!(2));
        assert_eq!(base["b"]["d"], json!(99));
        assert_eq!(base["b"]["e"], json!(4));
        assert_eq!(base["f"], json!(5));
    }

    #[test]
    fn test_deep_merge_array_replaces() {
        let mut base = json!(["a", "b"]);
        let overlay = json!(["c"]);
        deep_merge(&mut base, overlay);
        assert_eq!(base, json!(["c"]));
    }

    #[test]
    fn test_deep_merge_scalar_replaces() {
        let mut base = json!("old");
        let overlay = json!("new");
        deep_merge(&mut base, overlay);
        assert_eq!(base, json!("new"));
    }

    #[test]
    fn test_deep_merge_null_base() {
        let mut base = Value::Null;
        let overlay = json!({"key": "val"});
        deep_merge(&mut base, overlay);
        assert_eq!(base["key"], json!("val"));
    }

    #[test]
    #[serial(env_global)]
    fn loads_yml_alias_and_renders_mcp_secret_headers() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(global.join("secret.env"), "CTX_KEY=secret-value\n").unwrap();
        std::fs::write(
            global.join("config.yml"),
            "mcp_servers:\n  - name: demo\n    transport: sse\n    url: https://example.com/mcp\n    headers:\n      CONTEXT7_API_KEY: \"{{ secret.CTX_KEY }}\"\n",
        )
        .unwrap();
        let home_s = home.path().to_str().unwrap();
        let global_s = global.to_str().unwrap();
        let entries = [
            ("HOME", home_s),
            ("XYLITOL_CONFIG_DIR", global_s),
            ("XYLITOL_PROJECT_DIR", home_s),
        ];
        let cfg = load_app_config_with(None, env_map(&entries), None).expect("load");
        let servers = cfg.mcp_servers.expect("mcp_servers");
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "demo");
        let headers = servers[0].headers.as_ref().expect("headers");
        assert_eq!(
            headers.get("CONTEXT7_API_KEY").map(String::as_str),
            Some("secret-value")
        );
    }

    #[test]
    fn compaction_model_inline_entry_parses() {
        use crate::types::AppConfig;

        let cfg: AppConfig = yaml_serde::from_str(
            r"
models: {}
compaction:
  enabled: true
  model:
    provider: fake
    model: inline-summary
  thinkingLevel: high
",
        )
        .expect("inline compaction model");
        let comp = cfg.compaction.expect("compaction section");
        let model = comp.model.expect("task model entry");
        assert_eq!(model.model, "inline-summary");
        assert_eq!(comp.thinking_level.as_deref(), Some("high"));
    }

    #[test]
    fn compaction_model_yaml_anchor_snake_case_fields() {
        use crate::types::AppConfig;

        let cfg: AppConfig = yaml_serde::from_str(
            r"
models:
  models:
    shared: &shared
      provider: fake
      model: anchor-wire
      base_url: https://summary.example/v1
      api_key: sk-anchor
compaction:
  model: *shared
",
        )
        .expect("snake_case anchor compaction model");
        let model = cfg.compaction.expect("compaction").model.expect("model");
        assert_eq!(model.model, "anchor-wire");
        assert_eq!(
            model.base_url.as_deref(),
            Some("https://summary.example/v1")
        );
        assert_eq!(model.api_key.as_deref(), Some("sk-anchor"));
    }

    #[test]
    fn compaction_model_yaml_anchor_alias() {
        use crate::types::AppConfig;

        let cfg: AppConfig = yaml_serde::from_str(
            r"
models:
  models:
    shared: &shared
      provider: fake
      model: anchor-wire
compaction:
  model: *shared
",
        )
        .expect("anchor alias compaction model");
        assert_eq!(
            cfg.compaction
                .expect("compaction")
                .model
                .expect("model")
                .model,
            "anchor-wire"
        );
    }

    #[test]
    fn compaction_model_anchor_with_secret_render() {
        use crate::secret_env::SecretMap;
        use crate::template::render_config_template;
        use crate::types::AppConfig;

        let mut secrets = SecretMap::new();
        secrets.insert("SUMMARY_KEY".into(), "sk-summary".into());
        let raw = r#"
models:
  models:
    sum: &sum
      provider: fake
      model: summary-bot
      api_key: "{{ secret.SUMMARY_KEY }}"
compaction:
  model: *sum
"#;
        let rendered = render_config_template(raw, std::path::Path::new("config.yaml"), &secrets)
            .expect("render secret into anchor entry");
        assert!(
            rendered.contains("sk-summary"),
            "rendered yaml must contain secret value: {rendered}"
        );
        let cfg: AppConfig = yaml_serde::from_str(&rendered).expect("parse rendered yaml");
        assert_eq!(
            cfg.compaction
                .expect("compaction")
                .model
                .expect("model")
                .api_key
                .as_deref(),
            Some("sk-summary")
        );
    }

    #[test]
    fn ignores_config_local_yaml() {
        let home = tempfile::tempdir().unwrap();
        let project_root = home.path().join("proj");
        let proj = project_root.join(".xylitol");
        std::fs::create_dir_all(&proj).unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();

        std::fs::write(
            proj.join("config.local.yaml"),
            "models:\n  default_model: from-local\n  models:\n    from-local:\n      provider: fake\n      model: fake\n",
        )
        .unwrap();

        let home_s = home.path().to_str().unwrap().to_string();
        let global_s = global.to_str().unwrap().to_string();
        let project_s = project_root.to_str().unwrap().to_string();
        let entries = [
            ("HOME", home_s.as_str()),
            ("XYLITOL_CONFIG_DIR", global_s.as_str()),
            ("XYLITOL_PROJECT_DIR", project_s.as_str()),
        ];
        let cfg = load_app_config_with(None, env_map(&entries), None).expect("load");
        assert!(
            cfg.model.default_model.is_none(),
            "config.local.yaml must not be merged"
        );

        std::fs::write(
            proj.join("config.yaml"),
            "models:\n  default_model: from-yaml\n  models:\n    from-yaml:\n      provider: fake\n      model: fake\n",
        )
        .unwrap();
        let cfg = load_app_config_with(None, env_map(&entries), None).expect("load with yaml");
        assert_eq!(cfg.model.default_model.as_deref(), Some("from-yaml"));
    }

    fn write_model_config(global: &std::path::Path, body: &str) {
        std::fs::write(
            global.join("config.yaml"),
            format!("models:\n  models:\n{body}"),
        )
        .unwrap();
    }

    fn load_with(global: &std::path::Path) -> Result<AppConfig, LoadError> {
        let home_s = global.parent().unwrap().parent().unwrap().to_str().unwrap();
        let global_s = global.to_str().unwrap();
        let entries = [
            ("HOME", home_s),
            ("XYLITOL_CONFIG_DIR", global_s),
            ("XYLITOL_PROJECT_DIR", home_s),
        ];
        load_app_config_with(None, env_map(&entries), None)
    }

    #[test]
    #[serial(env_global)]
    fn model_entry_literal_passes_through() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        write_model_config(
            &global,
            "    x:\n      provider: openai\n      model: m1\n      api_key: sk-anchor\n",
        );
        let cfg = load_with(&global).expect("load");
        let e = cfg.model.models.get("x").unwrap();
        assert_eq!(e.api_key.as_deref(), Some("sk-anchor"), "字面量 MUST 原样");
        assert_eq!(e.model, "m1");
    }

    #[test]
    #[serial(env_global)]
    fn model_entry_env_var_expands_from_secret_env() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(
            global.join("secret.env"),
            "XYLITOL_A1_SECRET_9f3b=resolved-key\n",
        )
        .unwrap();
        write_model_config(
            &global,
            "    x:\n      provider: openai\n      model: m1\n      api_key: $XYLITOL_A1_SECRET_9f3b\n",
        );
        let cfg = load_with(&global).expect("load");
        let e = cfg.model.models.get("x").unwrap();
        assert_eq!(
            e.api_key.as_deref(),
            Some("resolved-key"),
            "dollar-var syntax MUST 从 secret.env/进程环境展开"
        );
    }

    #[test]
    #[serial(env_global)]
    fn model_entry_default_syntax_expands() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        write_model_config(
            &global,
            "    x:\n      provider: openai\n      model: m1\n      api_key: ${XYLITOL_A1_UNDEF_9f3b:-fallback-key}\n",
        );
        let cfg = load_with(&global).expect("load");
        let e = cfg.model.models.get("x").unwrap();
        assert_eq!(
            e.api_key.as_deref(),
            Some("fallback-key"),
            "braced-default syntax MUST 使用默认值"
        );
    }

    #[test]
    #[serial(env_global)]
    fn model_entry_shell_command_expands() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        write_model_config(
            &global,
            "    x:\n      provider: openai\n      model: \"!printf evalm\"\n",
        );
        let cfg = load_with(&global).expect("load");
        let e = cfg.model.models.get("x").unwrap();
        assert_eq!(e.model, "evalm", "shell-command syntax MUST 展开为 stdout");
    }

    #[test]
    #[serial(env_global)]
    fn model_entry_unbound_variable_rejects_with_context() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        write_model_config(
            &global,
            "    x:\n      provider: openai\n      model: m1\n      api_key: $XYLITOL_A1_UNDEF_9f3b\n",
        );
        let err = load_with(&global).expect_err("未绑定变量 MUST 拒绝装配");
        let msg = err.to_string();
        assert!(
            msg.contains("models.x.api_key"),
            "错误 MUST 带 alias:field 上下文，got: {msg}"
        );
        assert!(msg.contains("unbound"), "错误 MUST 可读，got: {msg}");
    }

    #[test]
    fn feature_plugin_defaults_are_stable() {
        let config = AppConfig::default();
        assert!(config.todo.is_active(), "todo plugin is default-on");
        assert!(!config.workflow.is_active(), "workflow plugin is opt-in");
        assert!(
            config.plan.is_active(),
            "plan plugin keeps the c2973 default"
        );
        assert!(config.plan.read_only, "plan mode is read-only by default");
    }

    #[test]
    #[serial(env_global)]
    fn legacy_loop_policies_keys_rejected_with_migration_hint() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(
            global.join("config.yaml"),
            "loop_policies:\n  enabled: true\n  todo_progress: true\n",
        )
        .unwrap();
        let home_s = home.path().to_str().unwrap();
        let global_s = global.to_str().unwrap();
        let entries = [
            ("HOME", home_s),
            ("XYLITOL_CONFIG_DIR", global_s),
            ("XYLITOL_PROJECT_DIR", home_s),
        ];
        let err = load_app_config_with(None, env_map(&entries), None).expect_err("reject");
        let msg = err.to_string();
        assert!(msg.contains("todo_progress"), "got: {msg}");
        assert!(msg.contains("todo.progress"), "got: {msg}");
    }

    #[test]
    #[serial(env_global)]
    fn legacy_loop_example_section_rejected_with_migration_hint() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(
            global.join("config.yaml"),
            "loop_example:\n  enabled: true\n  max_settles: 2\n",
        )
        .unwrap();
        let home_s = home.path().to_str().unwrap();
        let global_s = global.to_str().unwrap();
        let entries = [
            ("HOME", home_s),
            ("XYLITOL_CONFIG_DIR", global_s),
            ("XYLITOL_PROJECT_DIR", home_s),
        ];
        let err = load_app_config_with(None, env_map(&entries), None).expect_err("reject");
        let msg = err.to_string();
        assert!(msg.contains("loop_example"), "got: {msg}");
        assert!(msg.contains("loop_policies"), "got: {msg}");
    }
}
