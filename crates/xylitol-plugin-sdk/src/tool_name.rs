use strum::{EnumString, IntoStaticStr};

pub const MCP_PUBLIC_DELIMITER: &str = "__";

const MCP_PUBLIC_MARK: &str = "mcp";

pub fn mcp_tool_public_name(server_id: &str, tool_name: &str) -> String {
    format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}{server_id}{MCP_PUBLIC_DELIMITER}{tool_name}")
}

pub fn mcp_tool_armed_prefix(server_id: &str) -> String {
    format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}{server_id}{MCP_PUBLIC_DELIMITER}")
}

pub fn is_mcp_tool_name(name: &str) -> bool {
    name.starts_with(&format!("{MCP_PUBLIC_MARK}{MCP_PUBLIC_DELIMITER}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum BuiltinToolName {
    Ask,
    Write,
    Edit,
}

impl BuiltinToolName {
    pub const ALL: [Self; 3] = [Self::Ask, Self::Write, Self::Edit];

    pub fn as_str(self) -> &'static str {
        self.into()
    }

    pub fn from_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }
}

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
        assert!(!is_mcp_tool_name("mcp:legacy:tool"));
        assert!(!is_mcp_tool_name("mcp_transition_form"));
        assert!(!is_mcp_tool_name("mcp-hyphen-transition"));
        assert_eq!(mcp_tool_armed_prefix("context7"), "mcp__context7__");
    }

    #[test]
    fn builtin_names_roundtrip_through_registry() {
        for kind in BuiltinToolName::ALL {
            assert_eq!(BuiltinToolName::from_name(kind.as_str()), Some(kind));
        }
        assert_eq!(BuiltinToolName::from_name("unknown"), None);
        assert_eq!(BuiltinToolName::Ask.as_str(), "ask");
        assert_eq!(BuiltinToolName::Write.as_str(), "write");
        assert_eq!(BuiltinToolName::Edit.as_str(), "edit");
    }

    #[test]
    fn hyphenated_server_and_tool_keep_hyphens_inside_segments() {
        let n = mcp_tool_public_name("music-studio", "get-something");
        assert_eq!(n, "mcp__music-studio__get-something");
        assert!(is_provider_safe_tool_name(&n));
        assert!(!n.contains(':'));
        let rest = n.strip_prefix("mcp__").unwrap();
        let (server, tool) = rest.split_once("__").unwrap();
        assert_eq!(server, "music-studio");
        assert_eq!(tool, "get-something");
    }
}
