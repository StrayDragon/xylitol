//! Multi-layer config loader with deep merge.
//!
//! Loading order (lowest → highest priority):
//! 1. Global base:   `~/.config/xylitol/config.yaml`
//! 2. Global local:  `~/.config/xylitol/config.local.yaml`
//! 3. Project base:  `<project>/.xylitol/config.yaml`
//! 4. Project local: `<project>/.xylitol/config.local.yaml`
//! 5. CLI override:  `--config <path>` (single file, no local overlay)

use std::path::Path;

use serde_json::Value;

use super::paths::ConfigPaths;
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

    // Deserialize to AppConfig.
    let config: AppConfig = serde_json::from_value(merged)?;

    Ok(config)
}

/// Load a YAML file, render templates, and parse to `serde_json::Value`.
fn load_and_render(path: &Path, _paths: &ConfigPaths) -> Result<Value, LoadError> {
    let raw = std::fs::read_to_string(path).map_err(|e| LoadError::Io {
        path: path.to_string_lossy().to_string(),
        source: e,
    })?;

    // Parse YAML directly (template/secret resolution removed — use env vars via the shell).
    let value: Value = yaml_serde::from_str(&raw).map_err(|e| LoadError::Yaml {
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
        // For arrays and scalars, overlay replaces base.
        (base, overlay) => *base = overlay,
    }
}

// ---------------------------------------------------------------------------
// Business rules on AppConfig
// ---------------------------------------------------------------------------

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
}
