#[cfg(test)]
mod tests {
    use xylitol_plugin_sdk::{is_mcp_tool_name, is_provider_safe_tool_name, mcp_tool_public_name};

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
    fn fixture_and_mcp_names_all_provider_safe() {
        let mcp = crate::test_support::named_tool("mcp__context7__resolve-library-id");
        let set = crate::test_support::named_tools(&["read", "write"]).plus(mcp);
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
        let set = crate::test_support::named_tools(&["mcp__lspz__get_diagnostics"]);
        assert!(set.get("mcp__lspz__get_diagnostics").is_some());
        let wire = xylitol_plugin_sdk::tool_wire::to_wire_tool_name("mcp:lspz:get_diagnostics");
        assert_eq!(wire, "mcp__lspz__get_diagnostics");
    }
}
