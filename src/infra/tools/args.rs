//! Shared serde helpers for built-in tool argument parsing.
//!
//! Trait boundary stays [`serde_json::Value`] (MCP / dynamic tools). Each built-in
//! tool deserializes once into a typed `*Args` struct at `execute` entry.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::protocol::error::XyToolError;

/// Deserialize tool JSON args into `T`, mapping failures to [`XyToolError::InvalidArgs`].
pub fn parse_tool_args<T: DeserializeOwned>(args: Value) -> Result<T, XyToolError> {
    serde_json::from_value(args).map_err(|e| XyToolError::InvalidArgs(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct SampleArgs {
        pattern: String,
        #[serde(default)]
        ignore_case: bool,
    }

    #[test]
    fn parse_maps_missing_required_to_invalid_args() {
        let err = parse_tool_args::<SampleArgs>(json!({})).unwrap_err();
        assert!(matches!(err, XyToolError::InvalidArgs(_)));
    }

    #[test]
    fn parse_accepts_camel_case_aliases() {
        let args: SampleArgs =
            parse_tool_args(json!({"pattern": "foo", "ignoreCase": true})).unwrap();
        assert_eq!(
            args,
            SampleArgs {
                pattern: "foo".into(),
                ignore_case: true,
            }
        );
    }
}
