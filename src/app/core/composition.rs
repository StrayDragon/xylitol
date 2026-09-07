//! Shared agent construction for all application entry points.

use std::sync::Arc;

use crate::agent::AgentBuilder;
use crate::agent::AgentRuntime;
use crate::agent::capabilities::QueueMode;
use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::tools::ToolSet;
use crate::app::core::driver_error::XyDriverError;
use crate::infra::config::types::HooksConfig;
use crate::infra::event::EventBus;
use crate::infra::hooks::HookDispatcher;
use crate::infra::permission;
use crate::infra::session::SessionManager;
use crate::protocol::ports::{
    XyBatchMode, XyEventSink, XyHookBus, XyModelBuilder, XyPermission, XySessionStore,
};

pub use crate::app::core::mcp_spec::{McpServerSpec, McpTransportSpec};

/// Options for [`build_agent`].
pub struct BuildAgentOptions {
    pub model_registry: ModelRegistry,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    /// Skills catalog for `<available_skills>` (c1085).
    pub skills: Vec<crate::protocol::resource::SkillInfo>,
    pub cwd: String,
    pub compaction_settings: Option<CompactionSettings>,
    pub permission: Option<Arc<dyn XyPermission>>,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    /// Optional lifecycle sink (compaction etc.). Default: in-process [`EventBus`].
    /// Turn UX still uses the `XyDriver::run` EventStream, not this bus.
    pub event_sink: Option<Arc<dyn XyEventSink>>,
    /// Three-tier script hook configuration (empty = zero-cost no-op).
    pub hooks_config: HooksConfig,
    /// Same-turn tool batch mode from `AppConfig.tool_batch` (c1545).
    pub batch_mode: XyBatchMode,
}

impl Default for BuildAgentOptions {
    fn default() -> Self {
        Self {
            model_registry: ModelRegistry::new(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            cwd: ".".into(),
            compaction_settings: None,
            permission: None,
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            event_sink: None,
            hooks_config: HooksConfig::default(),
            batch_mode: XyBatchMode::Sequential,
        }
    }
}

/// Construct a clonable [`crate::agent::RuntimePorts`] baseline from the given options.
///
/// Host processes share one baseline and lazy-materialize isolated Drivers per
/// session slot. Print/TUI embed still call [`build_agent`] (one actor).
pub fn build_ports(
    options: BuildAgentOptions,
) -> Result<crate::agent::RuntimePorts, XyDriverError> {
    let sessions_dir = SessionManager::default_dir();
    std::fs::create_dir_all(&sessions_dir).map_err(|e| format!("create sessions dir: {e}"))?;
    let session_mgr = SessionManager::new(sessions_dir);
    build_ports_with_store(options, Arc::new(session_mgr))
}

/// Like [`build_ports`], but injects an existing session store (tests / Host).
pub fn build_ports_with_store(
    options: BuildAgentOptions,
    store: Arc<dyn XySessionStore>,
) -> Result<crate::agent::RuntimePorts, XyDriverError> {
    let sink: Arc<dyn XyEventSink> = options
        .event_sink
        .unwrap_or_else(|| Arc::new(EventBus::new()));

    let hook_dispatcher = Arc::new(HookDispatcher::new(&options.hooks_config));
    let hooks_for_provider = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher.clone())
    };
    let hook_bus: Option<Arc<dyn XyHookBus>> = if hook_dispatcher.is_empty() {
        None
    } else {
        Some(hook_dispatcher)
    };

    let model_builder: XyModelBuilder = {
        let hooks = hooks_for_provider;
        Arc::new(move |config| {
            crate::infra::provider::factory::build_provider_with_hooks(config, hooks.clone())
        })
    };
    let permission = options
        .permission
        .unwrap_or_else(permission::allow_all_permission);

    let mut builder = AgentBuilder::new(
        options.model_registry,
        model_builder,
        store,
        sink,
        permission,
    )
    .tools(ToolSet::from_iter(crate::infra::tools::default_tools()))
    .context_files(options.context_files)
    .append_system_prompt(options.append_system_prompt)
    .skills(options.skills)
    .compaction_settings(options.compaction_settings)
    .cwd(options.cwd)
    .steering_mode(options.steering_mode)
    .follow_up_mode(options.follow_up_mode)
    .hook_bus(hook_bus)
    .batch_mode(options.batch_mode);

    if let Some(sp) = options.system_prompt {
        builder = builder.system_prompt(sp);
    }

    Ok(builder.build_ports())
}

