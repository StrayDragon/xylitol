use jsonschema::Validator;
use schemars::schema_for;
use serde_json::Value;

use super::error::LoadError;
use super::sections::sections;
use super::types::BootstrapConfig;

pub fn app_config_schema() -> serde_json::Value {
    let mut root = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "AppConfig",
        "type": "object",
        "properties": {},
        "$defs": {},
        "additionalProperties": true
    });
    let mut bootstrap =
        serde_json::to_value(schema_for!(BootstrapConfig)).expect("bootstrap schema");
    move_defs(&mut root, &mut bootstrap, "xylitol");
    root["properties"]["xylitol"] = bootstrap;
    for section in sections() {
        let mut schema = (section.schema)();
        move_defs(&mut root, &mut schema, section.key);
        root["properties"][section.key] = schema;
    }
    root
}

fn move_defs(root: &mut Value, schema: &mut Value, prefix: &str) {
    let defs = schema
        .as_object_mut()
        .and_then(|map| map.remove("$defs"))
        .and_then(|defs| defs.as_object().cloned())
        .unwrap_or_default();
    if let Some(map) = schema.as_object_mut() {
        map.remove("$schema");
    }
    prefix_refs(schema, prefix);
    let root_defs = root["$defs"].as_object_mut().expect("root $defs");
    for (key, mut value) in defs {
        prefix_refs(&mut value, prefix);
        root_defs.insert(format!("{prefix}_{key}"), value);
    }
}

fn prefix_refs(value: &mut Value, prefix: &str) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "$ref" {
                    if let Some(path) = child.as_str()
                        && let Some(rest) = path.strip_prefix("#/$defs/")
                    {
                        *child = Value::String(format!("#/$defs/{prefix}_{rest}"));
                    }
                } else {
                    prefix_refs(child, prefix);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                prefix_refs(item, prefix);
            }
        }
        _ => {}
    }
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
    let Some(map) = value.as_object() else {
        return Err(LoadError::validation("config root must be a mapping"));
    };
    if let Some(bootstrap) = map.get("xylitol") {
        let schema = serde_json::to_value(schema_for!(BootstrapConfig)).expect("bootstrap schema");
        validate_with(&schema, bootstrap, "xylitol")?;
    }
    for section in sections() {
        if let Some(section_value) = map.get(section.key) {
            validate_with(&(section.schema)(), section_value, section.key)?;
        }
    }
    Ok(())
}

fn validate_with(schema: &Value, value: &Value, label: &str) -> Result<(), LoadError> {
    let validator = Validator::new(schema).map_err(|e| {
        LoadError::validation(format!("config schema for `{label}` is invalid: {e}"))
    })?;
    validator
        .validate(value)
        .map_err(|e| LoadError::validation(format!("config section `{label}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_wrong_field_type() {
        let err = validate_merged_config(&json!({ "models": 42 })).unwrap_err();
        assert!(
            err.to_string().contains("config section"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_known_sections() {
        validate_merged_config(&json!({
            "xylitol": {"schema_version": 1, "unknown_keys": "error"},
            "models": {"default_model": "a", "entries": {}},
            "tui": {"activity_fold": {"enabled": false}}
        }))
        .unwrap();
    }

    #[test]
    fn checked_in_schema_matches_registry() {
        let expected = include_str!("../../../configs/config.schema.json");
        assert_eq!(expected, render_app_config_schema());
    }

    #[test]
    fn rejects_unknown_bootstrap_key() {
        let err = validate_merged_config(&json!({
            "xylitol": {"schema_version": 1, "bogus": true}
        }))
        .unwrap_err();
        assert!(err.to_string().contains("xylitol"), "unexpected: {err}");
    }
}
