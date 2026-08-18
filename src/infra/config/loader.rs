//! Multi-layer config loader with deep merge.
//!
//! Loading order (lowest → highest priority):
//! 1. Global:   `~/.config/xylitol/config.yaml`
//! 2. Project:  `<project>/.xylitol/config.yaml`
//! 3. CLI:      `--config <path>`
//!
//! `config.local.yaml` / `.yml` are **not** loaded (c1400). Use `secret.env` for
//! secrets and `config.yaml` for shareable settings.
//!
//! Before YAML parse, each file is rendered with minijinja
//! (`{{ env.KEY }}` / `{{ secret.KEY }}` / `{{ vars.home }}`). `secret.env`
//! is loaded first.

use std::path::Path;

use serde_json::Value;

use super::error::LoadError;
use super::paths::ConfigPaths;
use super::secret_env::SecretMap;
use super::types::AppConfig;

/// Result of loading AppConfig YAML layers.
#[derive(Debug, Clone)]
pub(crate) struct LoadedAppConfig {
    pub config: AppConfig,
    /// True if at least one `config.yaml` / `config.yml` / `--config` layer was merged.
    pub from_yaml_layers: bool,
}

/// Load configuration from all layers, returning the merged `AppConfig`.
///
/// `cli_config` — optional path to a CLI `--config` YAML file (highest priority).
///
/// Side effects: loads `secret.env` into the process environment (unset keys
/// only) and renders `{{ env.* }}` / `{{ secret.* }}` / `{{ vars.home }}` in
/// each YAML layer.
pub(crate) fn load_app_config(cli_config: Option<&Path>) -> Result<AppConfig, LoadError> {
    Ok(load_app_config_detailed(cli_config)?.config)
}

/// Like [`load_app_config`], but reports whether any YAML layer contributed.
pub(crate) fn load_app_config_detailed(
    cli_config: Option<&Path>,
) -> Result<LoadedAppConfig, LoadError> {
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

/// Injectable load: path discovery via `get_env` / `cwd` (see [`ConfigPaths::discover_with`]).
///
/// Still injects missing `secret.env` keys into the process environment (product
/// parity). Tests that exercise that side effect remain on `env_global`.
#[cfg(test)]
pub(crate) fn load_app_config_with(
    cli_config: Option<&Path>,
    get_env: impl Fn(&str) -> Option<String>,
    cwd: Option<&Path>,
) -> Result<AppConfig, LoadError> {
    Ok(load_app_config_detailed_with(cli_config, get_env, cwd)?.config)
}

/// Like [`load_app_config_detailed`], but path discovery is injectable.
pub(crate) fn load_app_config_detailed_with(
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

    let config: AppConfig = serde_json::from_value(merged.clone())?;
    super::validate::validate_merged_config(&merged)?;
    config.validate_thinking_levels()?;
    config.validate_model_tokenizers()?;
    config.validate_session_max_turns()?;
    Ok(LoadedAppConfig {
        config,
        from_yaml_layers,
    })
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

/// Deep-merge `overlay` into `base` (mutates `base`).
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

    // env_global: 渲染 mcp secret 头经 inject_missing_env 真写进程环境，须与 env_global
    // 域串行防交叉污染。消除路径：断言自清理（tempdir scout）或改注入式方可并行。
    #[test]
    #[serial(env_global)]
    fn loads_yml_alias_and_renders_mcp_secret_headers() {
        let home = tempfile::tempdir().unwrap();
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(global.join("secret.env"), "CTX_KEY=secret-value\n").unwrap();
        // Quoted mustache matches configs/example.yaml + typical user config.
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
        // secret.env still injects into process env → keep env_global.
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
}
