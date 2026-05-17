//! Config validation: JSON Schema runtime check + business rules.

use schemars::schema_for;
use serde_json::Value;

use super::types::AppConfig;

/// Errors from config validation.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ValidationError {
    #[error("JSON Schema compilation failed: {0}")]
    SchemaCompile(String),
    #[error("config validation failed:\n{errors}")]
    Validation { errors: String },
    #[error("business rule violation: {message}")]
    BusinessRule { message: String },
}

/// Validate a parsed (merged) config value against the AppConfig JSON Schema.
pub(crate) fn validate_config(value: &Value) -> Result<(), ValidationError> {
    // Build JSON Schema from AppConfig type.
    let schema = schema_for!(AppConfig);
    let schema_value =
        serde_json::to_value(&schema).map_err(|e| ValidationError::SchemaCompile(e.to_string()))?;

    // Compile schema.
    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| ValidationError::SchemaCompile(e.to_string()))?;

    // Validate — collect all errors for a human-readable report.
    let errors: Vec<_> = validator.iter_errors(value).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        let mut msgs: Vec<String> = errors
            .iter()
            .map(|e| format!("  - {}: {}", e.instance_path(), e))
            .collect();
        msgs.sort();
        Err(ValidationError::Validation {
            errors: msgs.join("\n"),
        })
    }
}

/// Generate JSON Schema from `AppConfig` and write to `configs/config.schema.json`.
pub(crate) fn write_schema_file() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(AppConfig);
    let json = serde_json::to_string_pretty(&schema)?;
    let path = std::path::Path::new("configs/config.schema.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, json)?;
    println!("Schema written to {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_config() {
        // A minimal valid config (all required fields present).
        let value = json!({
            "model": {
                "default_model": "gpt-4o",
                "models": {}
            },
            "execution": {
                "max_retries": 3
            },
            "patch_apply": {},
            "hooks": {
                "global": [],
                "project": [],
                "user": []
            },
            "security": {
                "enabled": false,
                "bash": { "timeout_secs": 120 },
                "filesystem": {},
                "network": {},
                "resource_limits": {
                    "max_memory_mb": 4096,
                    "max_cpu_percent": 80,
                    "max_disk_mb": 1024
                }
            },
            "repeat_detection": {
                "enabled": false,
                "min_n": 3,
                "max_n": 10,
                "window_size": 100,
                "consecutive_hit_threshold": 3,
                "recovery": {
                    "strategy": "backoff",
                    "backoff_factor": 2.0
                }
            },
            "tools": {}
        });
        match validate_config(&value) {
            Ok(_) => {}
            Err(e) => panic!("validation failed: {}", e),
        }
    }

    #[test]
    fn test_validate_default_config() {
        let config = AppConfig::default();
        let value = serde_json::to_value(&config).unwrap();
        assert!(validate_config(&value).is_ok());
    }

    /// Generate `configs/config.schema.json` from AppConfig types.
    /// Run manually when types change: `cargo test gen_schema -- --ignored`
    #[test]
    #[ignore]
    fn gen_schema() {
        write_schema_file().expect("schema generation failed");
    }

    #[test]
    fn test_validate_invalid_type() {
        // model field should be an object, not a string.
        let value = json!({
            "model": "not-an-object",
        });
        let result = validate_config(&value);
        assert!(result.is_err());
    }
}
