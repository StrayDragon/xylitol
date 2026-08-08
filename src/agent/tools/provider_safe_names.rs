//! Regression: MCP / registry tool names must be provider-safe on the wire.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::infra::mcp::{McpClientManager, McpToolAdapter};
    use crate::protocol::ports::XyTool;
    use crate::protocol::{is_mcp_tool_name, is_provider_safe_tool_name, mcp_tool_public_name};

    #[test]
    fn mcp_public_names_use_double_underscore_delim_and_are_provider_safe() {
        let n = mcp_tool_public_name("context7", "resolve-library-id");
        assert_eq!(n, "mcp__context7__resolve-library-id");
        assert!(is_provider_safe_tool_name(&n));
        assert!(is_mcp_tool_name(&n));
        assert!(!n.contains(':'));
        assert!(!n.contains('.'));
    }

    #[test]
    fn adapter_and_default_tools_all_provider_safe() {
        let manager = Arc::new(McpClientManager::new());
        let mcp: Arc<dyn XyTool> = Arc::new(McpToolAdapter::new(
            "context7".into(),
            "resolve-library-id".into(),
            "docs".into(),
            None,
            manager,
        ));
        let set =
            crate::agent::tools::ToolSet::from_iter(crate::infra::tools::default_tools()).plus(mcp);
        let names: Vec<_> = set.iter().map(|t| t.name().to_string()).collect();
        assert!(
            names
                .iter()
                .any(|n| n == "mcp__context7__resolve-library-id")
        );
        for name in &names {
            assert!(
                is_provider_safe_tool_name(name),
                "unsafe registry name: {name}"
            );
            assert!(!name.contains(':'), "colon must not appear: {name}");
        }
    }

    #[test]
    fn toolset_get_finds_mcp_double_underscore_name() {
        let manager = Arc::new(McpClientManager::new());
        let mcp: Arc<dyn XyTool> = Arc::new(McpToolAdapter::new(
            "lspz".into(),
            "get_diagnostics".into(),
            "d".into(),
            None,
            manager,
        ));
        let set = crate::agent::tools::ToolSet::from_iter([mcp]);
        assert!(set.get("mcp__lspz__get_diagnostics").is_some());
        let wire =
            xylitol_ai_bridge::provider::tool_wire::to_wire_tool_name("mcp:lspz:get_diagnostics");
        assert_eq!(wire, "mcp__lspz__get_diagnostics");
    }
}
