//! Shared MCP tool naming + provider wire-safety.
//!
//! Public registry names MUST match `^[a-zA-Z0-9_-]+$` (DeepSeek / Anthropic /
//! OpenAI-compat). Compose as `mcp{DELIM}{server_id}{DELIM}{tool_name}`.
//!
//! **Change delimiter only here** ([`MCP_PUBLIC_DELIMITER`]). Prefer Claude/Codex
//! style `__`: segment sep is rare inside server/tool ids, while `-`/`_` remain
//! legal **inside** segments (`resolve-library-id`, `get_diagnostics`).
//! Live probe: DeepSeek rejects `.`/`:`; accepts `-`/`_`. Do **not** sanitize
//! segment hyphens away (unlike Codex callable names).
//!
//! **Never reverse-parse** the public name for `tools/call`; adapters keep
//! `server_id` / `tool_name` fields (tool names MAY still contain `__`).

/// Segment delimiter between `mcp`, `server_id`, and `tool_name`.
///
/// Single SSOT: retarget public naming by changing this const (+ specs/docs).
pub const MCP_PUBLIC_DELIMITER: &str = "__";

const MCP_PUBLIC_MARK: &str = "mcp";

/// Compose the public tool name for an MCP tool.
pub fn mcp_tool_public_name(server_id: &str, tool_name: &str) -> String {
    format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}{server_id}{MCP_PUBLIC_DELIMITER}{tool_name}")
}

/// Prefix used to count armed tools for a server in ToolSet snapshots.
pub fn mcp_tool_armed_prefix(server_id: &str) -> String {
    format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}{server_id}{MCP_PUBLIC_DELIMITER}")
}

/// True when `name` is an MCP-registered tool.
///
/// Accepts modern `mcp__…`, transition `mcp-…` / `mcp_…`, and legacy `mcp:…`.
pub fn is_mcp_tool_name(name: &str) -> bool {
    name.starts_with(&format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}"))
        || name.starts_with("mcp-")
        || name.starts_with("mcp_")
        || name.starts_with("mcp:")
}

/// Provider-visible tool name pattern shared by DeepSeek / Anthropic / OpenAI-compat.
pub fn is_provider_safe_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_name_is_provider_safe_double_underscore_delim() {
        let n = mcp_tool_public_name("context7", "resolve-library-id");
        assert_eq!(n, "mcp__context7__resolve-library-id");
        assert_eq!(MCP_PUBLIC_DELIMITER, "__");
        assert!(is_provider_safe_tool_name(&n));
        assert!(is_mcp_tool_name(&n));
        assert!(!is_mcp_tool_name("read"));
        assert!(is_mcp_tool_name("mcp:legacy:tool"));
        assert!(is_mcp_tool_name("mcp_transition_form"));
        assert!(is_mcp_tool_name("mcp-hyphen-transition"));
        assert_eq!(mcp_tool_armed_prefix("context7"), "mcp__context7__");
    }

    #[test]
    fn hyphenated_server_and_tool_keep_hyphens_inside_segments() {
        // Delimiter `__` — segments may still contain `-` (Claude plugin style;
        // llmanspec/changes/archive/2026-08-08-c1940-update-multi-api-named-compat/research/mcp-tool-public-naming-hyphen-2026.md). Dispatch MUST
        // use stored fields, not reverse-parse.
        let n = mcp_tool_public_name("music-studio", "get-something");
        assert_eq!(n, "mcp__music-studio__get-something");
        assert!(is_provider_safe_tool_name(&n));
        assert!(!n.contains(':'));
        // Optional debug split: strip mark+delim, then split once on delim.
        let rest = n.strip_prefix("mcp__").unwrap();
        let (server, tool) = rest.split_once("__").unwrap();
        assert_eq!(server, "music-studio");
        assert_eq!(tool, "get-something");
    }
}
