//! JSON-Schema structural gate for merged config layers (jsonschema wiring).
//!
//! Merged YAML layers are validated **before** deserialization, so structural
//! mistakes (wrong field type) get a deterministic, schema-anchored error
//! instead of a serde miss. Semantic cross-field rules keep living on
//! [`AppConfig::validate_*`] — this module only answers "does the shape match
//! the declared schema".

use jsonschema::Validator;
use schemars::schema_for;
use serde_json::Value;

use super::types::AppConfig;

/// Root JSON Schema for `AppConfig` (schemars mirrors serde attrs — c510 pattern).
pub fn app_config_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(AppConfig)).expect("AppConfig schema must serialize")
}

/// Validate a merged config document against the `AppConfig` schema.
///
/// Returns a stable one-line message on the first violation. Structural errors
/// (wrong field type) surface here before serde hits them; unknown keys stay
/// permissive (schemars does not mirror `deny_unknown_fields` — historical
/// configs keep loading through `migrate`).
pub(crate) fn validate_merged_config(value: &Value) -> Result<(), String> {
    // Null = "no YAML layers" — the loader short-circuits to `AppConfig::default()`
    // before this call; keep the contract total for future call sites.
    if value.is_null() {
        return Ok(());
    }
    // `merged` is the result of deep-merging YAML layers — the same Value that
    // goes into `AppConfig` deserialization.
    let schema = app_config_schema();
    let validator =
        Validator::new(&schema).map_err(|e| format!("config schema is invalid: {e}"))?;
    validator
        .validate(value)
        .map_err(|e| format!("config schema violation: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_wrong_field_type() {
        // `models` must be an object; a scalar trips the schema before serde.
        let err = validate_merged_config(&json!({ "models": 42 })).unwrap_err();
        assert!(err.contains("schema"), "unexpected error: {err}");
    }

    #[test]
    fn accepts_minimal_document() {
        // Empty document is legal (`AppConfig` derives Default; every field defaults).
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
