use serde_json::Value;

pub(crate) fn is_mcp_tool_name(name: &str) -> bool {
    xylitol_runtime::is_mcp_tool_name(name)
}

pub(crate) fn pretty_json_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

pub(crate) fn pretty_json_text(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return text.to_string();
    }
    match serde_json::from_str::<Value>(trimmed) {
        Ok(value) => pretty_json_value(&value),
        Err(_) => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mcp_prefix() {
        assert!(is_mcp_tool_name("mcp__lspz__get_diagnostics"));
        assert!(!is_mcp_tool_name("mcp-lspz-get_diagnostics"));
        assert!(!is_mcp_tool_name("mcp_lspz_get_diagnostics"));
        assert!(!is_mcp_tool_name("read"));
        assert!(!is_mcp_tool_name("mcp"));
    }

    #[test]
    fn pretty_json_text_formats_object() {
        let out = pretty_json_text(r#"{"a":1,"b":[2]}"#);
        assert!(out.contains('\n'), "expected multiline pretty: {out}");
        assert!(out.contains("\"a\": 1"), "{out}");
    }

    #[test]
    fn pretty_json_text_passthrough_plain() {
        assert_eq!(pretty_json_text("hello\nworld"), "hello\nworld");
    }
}
