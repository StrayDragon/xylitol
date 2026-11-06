use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_app_tui::driver::{LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot};

pub struct McpCueBdd {
    pub snapshot: RefCell<LoadedResourcesSnapshot>,
}

#[fixture]
pub fn mcp_cue_bdd() -> McpCueBdd {
    McpCueBdd {
        snapshot: RefCell::new(LoadedResourcesSnapshot::default()),
    }
}

fn server(id: &str, phase: McpServerPhase, armed: bool) -> McpServerSnapshot {
    McpServerSnapshot {
        id: id.into(),
        phase,
        tools_armed: armed,
        tool_count: if armed { 2 } else { 0 },
    }
}

#[given("MCP 已 connected 且 armed 但 connecting_label 残留")]
fn g_settled_with_stale_label(mcp_cue_bdd: &McpCueBdd) {
    *mcp_cue_bdd.snapshot.borrow_mut() = LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_connected: vec![("fixture".into(), 2)],
        mcp_servers: vec![server("fixture", McpServerPhase::Connected, true)],
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        mcp_connecting_label: Some("connecting 0/1".into()),
        ..Default::default()
    };
}

#[when("刷新 loaded-resources 快照")]
fn w_refresh(mcp_cue_bdd: &McpCueBdd) {
    let _ = mcp_cue_bdd;
}

#[then("cue 清除")]
fn t_cue_cleared(mcp_cue_bdd: &McpCueBdd) {
    assert!(!mcp_cue_bdd.snapshot.borrow().mcp_tools_pending());
}

#[given("gate timeout 后 tools frozen")]
fn g_gate_timeout(mcp_cue_bdd: &McpCueBdd) {
    *mcp_cue_bdd.snapshot.borrow_mut() = LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_servers: vec![server("fixture", McpServerPhase::Failed, false)],
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        mcp_gate_notice: Some("session prepare timed out".into()),
        mcp_connecting_label: Some("connecting 0/1".into()),
        ..Default::default()
    };
}

#[then("cue 清除且诊断保留")]
fn t_timeout_settled(mcp_cue_bdd: &McpCueBdd) {
    let snapshot = mcp_cue_bdd.snapshot.borrow();
    assert!(!snapshot.mcp_tools_pending());
    assert!(snapshot.mcp_gate_notice.is_some());
}

#[given("挂死 server 超过 connect_timeout_ms")]
fn g_hanging_server(mcp_cue_bdd: &McpCueBdd) {
    *mcp_cue_bdd.snapshot.borrow_mut() = LoadedResourcesSnapshot {
        mcp_configured: 1,
        mcp_servers: vec![server("fixture", McpServerPhase::Failed, false)],
        mcp_diag_short: vec!["fixture: connect timed out after 2s".into()],
        mcp_bootstrap_complete: true,
        tools_table_frozen: true,
        ..Default::default()
    };
}

#[then("header 与面板一致且不再 connecting")]
fn t_hanging_settled(mcp_cue_bdd: &McpCueBdd) {
    let snapshot = mcp_cue_bdd.snapshot.borrow();
    assert!(snapshot.mcp_connecting_label.is_none());
    assert!(!snapshot.mcp_tools_pending());
    assert!(
        snapshot
            .mcp_servers
            .iter()
            .all(|server| server.phase != McpServerPhase::Connecting)
    );
}