/// Construct a fully-wired [`AgentRuntime`] from the given options.
///
/// This is the single composition-root helper used by CLI, RPC, server, and
/// future TUI/GUI modes. It injects the concrete infra implementations
/// (`SessionManager`, `XyEventSink`, bang/export via Driver) into the
/// agent without letting `agent/` know about `infra/` types.
///
/// **Event paths:** turn progress is the `XyDriver::run` → `XyEvent` stream.
/// The injected [`XyEventSink`] (default [`EventBus`]) is for side lifecycle
/// (e.g. compaction); it is not the multi-client turn bus.
pub fn build_agent(options: BuildAgentOptions) -> Result<AgentRuntime, XyDriverError> {
    Ok(build_ports(options)?.materialize_runtime())
}

/// Outcome of [`McpSession::reload`] (c1205).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpReloadOutcome {
    /// New tools installed (or empty→builtins re-freeze).
    Installed,
    /// Cancelled before install; previous manager/tools left untouched.
    Cancelled,
}

/// Owns MCP client connections for a local XyDriver session (composition seam).
///
/// Held by the surface (cli/server) so connections stay alive across turns and
/// can be shut down / replaced on reload without `XyDriver` importing infra.
pub struct McpSession {
    manager: Option<Arc<crate::infra::mcp::McpClientManager>>,
}

impl Default for McpSession {
    fn default() -> Self {
        Self::new()
    }
}

impl McpSession {
    pub fn new() -> Self {
        Self { manager: None }
    }

    #[cfg(test)]
    pub fn has_manager(&self) -> bool {
        self.manager.is_some()
    }

    /// Read-only connected MCP snapshot (c1080 / mcp5). Empty when no manager.
    pub async fn connected_servers(&self) -> Vec<crate::infra::mcp::ConnectedMcpServer> {
        match &self.manager {
            Some(m) => m.connected_servers().await,
            None => Vec::new(),
        }
    }

    /// Diagnostics from the last connect/reload (c1080 / mcp4).
    pub async fn diagnostics(&self) -> Vec<crate::infra::mcp::McpConnectDiagnostic> {
        match &self.manager {
            Some(m) => m.diagnostics().await,
            None => Vec::new(),
        }
    }

    /// Install a freshly discovered manager (c1200). Caller updates driver tools separately
    /// when borrow-split is required.
    pub fn set_manager(&mut self, manager: Arc<crate::infra::mcp::McpClientManager>) {
        self.manager = Some(manager);
    }

    pub fn take_manager(&mut self) -> Option<Arc<crate::infra::mcp::McpClientManager>> {
        self.manager.take()
    }

