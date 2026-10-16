//! Streaming / partial JSON helpers (pi `parseStreamingJson` analogue).

use serde_json::Value;

/// Best-effort parse of incomplete tool-argument JSON.
///
/// Never panics: empty / unsalvageable input yields `{}`.
pub fn parse_streaming_json(partial: &str) -> Value {
    let trimmed = partial.trim();
    if trimmed.is_empty() {
        return Value::Object(serde_json::Map::new());
    }
    if let Ok(v) = serde_json::from_str(trimmed) {
        return v;
    }
    let fixed = partial_json_fixer::fix_json(trimmed);
    serde_json::from_str(&fixed).unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_yields_object() {
        assert_eq!(parse_streaming_json(""), json!({}));
        assert_eq!(parse_streaming_json("   "), json!({}));
    }

    #[test]
    fn complete_object() {
        assert_eq!(
            parse_streaming_json(r#"{"command":"ls"}"#),
            json!({"command": "ls"})
        );
    }

    #[test]
    fn partial_object_no_panic() {
        let v = parse_streaming_json(r#"{"command":"ls"#);
        assert!(v.is_object(), "got {v:?}");
        assert_eq!(v.get("command").and_then(|c| c.as_str()), Some("ls"));
    }

    #[test]
    fn garbage_yields_object() {
        assert!(parse_streaming_json("not-json{{{").is_object());
    }
}
