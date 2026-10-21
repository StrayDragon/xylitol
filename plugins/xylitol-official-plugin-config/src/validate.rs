use jsonschema::Validator;
use schemars::schema_for;
use serde_json::Value;

use super::error::LoadError;
use super::types::AppConfig;

pub fn app_config_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(AppConfig)).expect("AppConfig schema must serialize")
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
}
