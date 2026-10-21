pub fn to_wire_tool_name(name: &str) -> String {
    name.replace(':', "__")
}

pub fn from_wire_tool_name(wire: &str, canonical: &[impl AsRef<str>]) -> String {
    if canonical.iter().any(|n| n.as_ref() == wire) {
        return wire.to_string();
    }
    for name in canonical {
        let n = name.as_ref();
        if to_wire_tool_name(n) == wire {
            return n.to_string();
        }
    }
    wire.to_string()
}

pub fn canonical_tool_names(tools: &[crate::dto::XyToolSchema]) -> Vec<String> {
    tools.iter().map(|t| t.name.clone()).collect()
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
    use crate::dto::XyToolSchema;
    use serde_json::json;

    #[test]
    fn modern_mcp_name_is_already_safe() {
        let n = "mcp__context7__resolve-library-id";
        assert_eq!(to_wire_tool_name(n), n);
        assert!(is_provider_safe_tool_name(n));
    }

    #[test]
    fn legacy_colon_encodes_to_double_underscore() {
        assert_eq!(
            to_wire_tool_name("mcp:context7:resolve-library-id"),
            "mcp__context7__resolve-library-id"
        );
        assert!(is_provider_safe_tool_name(&to_wire_tool_name(
            "mcp:context7:resolve-library-id"
        )));
    }

    #[test]
    fn decode_legacy_wire_to_canonical() {
        let names = ["read", "mcp__context7__resolve-library-id"];
        assert_eq!(
            from_wire_tool_name("mcp__context7__resolve-library-id", &names),
            "mcp__context7__resolve-library-id"
        );
        let legacy = ["mcp:context7:resolve-library-id"];
        assert_eq!(
            from_wire_tool_name("mcp__context7__resolve-library-id", &legacy),
            "mcp:context7:resolve-library-id"
        );
    }

    #[test]
    fn convert_tools_style_payload_rejects_unsafe_names() {
        let tools = [
            XyToolSchema {
                name: "read".into(),
                description: "r".into(),
                parameters: json!({"type": "object"}),
            },
            XyToolSchema {
                name: "mcp__lspz__get_diagnostics".into(),
                description: "d".into(),
                parameters: json!({"type": "object"}),
            },
            XyToolSchema {
                name: "mcp:should:not:reach:wire".into(),
                description: "legacy".into(),
                parameters: json!({"type": "object"}),
            },
        ];
        for t in &tools {
            let wire = to_wire_tool_name(&t.name);
            assert!(
                is_provider_safe_tool_name(&wire),
                "unsafe wire name for {}: {wire}",
                t.name
            );
            assert!(!wire.contains(':'));
        }
        let names = canonical_tool_names(&tools);
        assert_eq!(names.len(), 3);
    }
}
