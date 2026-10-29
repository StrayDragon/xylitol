use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::plugin_events::{
    SessionPrepare, SessionPrepareReason, SessionPrepareResult,
};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session_prepare::{SessionPrepareHub, SessionPrepareView};
use xylitol_plugin_sdk::tool_name::is_mcp_tool_name;

use super::types::{
    LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot, RuntimeReloadReport,
};
use super::{XyDriverError, XyInProcessDriver};
use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;
use crate::agent::tools::ToolSet;

#[async_trait]
pub trait InProcessReloadHook: Send + Sync {
    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        cancel: &CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError>;
}

impl XyInProcessDriver {
    pub fn attach_ctx(&mut self, ctx: Ctx) {
        self.ctx = Some(ctx);
    }

    pub fn set_prepare_hub(&mut self, hub: Arc<dyn SessionPrepareHub>) {
        self.prepare_hub = Some(hub);
    }

    pub fn set_loop_middleware(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    ) {
        self.agent.set_loop_middleware(registry);
    }

    pub fn set_loop_middleware_scoped(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        scope: xylitol_plugin_sdk::LoopScope,
    ) {
        self.agent.set_loop_middleware_scoped(registry, scope);
    }

    pub fn set_reload_hook(&mut self, hook: Arc<dyn InProcessReloadHook>) {
        self.reload_hook = Some(hook);
    }

    pub fn enable_reload_state(
        &mut self,
        cwd: std::path::PathBuf,
        agent_dir: std::path::PathBuf,
        _project_trusted: bool,
    ) {
        self.reload_cwd = Some(cwd);
        self.reload_agent_dir = Some(agent_dir);
    }

    pub fn reload_cwd(&self) -> Option<&std::path::PathBuf> {
        self.reload_cwd.as_ref()
    }

    pub fn reload_agent_dir(&self) -> Option<&std::path::PathBuf> {
        self.reload_agent_dir.as_ref()
    }

    pub async fn mcp_status_summary(&self) -> Option<String> {
        let view = match &self.prepare_hub {
            Some(hub) => hub.view(),
            None => return None,
        };
        if view.configured == 0 {
            return None;
        }
        let ids: Vec<_> = view.connected.iter().map(|(id, _)| id.as_str()).collect();
        let mut line = format!(
            "MCP: {} configured, {} connected [{}]",
            view.configured,
            view.connected.len(),
            ids.join(", ")
        );
        if !view.diags.is_empty() {
            line.push_str(&format!("; diagnostics: {}", view.diags.join("; ")));
        }
        Some(line)
    }

    pub fn fire_session_prepare(&mut self, reason: SessionPrepareReason) {
        self.prepare_acked = false;
        if reason != SessionPrepareReason::Generate
            && let Some(hub) = &self.prepare_hub
        {
            hub.cue_connecting();
        }
        let session_id = self.agent.session_id().unwrap_or("").to_string();
        if let Some(ctx) = &self.ctx {
            let ev = SessionPrepare { session_id, reason };
            let _ = ctx.emit(ev);
            return;
        }
        if let Some(hub) = &self.prepare_hub
            && reason != SessionPrepareReason::Generate
        {
            hub.fire();
        }
    }

    pub(super) async fn join_session_prepare(&mut self) -> SessionPrepareResult {
        let session_id = self.agent.session_id().unwrap_or("").to_string();
        let ev = SessionPrepare {
            session_id,
            reason: SessionPrepareReason::Generate,
        };
        let timeout = MCP_FIRST_TURN_GATE_TIMEOUT;
        let result = if let Some(ctx) = self.ctx.clone() {
            match tokio::time::timeout(timeout, ctx.waterfall(ev)).await {
                Ok(Ok(Some(r))) => r,
                Ok(Ok(None)) | Ok(Err(_)) => self.hub_wait(timeout).await,
                Err(_) => {
                    self.mcp_gate_notice =
                        Some(xylitol_plugin_sdk::SESSION_PREPARE_TIMEOUT_NOTICE.into());
                    self.hub_snapshot_result()
                }
            }
        } else {
            self.hub_wait(timeout).await
        };
        self.apply_prepare_tools();
        if !result.fingerprint.is_empty() {
            self.agent
                .set_pending_prepare(xylitol_plugin_sdk::SessionPrepareSnapshot::from(&result));
        }
        result
    }

