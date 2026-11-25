use std::path::PathBuf;
use std::sync::Arc;

use xylitol_plugin_sdk::XySessionStore;
use xylitol_plugin_sdk::config_section::SectionBinding;
use xylitol_runtime::agent::AgentRuntime;
use xylitol_runtime::agent::capabilities::ToolsWriteOutcome;
use xylitol_runtime::{BangExecHandler, SessionExporter, XyInProcessDriver as RuntimeDriver};

use super::CommandInfo;

mod reload;
#[cfg(test)]
mod tests;

pub use reload::ProductReload;
pub use xylitol_runtime::XyInProcessDriver;
#[cfg(test)]
pub use xylitol_runtime::driver::{
    LoadedResourcesSnapshot, ProjectTrustMode, relay_bash_chunks, utf8_emit_len,
};

#[derive(Clone)]
pub struct DriverContext {
    pub project_root: PathBuf,
    pub trusted: bool,
    pub static_tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>,
}

impl Default for DriverContext {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            trusted: true,
            static_tools: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolFeatures {
    pub todo: bool,
    pub workflow: bool,
    pub workflow_max_depth: u32,
    pub workflow_max_total_steps: usize,
    pub tool_selection: Vec<String>,
    pub codemode_enabled: bool,
}

impl Default for ToolFeatures {
    fn default() -> Self {
        Self {
            todo: true,
            workflow: false,
            workflow_max_depth: xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            workflow_max_total_steps: xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
            tool_selection: Vec::new(),
            codemode_enabled: false,
        }
    }
}

impl ToolFeatures {
    pub fn from_config(config: &xylitol_official_plugin_config::types::AppConfig) -> Self {
        let todo = xylitol_official_plugin_todo::TodoConfig::try_from_sections(&config.sections)
            .unwrap_or_default()
            .unwrap_or_default();
        let workflow =
            xylitol_official_plugin_workflow::WorkflowConfig::try_from_sections(&config.sections)
                .unwrap_or_default()
                .unwrap_or_default();
        let tool_selection =
            xylitol_official_plugin_config::sections::ToolsSection::try_from_sections(
                &config.sections,
            )
            .unwrap_or_default()
            .unwrap_or_default();
        let codemode = xylitol_plugin_sdk::CodemodeConfig::try_from_sections(&config.sections)
            .unwrap_or_default()
            .unwrap_or_default();
        Self {
            todo: todo.is_active(),
            workflow: workflow.is_active(),
            workflow_max_depth: workflow.max_depth,
            workflow_max_total_steps: workflow.max_total_steps,
            tool_selection: tool_selection.0,
            codemode_enabled: codemode.enabled,
        }
    }
}

pub fn assemble_driver(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> RuntimeDriver {
    assemble_driver_with_features_and_context(
        agent,
        store,
        ToolFeatures::default(),
        DriverContext::default(),
    )
}

fn assemble_driver_with_features_and_context(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    features: ToolFeatures,
    context: DriverContext,
) -> RuntimeDriver {
    assemble_driver_full(
        agent,
        store,
        BangExecHandler::new(Some(Arc::new(
            xylitol_official_plugin_tools::bash_exec::InfraBashExecutor::new(),
        ))),
        SessionExporter::new(Some(Arc::new(crate::export::StdExportIo::new()))),
        features,
        context,
    )
}

pub fn assemble_session_driver(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    features: ToolFeatures,
    context: DriverContext,
    mcp_server_ids: Vec<String>,
    profile: xylitol_plugin_sdk::session::SessionProfile,
) -> RuntimeDriver {
    let mut driver = assemble_driver_with_features_and_context(agent, store, features, context);
    driver.set_mcp_server_ids(mcp_server_ids);
    driver.set_session_profile(profile);
    driver
}

pub(crate) fn attach_ctx_services(driver: &mut RuntimeDriver, ctx: &xylitol_plugin_sdk::rt::Ctx) {
    driver.attach_ctx(ctx.clone());
    if let Some(hub) = ctx.get::<dyn xylitol_plugin_sdk::SessionPrepareHub>() {
        driver.set_prepare_hub(hub);
    }
    if let Some(registry) = ctx.get::<xylitol_plugin_sdk::LoopMiddlewareRegistry>() {
        driver.set_loop_middleware(Some(registry));
    }
}

fn assemble_driver_full(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
    features: ToolFeatures,
    context: DriverContext,
) -> RuntimeDriver {
    let mut inner = RuntimeDriver::with_surface_ops(agent, store, bang, exporter);
    install_product_tool_set(&mut inner, features, &context, None);
    inner.set_slash_commands(
        crate::product_commands::product_slash_commands()
            .into_iter()
            .map(|c| CommandInfo {
                name: c.name.to_string(),
                description: c.description.to_string(),
            })
            .collect(),
    );
    inner.set_reload_hook(Arc::new(ProductReload));
    inner.set_reload_registry(reload::default_reload_registry());
    inner
}

pub(crate) fn install_product_ask_tool(
    driver: &mut RuntimeDriver,
    gateway: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>,
    features: ToolFeatures,
    context: DriverContext,
) {
    driver.install_ask_tool(gateway.clone());
    install_product_tool_set(driver, features, &context, Some(gateway));
}

fn install_product_tool_set(
    driver: &mut RuntimeDriver,
    features: ToolFeatures,
    context: &DriverContext,
    ask: Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>>,
) {
    let todo = xylitol_official_plugin_todo::SessionAgentTodoGateway::new(driver.store.clone());
    let workflow =
        xylitol_official_plugin_workflow::SessionWorkflowGateway::new(driver.store.clone());
    let automation =
        xylitol_official_plugin_tools::SessionAutomationGateway::new(driver.store.clone());
    driver.set_todo_gateway(todo.clone());
    driver.set_automation_gateway(automation.clone());
    driver.set_workflow_gateway(workflow.clone());
    let tools = compose_tools(features, context, todo, workflow, automation, ask);
    driver.set_base_tools(tools.clone());
    if let Some(outcome) = driver.try_set_tools_inline(tools) {
        debug_assert!(matches!(outcome, ToolsWriteOutcome::Applied));
    }
}

fn static_tool_base(context: &DriverContext) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
    if context.static_tools.is_empty() {
        xylitol_official_plugin_tools::core_tools()
    } else {
        context.static_tools.clone()
    }
}

fn compose_tools(
    features: ToolFeatures,
    context: &DriverContext,
    todo: Arc<xylitol_official_plugin_todo::SessionAgentTodoGateway>,
    workflow: Arc<xylitol_official_plugin_workflow::SessionWorkflowGateway>,
    automation: Arc<xylitol_official_plugin_tools::SessionAutomationGateway>,
    ask: Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>>,
) -> xylitol_runtime::agent::tools::ToolSet {
    let mut tools = static_tool_base(context);
    if features.todo {
        tools.extend(xylitol_official_plugin_todo::todo_tools(todo.clone()));
    }
    tools.extend(xylitol_official_plugin_tools::automation_tools(
        automation.clone(),
    ));
    tools.extend(xylitol_official_plugin_tools::goal_tools(automation));
    if features.workflow {
        tools.extend(xylitol_official_plugin_workflow::workflow_tools(
            workflow,
            features.workflow_max_depth,
            features.workflow_max_total_steps,
        ));
    }
    if let Some(ask) = ask {
        tools.push(xylitol_official_plugin_tools::ask_tool(ask));
    }
    if !features.codemode_enabled && xylitol_plugin_sdk::run_code_selected(&features.tool_selection)
    {
        log::warn!(
            target: "xylitol::tools",
            "codemode disabled; run_code selection ignored"
        );
    }
    resolve_tool_set(tools, &features.tool_selection)
}

fn resolve_tool_set(
    tools: Vec<Arc<dyn xylitol_plugin_sdk::XyTool>>,
    selection: &[String],
) -> xylitol_runtime::agent::tools::ToolSet {
    let (set, warnings) = xylitol_runtime::agent::tools::resolve_tool_selection(tools, selection);
    for warning in warnings {
        log::warn!(target: "xylitol::tools", "{warning}");
    }
    set
}
