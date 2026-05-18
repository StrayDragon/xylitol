//! Multi-layer config loader with deep merge.
//!
//! Loading order (lowest → highest priority):
//! 1. Global base:   `~/.config/xylitol/config.yaml`
//! 2. Global local:  `~/.config/xylitol/config.local.yaml`
//! 3. Project base:  `<project>/.xylitol/config.yaml`
//! 4. Project local: `<project>/.xylitol/config.local.yaml`
//! 5. CLI override:  `--config <path>` (single file, no local overlay)

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use super::paths::ConfigPaths;
use super::secret::load_secret_env;
use super::template::render;
use super::types::AppConfig;
use super::validate::{ValidationError, validate_config};

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
        source: serde_yaml::Error,
    },
    #[error("template {0}")]
    Template(#[from] super::template::TemplateError),
    #[error("secret: {0}")]
    Secret(#[from] super::secret::SecretError),
    #[error("validation: {0}")]
    Validation(#[from] ValidationError),
    #[error("deserialize: {0}")]
    Deserialize(#[from] serde_json::Error),
}

/// Load configuration from all layers, returning the merged `AppConfig`.
///
/// `cli_config` — optional path to a CLI `--config` YAML file (highest priority).
pub(crate) fn load_app_config(cli_config: Option<&Path>) -> Result<AppConfig, LoadError> {
    let paths = ConfigPaths::discover();

    // Load all config levels into serde_json::Value, rendering templates.
    let mut merged = Value::Null;

    // Level 1: global base
    let global_base = paths.global_dir.join("config.yaml");
    if global_base.exists() {
        let val = load_and_render(&global_base, &paths)?;
        deep_merge(&mut merged, val);
    }

    // Level 2: global local
    let global_local = paths.global_dir.join("config.local.yaml");
    if global_local.exists() {
        let val = load_and_render(&global_local, &paths)?;
        deep_merge(&mut merged, val);
    }

    // Level 3: project base
    if let Some(ref proj_dir) = paths.project_dir {
        let proj_base = proj_dir.join("config.yaml");
        if proj_base.exists() {
            let val = load_and_render(&proj_base, &paths)?;
            deep_merge(&mut merged, val);
        }

        // Level 4: project local
        let proj_local = proj_dir.join("config.local.yaml");
        if proj_local.exists() {
            let val = load_and_render(&proj_local, &paths)?;
            deep_merge(&mut merged, val);
        }
    }

    // Level 5: CLI --config
    if let Some(cli_path) = cli_config
        && cli_path.exists()
    {
        let val = load_and_render(cli_path, &paths)?;
        deep_merge(&mut merged, val);
    }

    // If no config was loaded at all, use default AppConfig.
    if merged.is_null() {
        return Ok(AppConfig::default());
    }

    // Validate against JSON schema.
    validate_config(&merged)?;

    // Deserialize to AppConfig.
    let config: AppConfig = serde_json::from_value(merged)?;

    // Post-load business rule validation.
    config.validate_business_rules()?;

    Ok(config)
}

/// Load a YAML file, render templates, and parse to `serde_json::Value`.
fn load_and_render(path: &Path, paths: &ConfigPaths) -> Result<Value, LoadError> {
    let raw = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        path: path.to_string_lossy().to_string(),
        source: e,
    })?;

    // Collect env vars.
    let env_vars: HashMap<String, String> = std::env::vars().collect();

    // Collect secret vars from all config locations.
    let mut secret_vars = HashMap::new();

    // Load from global secret.env.
    let global_secret = paths.global_dir.join("secret.env");
    let result = load_secret_env(&global_secret);
    secret_vars.extend(result.vars);
    // Report warnings (non-fatal).
    for w in &result.warnings {
        tracing::warn!("{}", w);
    }

    // Load from project secret.env.
    if let Some(ref proj_dir) = paths.project_dir {
        let proj_secret = proj_dir.join("secret.env");
        let result = load_secret_env(&proj_secret);
        secret_vars.extend(result.vars);
        for w in &result.warnings {
            tracing::warn!("{}", w);
        }
    }

    // OS env vars take precedence over secret.env.
    for (k, v) in &env_vars {
        secret_vars.entry(k.clone()).or_insert_with(|| v.clone());
    }

    // Render template.
    let rendered = render(&raw, &env_vars, &secret_vars)?;

    // Parse YAML.
    let value: Value = serde_yaml::from_str(&rendered).map_err(|e| LoadError::Yaml {
        path: path.to_string_lossy().to_string(),
        source: e,
    })?;

    Ok(value)
}

/// Deep-merge `overlay` into `base` (mutates `base`).
///
/// Rules:
/// - `Value::Object`: recursive merge, latter keys override earlier.
/// - `Value::Array`: overlay replaces base (default).
/// - Scalar values: overlay replaces base.
pub(crate) fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (base @ &mut Value::Object(_), Value::Object(map)) => {
            let base_map = base.as_object_mut().unwrap();
            for (k, v) in map {
                if let Some(existing) = base_map.get_mut(&k) {
                    deep_merge(existing, v);
                } else {
                    base_map.insert(k, v);
                }
            }
        }
        // For arrays and scalars, overlay replaces base.
        (base, overlay) => *base = overlay,
    }
}

