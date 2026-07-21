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

use super::paths::ConfigPaths;
use super::secret_env::SecretMap;
use super::types::AppConfig;

/// Errors from config loading.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LoadError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("YAML parse error in {path}: {source}")]
    Yaml {
        path: String,
        source: yaml_serde::Error,
    },
    #[error("{0}")]
    Template(String),

    #[error("deserialize: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("{0}")]
    Validation(String),
}

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
    let paths = ConfigPaths::discover();

    let (secrets, injected) = super::secret_env::load_secret_env_files(&paths);
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

    let config: AppConfig = serde_json::from_value(merged)?;
    config
        .validate_thinking_levels()
        .map_err(LoadError::Validation)?;
    config
        .validate_model_tokenizers()
        .map_err(LoadError::Validation)?;
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

    let rendered = super::template::render_config_template(&raw, path, secrets)
        .map_err(LoadError::Template)?;

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
    fn loads_yml_alias_and_renders_mcp_secret_headers() {
        let home = tempfile::tempdir().unwrap();
        // Isolate migrate_legacy (reads `$HOME/.xylitol`) from the real home tree.
        let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
        // Avoid merging the repo's project `.xylitol/` while tests run from workspace cwd.
        let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", home.path().to_str().unwrap());
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        std::fs::write(global.join("secret.env"), "CTX_KEY=secret-value\n").unwrap();
        // Quoted mustache matches configs/example.yaml + typical user config.
        std::fs::write(
            global.join("config.yml"),
            "mcp_servers:\n  - name: demo\n    transport: sse\n    url: https://example.com/mcp\n    headers:\n      CONTEXT7_API_KEY: \"{{ secret.CTX_KEY }}\"\n",
        )
        .unwrap();
        let _cfg_dir = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());
        let cfg = load_app_config(None).expect("load");
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
        let _home = EnvGuard::set("HOME", home.path().to_str().unwrap());
        let project_root = home.path().join("proj");
        let proj = project_root.join(".xylitol");
        std::fs::create_dir_all(&proj).unwrap();
        let _proj = EnvGuard::set("XYLITOL_PROJECT_DIR", project_root.to_str().unwrap());
        let global = home.path().join(".config").join("xylitol");
        std::fs::create_dir_all(&global).unwrap();
        let _cfg_dir = EnvGuard::set("XYLITOL_CONFIG_DIR", global.to_str().unwrap());

        std::fs::write(
            proj.join("config.local.yaml"),
            "models:\n  default_model: from-local\n  models:\n    from-local:\n      provider: fake\n      model: fake\n",
        )
        .unwrap();

        let cfg = load_app_config(None).expect("load");
        assert!(
            cfg.model.default_model.is_none(),
            "config.local.yaml must not be merged"
        );

        std::fs::write(
            proj.join("config.yaml"),
            "models:\n  default_model: from-yaml\n  models:\n    from-yaml:\n      provider: fake\n      model: fake\n",
        )
        .unwrap();
        let cfg = load_app_config(None).expect("load with yaml");
        assert_eq!(cfg.model.default_model.as_deref(), Some("from-yaml"));
    }

    /// RAII env var restore for loader path tests.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: test-only; short-lived; restored on drop.
            unsafe { std::env::set_var(key, value) };
            Self { key, prev }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
