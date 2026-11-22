use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use xylitol_plugin_sdk::plugin_events::{
    SessionPrepare, SessionPrepareReason, SessionPrepareResult,
};
use xylitol_plugin_sdk::rt::Ctx;
use xylitol_plugin_sdk::session_prepare::{SessionPrepareHub, SessionPrepareView};
use xylitol_plugin_sdk::tool_name::is_mcp_tool_name;

use super::reload::ReloadRegistry;
use super::types::{
    LoadedResourcesSnapshot, McpServerPhase, McpServerSnapshot, RuntimeReloadReport,
};
use super::{OverlayPhase, XyDriverError, XyInProcessDriver};
use crate::agent::MCP_FIRST_TURN_GATE_TIMEOUT;
use crate::agent::capabilities::ToolsWriteOutcome;
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
        self.handle.shared().set_prepare_hub(Arc::clone(&hub));
        self.prepare_hub = Some(hub);
    }

    pub fn set_loop_middleware(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
    ) {
        self.state_mut().runtime_mut().set_loop_middleware(registry);
    }

    pub fn set_loop_middleware_scoped(
        &mut self,
        registry: Option<Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>>,
        scope: xylitol_plugin_sdk::LoopScope,
    ) {
        self.state_mut()
            .runtime_mut()
            .set_loop_middleware_scoped(registry, scope);
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
        self.handle.shared().set_reload_cwd(cwd.clone());
        self.reload_cwd = Some(cwd);
        self.reload_agent_dir = Some(agent_dir);
    }

    pub fn reload_cwd(&self) -> Option<&std::path::PathBuf> {
        self.reload_cwd.as_ref()
    }

    pub fn set_reload_config_path(&mut self, path: Option<std::path::PathBuf>) {
        self.reload_config_path = path;
    }

    pub fn reload_config_path(&self) -> Option<&std::path::PathBuf> {
        self.reload_config_path.as_ref()
    }

    pub fn set_reload_registry(&mut self, registry: Arc<ReloadRegistry>) {
        self.reload_registry = registry;
    }

    pub fn ctx(&self) -> Option<&xylitol_plugin_sdk::rt::Ctx> {
        self.ctx.as_ref()
    }

    pub fn reload_registry(&self) -> &Arc<ReloadRegistry> {
        &self.reload_registry
    }

    pub fn reload_agent_dir(&self) -> Option<&std::path::PathBuf> {
        self.reload_agent_dir.as_ref()
    }

    pub fn set_session_profile(&mut self, profile: xylitol_plugin_sdk::session::SessionProfile) {
        self.handle.shared().set_session_profile(profile.clone());
        self.session_profile = profile;
    }

    pub fn take_profile_mismatch_notice(&mut self) -> Option<String> {
        self.handle
            .shared()
            .take_profile_mismatch_notice()
            .or_else(|| self.profile_mismatch_notice.take())
    }

    pub fn set_trust_audit_config(&mut self, config: crate::trust::TrustAuditConfig) {
        self.trust_audit = config;
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
        for detail in &view.server_details {
            line.push_str(&format!("\n{detail}"));
        }
        if !view.diags.is_empty() {
            line.push_str(&format!("\ndiagnostics: {}", view.diags.join("; ")));
        }
        Some(line)
    }

    pub fn fire_session_prepare(&mut self, reason: SessionPrepareReason) {
        self.prepare_acked = false;
        self.handle.shared().set_prepare_acked(false);
        if reason != SessionPrepareReason::Generate
            && let Some(hub) = &self.prepare_hub
        {
            hub.cue_connecting();
        }
        let session_id = self
            .snapshot()
            .session_id
            .as_deref()
            .unwrap_or("")
            .to_string();
        let workspaces = {
            let snapshot = self.snapshot();
            let mut workspaces = snapshot.workspaces.clone();
            if workspaces.is_empty() && !snapshot.cwd.trim().is_empty() {
                workspaces.push(snapshot.cwd);
            }
            workspaces
        };
        if let Some(ctx) = &self.ctx {
            let ev = SessionPrepare {
                session_id,
                reason,
                workspaces,
            };
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
        let session_id = self
            .snapshot()
            .session_id
            .as_deref()
            .unwrap_or("")
            .to_string();
        let ev = SessionPrepare {
            session_id,
            reason: SessionPrepareReason::Generate,
            workspaces: {
                let snapshot = self.snapshot();
                let mut workspaces = snapshot.workspaces.clone();
                if workspaces.is_empty() && !snapshot.cwd.trim().is_empty() {
                    workspaces.push(snapshot.cwd);
                }
                workspaces
            },
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
        self.finish_prepare(&result).await;
        result
    }

    pub async fn poll_tool_gate(&mut self) -> bool {
        if self.snapshot().tools_frozen {
            self.tool_gate_poll_started = None;
            return true;
        }
        self.state_mut().runtime_mut().begin_tool_gating();
        let started = *self.tool_gate_poll_started.get_or_insert(Instant::now());
        let settled = self.prepare_hub.as_ref().is_none_or(|hub| {
            let view = hub.view();
            view.configured == 0 || view.complete
        });
        if !settled && started.elapsed() < MCP_FIRST_TURN_GATE_TIMEOUT {
            return false;
        }
        if !settled {
            self.mcp_gate_notice = Some(xylitol_plugin_sdk::SESSION_PREPARE_TIMEOUT_NOTICE.into());
        }
        let result = self.hub_snapshot_result();
        self.settle_tool_gate(result).await;
        self.tool_gate_poll_started = None;
        self.snapshot().tools_frozen
    }

    async fn finish_prepare(&mut self, result: &SessionPrepareResult) {
        self.apply_prepare_tools().await;
        if !result.fingerprint.is_empty() {
            self.state_mut()
                .runtime_mut()
                .set_pending_prepare(xylitol_plugin_sdk::SessionPrepareSnapshot::from(result));
        }
    }

    async fn settle_tool_gate(&mut self, result: SessionPrepareResult) {
        self.finish_prepare(&result).await;
        let tools = self.state().runtime().clone_tools();
        self.state_mut().runtime_mut().freeze_tools(tools);
        let frozen = self.state().runtime().is_tools_frozen();
        self.handle.shared().set_tools_frozen(frozen);
        self.prepare_acked = true;
        self.handle.shared().set_prepare_acked(true);
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

    pub(super) async fn apply_prepare_tools(&mut self) {
        let Some(hub) = &self.prepare_hub else {
            return;
        };
        let mcp_tools = hub.tools();
        if mcp_tools.is_empty() {
            return;
        }
        if self.snapshot().tools_frozen {
            return;
        }
        let builtins: Vec<_> = self
            .base_tools
            .as_ref()
            .map(|set| set.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
            .filter(|t| !is_mcp_tool_name(t.name()))
            .collect();
        let set = ToolSet::rebuild_agent_tools(builtins, mcp_tools);
        self.base_tools = Some(set.clone());
        let filtered =
            crate::agent::tools::filter_tools_by_capability_overlay(set, &self.capability_disabled);
        let applied = self
            .actor
            .as_mut()
            .map(|state| state.runtime_mut().set_tools(filtered.clone()))
            .unwrap_or(ToolsWriteOutcome::RejectedMidTurn);
        match applied {
            ToolsWriteOutcome::Applied => {}
            ToolsWriteOutcome::RejectedMidTurn | ToolsWriteOutcome::RejectedFrozen => {
                log::warn!(
                    target: "xylitol::agent",
                    "prepare tools overlay not applied (mid-turn or frozen)"
                );
            }
        }
        self.capability_applied = self.capability_disabled.clone();
        self.overlay_phase = OverlayPhase::Idle;
    }

    pub(super) async fn prepare_loaded_resources(&self) -> LoadedResourcesSnapshot {
        let skill_names = self.loaded_skill_names();
        let snapshot = self.snapshot();
        let tools_table_frozen = snapshot.tools_frozen;
        let tool_names = snapshot.tool_names;
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
        let settled_now = view.complete || (tools_table_frozen && self.prepare_acked);
        if settled_now {
            self.handle.shared().mark_bootstrap_settled();
        }
        let settled = self.handle.shared().bootstrap_settled();
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
                } else if failed || settled {
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
        let (mcp_diag_short, mcp_details) = mcp_diagnostic_lines(&view);
        LoadedResourcesSnapshot {
            skill_names,
            mcp_connected: view.connected,
            mcp_configured: view.configured,
            mcp_diag_short,
            mcp_details,
            mcp_connecting_label: if settled { None } else { view.connecting_label },
            mcp_servers,
            mcp_bootstrap_complete: view.complete || settled,
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

fn mcp_diagnostic_lines(view: &SessionPrepareView) -> (Vec<String>, Vec<String>) {
    (
        view.diags.clone(),
        view.probe_lines
            .iter()
            .chain(view.server_details.iter())
            .cloned()
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_diagnostics_split_short_and_details() {
        let view = SessionPrepareView {
            diags: vec!["bad: spawn failed".into()],
            probe_lines: vec!["probe server=bad status=unsupported".into()],
            server_details: vec!["server=bad transport=Stdio scope=Workspace".into()],
            ..SessionPrepareView::default()
        };
        let (short, details) = mcp_diagnostic_lines(&view);
        assert_eq!(short, vec!["bad: spawn failed"]);
        assert!(details.iter().any(|line| line.contains("server=bad")));
        assert!(!short.iter().any(|line| line.contains("server=")));
    }
}

#[cfg(test)]
mod tool_gate_poll_tests {
    use super::XyInProcessDriver;
    use crate::agent::runtime::baseline::RuntimeBaseline;
    use crate::test_support::{MemoryStore, NoopSink};
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use xylitol_plugin_sdk::plugin_events::SessionPrepareResult;
    use xylitol_plugin_sdk::{SessionPrepareHub, SessionPrepareView, XySessionStore};

    struct CountingHub {
        ensure_calls: AtomicUsize,
        wait_calls: AtomicUsize,
        complete: AtomicBool,
    }

    impl CountingHub {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                ensure_calls: AtomicUsize::new(0),
                wait_calls: AtomicUsize::new(0),
                complete: AtomicBool::new(false),
            })
        }

        fn set_complete(&self, complete: bool) {
            self.complete.store(complete, Ordering::SeqCst);
        }

        fn drive_calls(&self) -> usize {
            self.ensure_calls.load(Ordering::SeqCst) + self.wait_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl SessionPrepareHub for CountingHub {
        fn fire(&self) {}

        async fn reset_join(&self) {}

        async fn ensure(&self) -> SessionPrepareResult {
            self.ensure_calls.fetch_add(1, Ordering::SeqCst);
            SessionPrepareResult::default()
        }

        async fn wait(&self, _timeout: Duration) -> SessionPrepareResult {
            self.wait_calls.fetch_add(1, Ordering::SeqCst);
            SessionPrepareResult::default()
        }

        fn view(&self) -> SessionPrepareView {
            SessionPrepareView {
                configured: 2,
                complete: self.complete.load(Ordering::SeqCst),
                ..SessionPrepareView::default()
            }
        }

        fn take_notice(&self) -> Option<String> {
            None
        }

        fn tools(&self) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
            Vec::new()
        }
    }

    fn poll_test_driver(hub: Arc<dyn SessionPrepareHub>) -> XyInProcessDriver {
        let store: Arc<dyn XySessionStore> = Arc::new(MemoryStore::new());
        let baseline = RuntimeBaseline {
            store: store.clone(),
            sink: Arc::new(NoopSink),
            permission: crate::permission::allow_all_permission(),
            hook_bus: None,
            model_registry: crate::agent::ModelRegistry::new(),
            model_builder: crate::test_support::dummy_model_builder(),
            cwd: ".".into(),
            steering_mode: crate::agent::capabilities::QueueMode::default(),
            follow_up_mode: crate::agent::capabilities::QueueMode::default(),
            compaction_settings: None,
            compaction_factory: xylitol_plugin_sdk::noop_compaction_factory(),
            tools: crate::agent::tools::ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            batch_mode: xylitol_plugin_sdk::XyBatchMode::Sequential,
        };
        let mut driver = XyInProcessDriver::new(baseline.materialize_runtime(), store);
        driver.set_prepare_hub(hub);
        driver
    }

    #[tokio::test]
    async fn poll_tool_gate_passive_poll_never_drives_prepare() {
        let hub = CountingHub::new();
        let mut driver = poll_test_driver(hub.clone());
        for _ in 0..5 {
            assert!(!driver.poll_tool_gate().await);
        }
        assert_eq!(
            hub.drive_calls(),
            0,
            "passive poll MUST NOT call ensure/wait"
        );
        assert!(!driver.state().runtime().is_tools_frozen());
    }

    #[tokio::test]
    async fn poll_tool_gate_settles_when_view_completes() {
        let hub = CountingHub::new();
        let mut driver = poll_test_driver(hub.clone());
        assert!(!driver.poll_tool_gate().await);
        hub.set_complete(true);
        assert!(driver.poll_tool_gate().await);
        assert!(driver.state().runtime().is_tools_frozen());
        assert_eq!(hub.drive_calls(), 0);
    }

    #[tokio::test]
    async fn poll_tool_gate_settles_once_with_notice_at_gate_deadline() {
        let hub = CountingHub::new();
        let mut driver = poll_test_driver(hub.clone());
        assert!(!driver.poll_tool_gate().await);
        driver.tool_gate_poll_started = Some(Instant::now() - Duration::from_secs(16));
        assert!(driver.poll_tool_gate().await);
        assert!(driver.state().runtime().is_tools_frozen());
        assert!(
            driver.mcp_gate_notice.is_some(),
            "gate deadline MUST surface the timeout notice once"
        );
        assert_eq!(hub.drive_calls(), 0);
    }
}
