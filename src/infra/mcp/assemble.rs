//! MCP assembly helpers — zero-cost gate, discover → `XyTool`, reload support.

use std::sync::Arc;

use serde_json::Value;

use super::adapter::McpToolAdapter;
use super::client::McpClientManager;
use super::types::McpServerConfig;
use crate::infra::config::types::AppConfig;
use crate::protocol::ports::XyTool;

/// True when MCP should be assembled (non-empty server list).
pub fn mcp_enabled(servers: &Option<Vec<McpServerConfig>>) -> bool {
    servers.as_ref().is_some_and(|s| !s.is_empty())
}

/// Build [`McpToolAdapter`]s from already-discovered tool rows (pure; testable).
pub fn adapters_from_discovered(
    manager: Arc<McpClientManager>,
    rows: &[(String, String, String, Value)],
) -> Vec<Arc<dyn XyTool>> {
    rows.iter()
        .map(|(server_id, tool_name, description, schema)| {
            Arc::new(McpToolAdapter::new(
                server_id.clone(),
                tool_name.clone(),
                description.clone(),
                Some(schema.clone()),
                manager.clone(),
            )) as Arc<dyn XyTool>
        })
        .collect()
}

/// Connect to the given servers (if any), discover tools, return manager + adapters.
///
/// Empty / disabled input returns `None` without constructing a manager.
pub async fn connect_and_discover(
    servers: &[McpServerConfig],
) -> Result<Option<(Arc<McpClientManager>, Vec<Arc<dyn XyTool>>)>, String> {
    connect_and_discover_with_progress(servers, None).await
}

/// Like [`connect_and_discover`], mirroring connect progress when `progress` is set (c1200).
pub async fn connect_and_discover_with_progress(
    servers: &[McpServerConfig],
    progress: Option<std::sync::Arc<tokio::sync::Mutex<super::client::McpConnectProgress>>>,
) -> Result<Option<(Arc<McpClientManager>, Vec<Arc<dyn XyTool>>)>, String> {
    if servers.is_empty() {
        return Ok(None);
    }

    let manager = Arc::new(McpClientManager::new());
    manager
        .connect_servers_with_progress(servers, progress.clone())
        .await?;
    let rows = manager.list_all_tools().await;
    let tools = adapters_from_discovered(manager.clone(), &rows);
    if let Some(progress) = progress.as_ref() {
        let mut snap = progress.lock().await;
        snap.connecting = false;
        snap.current = None;
    }
    Ok(Some((manager, tools)))
}

/// Convenience: pull servers from [`AppConfig`] and assemble.
pub async fn connect_and_discover_from_config(
    config: &AppConfig,
) -> Result<Option<(Arc<McpClientManager>, Vec<Arc<dyn XyTool>>)>, String> {
    match &config.mcp_servers {
        Some(servers) if !servers.is_empty() => connect_and_discover(servers).await,
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_enabled_none_and_empty() {
        assert!(!mcp_enabled(&None));
        assert!(!mcp_enabled(&Some(vec![])));
    }

    #[test]
    fn mcp_enabled_nonempty() {
        let servers = Some(vec![McpServerConfig {
            name: "fs".into(),
            ..Default::default()
        }]);
        assert!(mcp_enabled(&servers));
    }

    #[test]
    fn adapters_from_discovered_names() {
        let manager = Arc::new(McpClientManager::new());
        let rows = vec![(
            "fs".into(),
            "read".into(),
            "Read a file".into(),
            serde_json::json!({"type": "object"}),
        )];
        let tools = adapters_from_discovered(manager, &rows);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "mcp__fs__read");
    }

    #[tokio::test]
    async fn connect_and_discover_empty_is_none() {
        let result = connect_and_discover(&[]).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn validate_stdio_requires_command() {
        let cfg = McpServerConfig {
            name: "x".into(),
            transport: crate::infra::mcp::McpTransportKind::Stdio,
            command: None,
            ..Default::default()
        };
        assert!(cfg.validate().unwrap_err().contains("command"));
    }

    #[tokio::test]
    async fn connect_and_discover_invalid_still_returns_manager() {
        // Invalid entries must not fail the Result; manager may exist with zero tools.
        let servers = [McpServerConfig {
            name: "bad".into(),
            transport: crate::infra::mcp::McpTransportKind::Stdio,
            command: None,
            ..Default::default()
        }];
        let result = connect_and_discover(&servers).await.unwrap();
        let Some((manager, tools)) = result else {
            panic!("non-empty server list should still construct manager");
        };
        assert!(tools.is_empty());
        let diags = manager.diagnostics().await;
        assert!(!diags.is_empty());
        assert!(manager.connected_servers().await.is_empty());
    }

    fn fixture_server_script() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/support/mcp_fixture_server.py")
    }

    fn fixture_mcp_config(name: &str, tools: &str) -> McpServerConfig {
        let mut env = std::collections::HashMap::new();
        env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), tools.into());
        McpServerConfig {
            name: name.into(),
            transport: crate::infra::mcp::McpTransportKind::Stdio,
            command: Some("python3".into()),
            args: Some(vec![fixture_server_script().display().to_string()]),
            env: Some(env),
            ..Default::default()
        }
    }

    /// Real stdio MCP: discover exposes `mcp__fixture__ping`.
    #[tokio::test]
    async fn connect_and_discover_fixture_exposes_ping_tool() {
        let script = fixture_server_script();
        assert!(
            script.is_file(),
            "missing fixture server at {}",
            script.display()
        );
        let result = connect_and_discover(&[fixture_mcp_config("fixture", "ping")])
            .await
            .expect("discover");
        let Some((manager, tools)) = result else {
            panic!("fixture server MUST produce a manager");
        };
        let names: Vec<_> = tools.iter().map(|t| t.name().to_string()).collect();
        assert!(
            names.iter().any(|n| n == "mcp__fixture__ping"),
            "expected mcp__fixture__ping, got {names:?}; diags={:?}",
            manager.diagnostics().await
        );
        assert!(
            manager
                .connected_servers()
                .await
                .iter()
                .any(|s| s.id == "fixture"),
            "fixture MUST show as connected"
        );
        let ping = tools
            .iter()
            .find(|t| t.name() == "mcp__fixture__ping")
            .expect("ping tool");
        let out = ping
            .execute(
                &crate::protocol::ports::XyToolCtx::new("call-1"),
                serde_json::json!({}),
            )
            .await
            .expect("ping execute");
        assert!(
            out.contains("pong"),
            "real MCP call MUST return fixture pong: {out}"
        );
        manager.shutdown().await;
    }

    /// Real stdio MCP: tools env change (add echo) is visible on rediscover.
    #[tokio::test]
    async fn connect_and_discover_fixture_can_add_echo_tool() {
        let result = connect_and_discover(&[fixture_mcp_config("fixture", "ping,echo")])
            .await
            .expect("discover");
        let Some((manager, tools)) = result else {
            panic!("fixture server MUST produce a manager");
        };
        let names: Vec<_> = tools.iter().map(|t| t.name().to_string()).collect();
        assert!(names.iter().any(|n| n == "mcp__fixture__ping"), "{names:?}");
        assert!(names.iter().any(|n| n == "mcp__fixture__echo"), "{names:?}");
        manager.shutdown().await;
    }
}
