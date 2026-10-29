use std::collections::{BTreeMap, BTreeSet};
use xylitol_plugin_sdk::session::{CapabilityEntry, CapabilityKind, CapabilitySnapshot};

use super::toolset::ToolSet;

pub fn tool_capability_id(name: &str) -> String {
    format!("tool:{name}")
}

pub fn mcp_capability_id(server: &str) -> String {
    format!("mcp:{server}")
}

pub fn filter_tools_by_capability_overlay(base: ToolSet, disabled: &BTreeSet<String>) -> ToolSet {
    base.retain(|tool| {
        let name = tool.name();
        if disabled.contains(&tool_capability_id(name)) {
            return false;
        }
        if let Some(rest) = name.strip_prefix("mcp__")
            && let Some((server, _)) = rest.split_once("__")
            && disabled.contains(&mcp_capability_id(server))
        {
            return false;
        }
        true
    })
}

pub fn capability_snapshot(
    base: &ToolSet,
    mcp_server_ids: &[String],
    disabled: &BTreeSet<String>,
    applied: &BTreeSet<String>,
) -> CapabilitySnapshot {
    let mut entries: BTreeMap<String, CapabilityEntry> = BTreeMap::new();
    let mut servers: BTreeSet<String> = mcp_server_ids.iter().cloned().collect();

    for tool in base.iter() {
        let name = tool.name();
        if let Some(rest) = name.strip_prefix("mcp__")
            && let Some((server, _)) = rest.split_once("__")
        {
            servers.insert(server.to_string());
            continue;
        }
        let id = tool_capability_id(name);
        entries.insert(
            id.clone(),
            CapabilityEntry {
                id,
                kind: CapabilityKind::Tool,
                label: name.to_string(),
                profile_enabled: true,
                session_disabled: disabled.contains(&tool_capability_id(name)),
                effective_enabled: !disabled.contains(&tool_capability_id(name)),
                pending: disabled.contains(&tool_capability_id(name))
                    != applied.contains(&tool_capability_id(name)),
            },
        );
    }

    for server in servers {
        let id = mcp_capability_id(&server);
        entries.insert(
            id.clone(),
            CapabilityEntry {
                id: id.clone(),
                kind: CapabilityKind::McpServer,
                label: server,
                profile_enabled: true,
                session_disabled: disabled.contains(&id),
                effective_enabled: !disabled.contains(&id),
                pending: disabled.contains(&id) != applied.contains(&id),
            },
        );
    }

    let entries: Vec<_> = entries.into_values().collect();
    let pending = entries.iter().any(|entry| entry.pending);
    CapabilitySnapshot { entries, pending }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use xylitol_plugin_sdk::XyTool;

    struct Stub {
        name: &'static str,
    }

    #[async_trait::async_trait]
    impl XyTool for Stub {
        fn name(&self) -> &str {
            self.name
        }

        fn description(&self) -> &str {
            "stub"
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object" })
        }

        async fn execute(
            &self,
            _: &xylitol_plugin_sdk::XyToolCtx,
            _: serde_json::Value,
        ) -> Result<String, xylitol_plugin_sdk::error::XyToolError> {
            Ok(String::new())
        }
    }

    fn tool(name: &'static str) -> Arc<dyn XyTool> {
        Arc::new(Stub { name })
    }

    #[test]
    fn filters_tool_and_mcp_server() {
        let base = ToolSet::from_iter(vec![
            tool("read"),
            tool("ffgrep"),
            tool("mcp__github__search"),
            tool("mcp__jira__issue"),
        ]);
        let mut disabled = BTreeSet::new();
        disabled.insert(tool_capability_id("ffgrep"));
        disabled.insert(mcp_capability_id("github"));
        let filtered = filter_tools_by_capability_overlay(base, &disabled);
        let names: Vec<_> = filtered.iter().map(|tool| tool.name()).collect();
        assert_eq!(names, vec!["read", "mcp__jira__issue"]);
    }

    #[test]
    fn snapshot_marks_pending_until_applied() {
        let base = ToolSet::from_iter(vec![tool("read"), tool("mcp__github__search")]);
        let mut disabled = BTreeSet::new();
        disabled.insert(tool_capability_id("read"));
        let applied = BTreeSet::new();
        let snap = capability_snapshot(&base, &[], &disabled, &applied);
        assert!(snap.pending);
        let read = snap
            .entries
            .iter()
            .find(|entry| entry.id == "tool:read")
            .expect("read entry");
        assert!(read.session_disabled);
        assert!(!read.effective_enabled);
        assert!(read.pending);
        let github = snap
            .entries
            .iter()
            .find(|entry| entry.id == "mcp:github")
            .expect("mcp entry");
        assert!(!github.session_disabled);
        assert!(github.effective_enabled);
        assert!(!github.pending);
    }

    #[test]
    fn snapshot_includes_configured_mcp_without_tools() {
        let base = ToolSet::from_iter(vec![tool("read")]);
        let snap = capability_snapshot(
            &base,
            &["empty".to_string()],
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        assert!(snap.entries.iter().any(|entry| entry.id == "mcp:empty"));
    }
}
