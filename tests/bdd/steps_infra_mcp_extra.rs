use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{then, when};

pub struct T6McpBdd {
    pub tool_names: RefCell<Vec<String>>,
    pub diag_count: RefCell<usize>,
    pub connected: RefCell<usize>,
}

#[fixture]
pub fn t6_mcp_bdd() -> T6McpBdd {
    T6McpBdd {
        tool_names: RefCell::new(Vec::new()),
        diag_count: RefCell::new(0),
        connected: RefCell::new(0),
    }
}

fn t6_fixture_config(name: &str, tools: &str) -> xylitol_kernel::config::types::McpServerConfig {
    let mut env = std::collections::HashMap::new();
    env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), tools.into());
    xylitol_kernel::config::types::McpServerConfig {
        name: name.into(),
        transport: xylitol_kernel::config::types::McpTransportKind::Stdio,
        command: Some("python3".into()),
        args: Some(vec![
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/support/mcp_fixture_server.py")
                .display()
                .to_string(),
        ]),
        env: Some(env),
        ..Default::default()
    }
}

async fn t6_discover(
    t6_mcp_bdd: &T6McpBdd,
    servers: &[xylitol_kernel::config::types::McpServerConfig],
) {
    let result = xylitol_official_plugin_mcp::connect_and_discover(servers).await;
    let Some((manager, tools)) = result else {
        panic!("c2829: 非空配置 MUST 构造 manager");
    };
    *t6_mcp_bdd.tool_names.borrow_mut() = tools.iter().map(|t| t.name().to_string()).collect();
    *t6_mcp_bdd.diag_count.borrow_mut() = manager.diagnostics().await.len();
    *t6_mcp_bdd.connected.borrow_mut() = manager.connected_servers().await.len();
    manager.shutdown().await;
}

#[when("以 fixture MCP 配置装配 ping 工具")]
pub(crate) async fn w_t6_mcp_ping(t6_mcp_bdd: &T6McpBdd) {
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "ping")]).await;
}

#[then("装配出 mcp 前缀的 XyTool 且连接在册")]
pub(crate) fn t_t6_mcp_tools(t6_mcp_bdd: &T6McpBdd) {
    let names = t6_mcp_bdd.tool_names.borrow().clone();
    assert!(
        names.iter().any(|n| n == "mcp__fixture__ping"),
        "c2829: fixture 应产出 mcp__fixture__ping：{names:?}"
    );
    assert!(
        *t6_mcp_bdd.connected.borrow() >= 1,
        "c2829: fixture server 应显示已连接"
    );
}

#[when("以两组不同工具集的 fixture 配置先后装配")]
pub(crate) async fn w_t6_mcp_config_driven(t6_mcp_bdd: &T6McpBdd) {
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "ping")]).await;
    let first = t6_mcp_bdd.tool_names.borrow().clone();
    t6_discover(t6_mcp_bdd, &[t6_fixture_config("fixture", "echo")]).await;
    let second = t6_mcp_bdd.tool_names.borrow().clone();
    assert!(
        first.iter().any(|n| n.contains("ping")) && !first.iter().any(|n| n.contains("echo")),
        "c2829: 第一组应有 ping 无 echo：{first:?}"
    );
    assert!(
        second.iter().any(|n| n.contains("echo")) && !second.iter().any(|n| n.contains("ping")),
        "c2829: 第二组应有 echo 无 ping：{second:?}"
    );
}

#[then("工具集随配置变化")]
pub(crate) fn t_t6_mcp_config_driven(_t6_mcp_bdd: &T6McpBdd) {}

#[when("以无效 MCP 条目装配")]
pub(crate) async fn w_t6_mcp_invalid(t6_mcp_bdd: &T6McpBdd) {
    let bad = xylitol_kernel::config::types::McpServerConfig {
        name: "bad".into(),
        transport: xylitol_kernel::config::types::McpTransportKind::Stdio,
        command: None,
        ..Default::default()
    };
    let result = xylitol_official_plugin_mcp::connect_and_discover(&[bad]).await;
    let Some((manager, tools)) = result else {
        panic!("c2829: 含无效条目的非空配置 MUST 仍构造 manager");
    };
    *t6_mcp_bdd.tool_names.borrow_mut() = tools.iter().map(|t| t.name().to_string()).collect();
    *t6_mcp_bdd.diag_count.borrow_mut() = manager.diagnostics().await.len();
    *t6_mcp_bdd.connected.borrow_mut() = manager.connected_servers().await.len();
    manager.shutdown().await;
}

#[then("诊断在册且不产出该条目工具")]
pub(crate) fn t_t6_mcp_invalid_diag(t6_mcp_bdd: &T6McpBdd) {
    assert!(
        *t6_mcp_bdd.diag_count.borrow() >= 1,
        "c2829: 无效条目应留下诊断"
    );
    assert!(
        t6_mcp_bdd.tool_names.borrow().is_empty(),
        "c2829: 无效条目 MUST NOT 产出工具"
    );
    assert_eq!(
        *t6_mcp_bdd.connected.borrow(),
        0,
        "c2829: 无效条目不应出现在已连接列表"
    );
}

#[then("已连接列表只读返回在册服务器摘要")]
pub(crate) fn t_mcp_connected_readonly(t6_mcp_bdd: &T6McpBdd) {
    let connected = *t6_mcp_bdd.connected.borrow();
    let tool_names = t6_mcp_bdd.tool_names.borrow();
    assert!(connected >= 1, "已连接摘要 MUST 返回在册服务器");
    assert!(!tool_names.is_empty(), "摘要 MUST 含工具数量或等价");
}