    async fn hub_wait(&self, timeout: Duration) -> SessionPrepareResult {
        match &self.prepare_hub {
            Some(hub) => hub.wait(timeout).await,
            None => SessionPrepareResult::default(),
        }
    }

    fn hub_snapshot_result(&self) -> SessionPrepareResult {
        match &self.prepare_hub {
            Some(hub) => hub.view().armed_result(),
            None => SessionPrepareResult::default(),
        }
    }

    pub(super) fn apply_prepare_tools(&mut self) {
        let Some(hub) = &self.prepare_hub else {
            return;
        };
        let mcp_tools = hub.tools();
        if mcp_tools.is_empty() {
            return;
        }
        if self.agent.is_tools_frozen() {
            return;
        }
        let builtins: Vec<_> = self
            .agent
            .clone_tools()
            .iter()
            .filter(|t| !is_mcp_tool_name(t.name()))
            .cloned()
            .collect();
        let set = ToolSet::rebuild_agent_tools(builtins, mcp_tools);
        self.base_tools = Some(set.clone());
        let filtered =
            crate::agent::tools::filter_tools_by_capability_overlay(set, &self.capability_disabled);
        self.agent.set_tools(filtered);
        self.capability_applied = self.capability_disabled.clone();
    }

    pub(super) async fn prepare_loaded_resources(&self) -> LoadedResourcesSnapshot {
        let skill_names = self.loaded_skill_names();
        let tools_table_frozen = self.agent.is_tools_frozen();
        let tool_names = self.agent.tool_names();
        let view = match &self.prepare_hub {
            Some(hub) => hub.view(),
            None => {
                return LoadedResourcesSnapshot {
                    skill_names,
                    tools_table_frozen,
                    mcp_bootstrap_complete: true,
                    mcp_gate_notice: self.mcp_gate_notice.clone(),
                    ..LoadedResourcesSnapshot::default()
                };
            }
        };
        self.snapshot_from_view(skill_names, tools_table_frozen, tool_names, view)
    }

    fn snapshot_from_view(
        &self,
        skill_names: Vec<String>,
        tools_table_frozen: bool,
        tool_names: Vec<String>,
        view: SessionPrepareView,
    ) -> LoadedResourcesSnapshot {
        let mcp_servers = view
            .server_ids
            .iter()
            .map(|id| {
                let connected = view.connected.iter().any(|(c, _)| c == id);
                let failed = view.diags.iter().any(|d| d.starts_with(id));
                let prefix = xylitol_plugin_sdk::mcp_tool_armed_prefix(id);
                let armed_count = tool_names.iter().filter(|n| n.starts_with(&prefix)).count();
                let phase = if connected {
                    McpServerPhase::Connected
                } else if failed {
                    McpServerPhase::Failed
                } else if view.connecting {
                    McpServerPhase::Connecting
                } else {
                    McpServerPhase::Failed
                };
                McpServerSnapshot {
                    id: id.clone(),
                    phase,
                    tools_armed: armed_count > 0,
                    tool_count: view
                        .connected
                        .iter()
                        .find(|(c, _)| c == id)
                        .map(|(_, n)| *n)
                        .unwrap_or(armed_count),
                }
            })
            .collect();
        LoadedResourcesSnapshot {
            skill_names,
            mcp_connected: view.connected,
            mcp_configured: view.configured,
            mcp_diag_short: view.diags,
            mcp_connecting_label: view.connecting_label,
            mcp_servers,
            mcp_bootstrap_complete: view.complete,
            tools_table_frozen,
            mcp_gate_notice: self.mcp_gate_notice.clone(),
            obs_diag: None,
        }
    }

    pub(super) fn prepare_poll_dirty(&mut self) -> bool {
        let Some(hub) = &self.prepare_hub else {
            return false;
        };
        let view = hub.view();
        let current = view.connecting_label.clone();
        if self.last_ui_mcp_label.as_ref() == Some(&current) {
            return false;
        }
        self.last_ui_mcp_label = Some(current);
        true
    }
}