    /// Reload MCP tools onto `driver` (empty servers → builtins only, zero-cost).
    ///
    /// c1900: reopens the freeze gate and **re-freezes** via upsert rebuild (not silent
    /// mid-session `set_tools` expand).
    ///
    /// c1205: keep the old manager until the new one is ready; on `cancel`, leave the
    /// previous manager/tools untouched.
    pub async fn reload(
        &mut self,
        driver: &mut crate::app::core::driver::XyInProcessDriver,
        servers: &[McpServerSpec],
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<McpReloadOutcome, XyDriverError> {
        use crate::infra::mcp::{connect_and_discover, mcp_enabled};

        if cancel.is_cancelled() {
            return Ok(McpReloadOutcome::Cancelled);
        }

        let infra = McpServerSpec::to_infra_list(servers);
        let builtins = driver.builtins_for_reload();

        let discovered = if mcp_enabled(&Some(infra.clone())) {
            let discover = connect_and_discover(&infra);
            tokio::pin!(discover);
            tokio::select! {
                biased;
                () = cancel.cancelled() => {
                    return Ok(McpReloadOutcome::Cancelled);
                }
                result = &mut discover => result,
            }
        } else {
            None
        };

        if cancel.is_cancelled() {
            // Discover finished but user cancelled before install — drop orphan manager.
            if let Some((manager, _)) = discovered {
                manager.shutdown().await;
            }
            return Ok(McpReloadOutcome::Cancelled);
        }

        // Install only after discover succeeds (or empty path).
        driver.reopen_tools_for_regate();
        let mut tools = ToolSet::from_iter(builtins.clone());
        let prev = self.manager.take();
        if let Some((manager, mcp_tools)) = discovered {
            tools = ToolSet::rebuild_agent_tools(builtins, mcp_tools);
            self.manager = Some(manager);
        }
        driver.freeze_tools(tools);
        if let Some(old) = prev {
            old.shutdown().await;
        }
        Ok(McpReloadOutcome::Installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::mcp::mcp_enabled;

    #[tokio::test]
    async fn reload_empty_is_zero_cost_no_manager() {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        let mut mcp = McpSession::new();
        mcp.reload(
            &mut driver,
            &[],
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(!mcp.has_manager());
        assert!(!mcp_enabled(&Some(vec![])));
        let names: Vec<_> = driver.tool_names_for_test();
        assert!(names.iter().all(|n| !crate::protocol::is_mcp_tool_name(n)));
        assert!(names.iter().any(|n| n == "read"));
        assert!(mcp.connected_servers().await.is_empty());
        assert!(mcp.diagnostics().await.is_empty());
    }

    #[tokio::test]
    async fn reload_pre_cancelled_preserves_manager_and_tools() {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        let before = driver.tool_names_for_test();

        let mut mcp = McpSession::new();
        let placeholder = Arc::new(crate::infra::mcp::McpClientManager::new());
        mcp.set_manager(Arc::clone(&placeholder));
        assert!(mcp.has_manager());

        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let outcome = mcp
            .reload(&mut driver, &[], &cancel)
            .await
            .expect("pre-cancelled reload");
        assert_eq!(outcome, McpReloadOutcome::Cancelled);
        assert!(
            mcp.has_manager(),
            "cancel before install MUST keep previous manager"
        );
        assert_eq!(
            driver.tool_names_for_test(),
            before,
            "cancel MUST NOT reopen/freeze tools"
        );
    }

    #[tokio::test]
    async fn reload_preserves_tui_ask_tool() {
        use crate::protocol::error::XyToolError;
        use crate::protocol::ports::ask::{AskArgs, AskUserGateway};
        use async_trait::async_trait;

        struct SkipGateway;
        #[async_trait]
        impl AskUserGateway for SkipGateway {
            async fn prompt(&self, _args: AskArgs) -> Result<String, XyToolError> {
                Ok(r#"{"status":"skipped","answers":[]}"#.into())
            }
        }

        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        driver.install_ask_tool(Arc::new(SkipGateway));
        assert!(driver.tool_names_for_test().iter().any(|n| n == "ask"));

        let mut mcp = McpSession::new();
        mcp.reload(
            &mut driver,
            &[],
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        let names = driver.tool_names_for_test();
        assert!(
            names.iter().any(|n| n == "ask"),
            "ask must survive MCP reload"
        );
        assert!(names.iter().any(|n| n == "read"));
    }

    /// c1205 / c1900 lab: add→remove MCP-like tools through the same
    /// reopen + rebuild + freeze path `McpSession::reload` uses on install.
    #[tokio::test]
    async fn lab_reload_add_remove_updates_provider_tool_names() {
        use crate::agent::tools::ToolSet;
        use crate::protocol::error::XyToolError;
        use crate::protocol::ports::{XyTool, XyToolCtx};
        use async_trait::async_trait;

        struct StubMcpTool {
            name: &'static str,
        }
        #[async_trait]
        impl XyTool for StubMcpTool {
            fn name(&self) -> &str {
                self.name
            }
            fn description(&self) -> &str {
                "stub mcp"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object", "properties": {}})
            }
            async fn execute(
                &self,
                _ctx: &XyToolCtx,
                _args: serde_json::Value,
            ) -> Result<String, XyToolError> {
                Ok("ok".into())
            }
        }

        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);

        let builtins = driver.builtins_for_reload();
        let with_a = ToolSet::rebuild_agent_tools(
            builtins.clone(),
            vec![Arc::new(StubMcpTool {
                name: "mcp__demo__alpha",
            }) as Arc<dyn XyTool>],
        );
        driver.freeze_tools(with_a);
        assert!(driver.is_tools_frozen());
        assert!(
            driver
                .tool_names_for_test()
                .iter()
                .any(|n| n == "mcp__demo__alpha"),
            "precondition: alpha armed in provider table"
        );

        // User removes MCP server from config → empty discover → install builtins only.
        let mut mcp = McpSession::new();
        mcp.set_manager(Arc::new(crate::infra::mcp::McpClientManager::new()));
        let outcome = mcp
            .reload(
                &mut driver,
                &[],
                &tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("reload empty");
        assert_eq!(outcome, McpReloadOutcome::Installed);
        assert!(driver.is_tools_frozen());
        let after_remove = driver.tool_names_for_test();
        assert!(
            !after_remove.iter().any(|n| n == "mcp__demo__alpha"),
            "remove path MUST drop stale MCP tool from provider table: {after_remove:?}"
        );
        assert!(
            after_remove.iter().any(|n| n == "read"),
            "builtins MUST remain after remove reload"
        );
        assert!(
            !mcp.has_manager(),
            "empty reload clears previous manager after install"
        );

        // User adds a different MCP tool (simulate install overlay without live MCP).
        driver.reopen_tools_for_regate();
        let with_b = ToolSet::rebuild_agent_tools(
            driver.builtins_for_reload(),
            vec![Arc::new(StubMcpTool {
                name: "mcp__demo__beta",
            }) as Arc<dyn XyTool>],
        );
        driver.freeze_tools(with_b);
        let after_add = driver.tool_names_for_test();
        assert!(
            after_add.iter().any(|n| n == "mcp__demo__beta"),
            "add path MUST expose new MCP tool: {after_add:?}"
        );
        assert!(
            !after_add.iter().any(|n| n == "mcp__demo__alpha"),
            "add path MUST NOT revive removed alpha: {after_add:?}"
        );

        // Without /reload reopen, FROZEN ignores silent expand (config change alone).
        let n = after_add.len();
        driver.set_tools(ToolSet::rebuild_agent_tools(
            driver.builtins_for_reload(),
            vec![
                Arc::new(StubMcpTool {
                    name: "mcp__demo__beta",
                }) as Arc<dyn XyTool>,
                Arc::new(StubMcpTool {
                    name: "mcp__demo__gamma",
                }) as Arc<dyn XyTool>,
            ],
        ));
        assert_eq!(
            driver.tool_names_for_test().len(),
            n,
            "FROZEN set_tools MUST NOT expand provider tools without reopen/freeze"
        );
    }

    /// Cancel before install: provider table stays on the pre-reload freeze epoch.
    #[tokio::test]
    async fn lab_reload_cancel_keeps_provider_tools_despite_desired_add() {
        use crate::agent::tools::ToolSet;
        use crate::protocol::error::XyToolError;
        use crate::protocol::ports::{XyTool, XyToolCtx};
        use async_trait::async_trait;

        struct StubMcpTool;
        #[async_trait]
        impl XyTool for StubMcpTool {
            fn name(&self) -> &str {
                "mcp__keep__me"
            }
            fn description(&self) -> &str {
                "stub"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object", "properties": {}})
            }
            async fn execute(
                &self,
                _ctx: &XyToolCtx,
                _args: serde_json::Value,
            ) -> Result<String, XyToolError> {
                Ok("ok".into())
            }
        }

        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        driver.freeze_tools(ToolSet::rebuild_agent_tools(
            driver.builtins_for_reload(),
            vec![Arc::new(StubMcpTool) as Arc<dyn XyTool>],
        ));
        let before = driver.tool_names_for_test();

        let mut mcp = McpSession::new();
        mcp.set_manager(Arc::new(crate::infra::mcp::McpClientManager::new()));
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let outcome = mcp.reload(&mut driver, &[], &cancel).await.unwrap();
        assert_eq!(outcome, McpReloadOutcome::Cancelled);
        assert_eq!(driver.tool_names_for_test(), before);
        assert!(driver.is_tools_frozen());
        assert!(
            before.iter().any(|n| n == "mcp__keep__me"),
            "cancel MUST keep prior MCP tool visible to provider"
        );
    }

    fn fixture_mcp_spec(name: &str, tools: &str) -> McpServerSpec {
        use std::collections::HashMap;

        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/support/mcp_fixture_server.py");
        let mut env = HashMap::new();
        env.insert("XYLITOL_MCP_FIXTURE_TOOLS".into(), tools.into());
        McpServerSpec {
            name: name.into(),
            transport: McpTransportSpec::Stdio,
            command: Some("python3".into()),
            args: Some(vec![script.display().to_string()]),
            url: None,
            env: Some(env),
            headers: None,
        }
    }

    /// Real MCP stdio: /reload install adds ping, then empty reload strips it.
    #[tokio::test]
    async fn real_mcp_reload_add_then_remove_updates_provider_tools() {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        driver.freeze_tools(crate::agent::tools::ToolSet::from_iter(
            driver.builtins_for_reload(),
        ));
        assert!(
            !driver
                .tool_names_for_test()
                .iter()
                .any(|n| n.starts_with("mcp__")),
            "precondition: no mcp tools"
        );

        let mut mcp = McpSession::new();
        let add = mcp
            .reload(
                &mut driver,
                &[fixture_mcp_spec("fixture", "ping")],
                &tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("reload add");
        assert_eq!(add, McpReloadOutcome::Installed);
        assert!(driver.is_tools_frozen());
        let after_add = driver.tool_names_for_test();
        assert!(
            after_add.iter().any(|n| n == "mcp__fixture__ping"),
            "real MCP add MUST freeze ping into provider table: {after_add:?}"
        );
        assert!(mcp.has_manager());

        let remove = mcp
            .reload(
                &mut driver,
                &[],
                &tokio_util::sync::CancellationToken::new(),
            )
            .await
            .expect("reload remove");
        assert_eq!(remove, McpReloadOutcome::Installed);
        let after_remove = driver.tool_names_for_test();
        assert!(
            !after_remove.iter().any(|n| n == "mcp__fixture__ping"),
            "real MCP remove MUST drop ping from provider table: {after_remove:?}"
        );
        assert!(!mcp.has_manager());
    }

    /// Real MCP: ping→ping,echo rediscover expands provider table.
    #[tokio::test]
    async fn real_mcp_reload_can_expand_tools_on_same_server() {
        let agent = build_agent(BuildAgentOptions::default()).expect("build");
        let store: Arc<dyn XySessionStore> = Arc::new(crate::infra::session::SessionManager::new(
            tempfile::tempdir().unwrap().path().join("sessions"),
        ));
        let mut driver = crate::app::core::driver::XyInProcessDriver::new(agent, store);
        let mut mcp = McpSession::new();

        mcp.reload(
            &mut driver,
            &[fixture_mcp_spec("fixture", "ping")],
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(
            driver
                .tool_names_for_test()
                .iter()
                .any(|n| n == "mcp__fixture__ping")
        );
        assert!(
            !driver
                .tool_names_for_test()
                .iter()
                .any(|n| n == "mcp__fixture__echo")
        );

        mcp.reload(
            &mut driver,
            &[fixture_mcp_spec("fixture", "ping,echo")],
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap();
        let names = driver.tool_names_for_test();
        assert!(names.iter().any(|n| n == "mcp__fixture__ping"), "{names:?}");
        assert!(names.iter().any(|n| n == "mcp__fixture__echo"), "{names:?}");
    }
}