// ---------------------------------------------------------------------------
// Business rules on AppConfig
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Validate post-deserialization business rules.
    ///
    /// Checks:
    /// - Model ID cross-references within `models` entries.
    /// - Provider constraints (MVP: only OpenAI / Anthropic).
    pub(crate) fn validate_business_rules(&self) -> Result<(), ValidationError> {
        // Check that model entries reference valid models.
        for (alias, entry) in &self.model.models {
            if let Some(ref fallback) = entry.fallback
                && fallback != alias
                && !self.model.models.contains_key(fallback)
            {
                return Err(ValidationError::BusinessRule {
                    message: format!(
                        "model `{alias}` has fallback `{fallback}` which is not defined in `models`"
                    ),
                });
            }
        }
        Ok(())
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
    fn test_business_rules_valid() {
        use std::collections::HashMap;
        let config = AppConfig {
            model: super::super::types::ModelConfig {
                default_model: "gpt-4o".into(),
                models: HashMap::from([
                    (
                        "gpt-4o".into(),
                        super::super::types::ModelEntry {
                            provider: super::super::types::ProviderKind::OpenAI,
                            model: "gpt-4o".into(),
                            fallback: Some("claude-3".into()),
                        },
                    ),
                    (
                        "claude-3".into(),
                        super::super::types::ModelEntry {
                            provider: super::super::types::ProviderKind::Anthropic,
                            model: "claude-3-5-sonnet".into(),
                            fallback: None,
                        },
                    ),
                ]),
            },
            ..Default::default()
        };
        assert!(config.validate_business_rules().is_ok());
    }

    #[test]
    fn test_business_rules_invalid_fallback() {
        use std::collections::HashMap;
        let config = AppConfig {
            model: super::super::types::ModelConfig {
                default_model: "gpt-4o".into(),
                models: HashMap::from([(
                    "gpt-4o".into(),
                    super::super::types::ModelEntry {
                        provider: super::super::types::ProviderKind::OpenAI,
                        model: "gpt-4o".into(),
                        fallback: Some("nonexistent-model".into()),
                    },
                )]),
            },
            ..Default::default()
        };
        assert!(config.validate_business_rules().is_err());
    }

    #[test]
    fn test_app_config_default_values() {
        let config = AppConfig::default();
        assert_eq!(config.model.default_model, "gpt-4o");
        assert!(config.model.models.is_empty());
        assert_eq!(config.execution.max_retries, 3);
        assert!(!config.security.enabled);
        assert_eq!(config.security.bash.timeout_secs, 120);
        assert!(!config.repeat_detection.enabled);
        assert_eq!(config.repeat_detection.recovery.strategy, "sequential");
        assert!(config.hooks.global.is_empty());
        assert!(config.tools.allowlist.is_empty());
    }

    #[test]
    fn test_load_app_config_no_files() {
        // Without any config files, should return defaults.
        let config = load_app_config(None).unwrap();
        assert_eq!(config.model.default_model, "gpt-4o");
    }

    #[test]
    fn test_load_app_config_cli_override() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("xylitol_test_cli_config");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cli-config.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "model:").unwrap();
        writeln!(f, "  default_model: claude-opus-4").unwrap();
        writeln!(f, "  models: {{}}").unwrap();
        writeln!(f, "execution:").unwrap();
        writeln!(f, "  max_retries: 5").unwrap();
        writeln!(f, "patch_apply: {{}}").unwrap();
        writeln!(f, "hooks:").unwrap();
        writeln!(f, "  global: []").unwrap();
        writeln!(f, "  project: []").unwrap();
        writeln!(f, "  user: []").unwrap();
        writeln!(f, "security:").unwrap();
        writeln!(f, "  enabled: true").unwrap();
        writeln!(
            f,
            "  bash:\n    timeout_secs: 300\n  filesystem: {{}}\n  network: {{}}"
        )
        .unwrap();
        writeln!(
            f,
            "  resource_limits:\n    max_memory_mb: 8192\n    max_cpu_percent: 90\n    max_disk_mb: 2048"
        )
        .unwrap();
        writeln!(f, "repeat_detection:").unwrap();
        writeln!(
            f,
            "  enabled: true\n  min_n: 2\n  max_n: 8\n  window_size: 50\n  consecutive_hit_threshold: 2\n  recovery:\n    strategy: rotate\n    max_attempts: 5\n    actions: []"
        )
        .unwrap();
        writeln!(f, "tools: {{}}").unwrap();

        let config = load_app_config(Some(&path)).unwrap();
        assert_eq!(config.model.default_model, "claude-opus-4");
        assert!(config.security.enabled);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
