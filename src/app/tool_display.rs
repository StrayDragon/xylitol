//! Cross-surface helpers for tool call/result display (TUI + Print).
//!
//! MCP tools ship machine JSON; keep presentation shared so Print and scrollback
//! stay aligned. No CallToolResult content extraction here (c1460).

use serde_json::Value;

/// MCP tools are registered as `mcp__{server_id}__{tool_name}` (legacy `mcp:` /
/// transition `mcp-` / `mcp_` still detected).
pub(crate) fn is_mcp_tool_name(name: &str) -> bool {
    crate::protocol::is_mcp_tool_name(name)
}

/// Pretty-print a JSON [`Value`]; falls back to compact `to_string` on failure.
pub(crate) fn pretty_json_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// If `text` is valid JSON, return pretty-printed form; otherwise return `text` unchanged.
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

/// Body text for a pending/done MCP tool: call args (pretty) then optional result (pretty).
pub(crate) fn mcp_tool_body(args: Option<&Value>, result: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(args) = args {
        parts.push(format!("args:\n{}", pretty_json_value(args)));
    }
    if let Some(result) = result {
        let pretty = pretty_json_text(result);
        if parts.is_empty() {
            parts.push(pretty);
        } else {
            parts.push(format!("result:\n{pretty}"));
        }
    }
    parts.join("\n\n")
}

/// Recover call args from a pending MCP body (`args:\\n{…}`), ignoring any streamed junk after.
pub(crate) fn extract_mcp_args_value(body: &str) -> Option<Value> {
    let rest = body.strip_prefix("args:")?.trim_start();
    let mut stream = serde_json::Deserializer::from_str(rest).into_iter::<Value>();
    stream.next()?.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mcp_prefix() {
        assert!(is_mcp_tool_name("mcp__lspz__get_diagnostics"));
        assert!(is_mcp_tool_name("mcp-lspz-get_diagnostics")); // transition hyphen
        assert!(is_mcp_tool_name("mcp_lspz_get_diagnostics")); // transition underscore
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

    #[test]
    fn mcp_body_args_then_result() {
        let args = serde_json::json!({"uri": "file:///x"});
        let body = mcp_tool_body(Some(&args), Some(r#"{"content":[],"isError":false}"#));
        assert!(body.starts_with("args:\n"), "{body}");
        assert!(body.contains("result:\n"), "{body}");
        assert!(body.contains("\"uri\": \"file:///x\""), "{body}");
        assert!(body.contains("\"isError\": false"), "{body}");
    }

    #[test]
    fn extract_args_ignores_streamed_tail() {
        let polluted = concat!(
            "args:\n{}\n",
            r#"{"content":[{"type":"text","text":"x"}],"isError":false}"#
        );
        assert_eq!(
            extract_mcp_args_value(polluted),
            Some(serde_json::json!({}))
        );
    }
}
