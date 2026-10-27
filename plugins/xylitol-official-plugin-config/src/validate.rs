use jsonschema::Validator;
use schemars::schema_for;
use serde_json::Value;

use super::error::LoadError;
use super::types::AppConfig;

pub fn app_config_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(AppConfig)).expect("AppConfig schema must serialize")
}

pub fn render_app_config_schema() -> String {
    let mut value = app_config_schema();
    sort_object_keys(&mut value);
    let mut out = serde_json::to_string_pretty(&value).expect("config schema must render");
    out.push('\n');
    out
}

fn sort_object_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> =
                std::mem::take(map).into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, mut child) in entries {
                sort_object_keys(&mut child);
                map.insert(key, child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sort_object_keys(item);
            }
        }
        _ => {}
    }
}

pub(crate) fn validate_merged_config(value: &Value) -> Result<(), LoadError> {
    if value.is_null() {
        return Ok(());
    }
    let schema = app_config_schema();
    let validator = Validator::new(&schema)
        .map_err(|e| LoadError::validation(format!("config schema is invalid: {e}")))?;
    validator
        .validate(value)
        .map_err(|e| LoadError::validation(format!("config schema violation: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_wrong_field_type() {
        let err = validate_merged_config(&json!({ "models": 42 })).unwrap_err();
        assert!(
            err.to_string().contains("schema"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_minimal_document() {
        assert!(validate_merged_config(&Value::Null).is_ok());
        assert!(validate_merged_config(&json!({})).is_ok());
    }

    #[test]
    fn accepts_realistic_models_block() {
        let doc = json!({
            "models": {
                "default": "gpt-x",
                "providers": {
                    "openai": { "api_key": "sk-x", "model": "gpt-x" }
                }
            }
        });
        assert!(validate_merged_config(&doc).is_ok(), "should pass: {doc}");
    }

    #[test]
    fn schema_rendering_is_idempotent() {
        assert_eq!(render_app_config_schema(), render_app_config_schema());
    }

    #[test]
    fn checked_in_schema_matches_rendered_schema() {
        let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("configs")
            .join("config.schema.json");
        let checked_in = std::fs::read_to_string(&target)
            .unwrap_or_else(|e| panic!("read {}: {e}", target.display()));
        assert_eq!(
            checked_in,
            render_app_config_schema(),
            "configs/config.schema.json drifted from AppConfig; run `just gen-config-schema`"
        );
    }

    #[test]
    fn checked_in_example_satisfies_schema() {
        let configs = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("configs");
        let example = std::fs::read_to_string(configs.join("example.yaml"))
            .unwrap_or_else(|e| panic!("read example.yaml: {e}"));
        let value: Value = yaml_serde::from_str(&example)
            .unwrap_or_else(|e| panic!("example.yaml is not valid YAML: {e}"));
        validate_merged_config(&value).unwrap_or_else(|e| {
            panic!("configs/example.yaml violates AppConfig schema: {e}; regenerate via `just gen-config-example`")
        });
    }
}
