use std::path::PathBuf;
use std::sync::Arc;

use xylitol_plugin_sdk::XySessionStore;
use xylitol_runtime::agent::AgentRuntime;
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
    pub plan: bool,
    pub workflow: bool,
    pub workflow_max_depth: u32,
    pub workflow_max_total_steps: usize,
    pub tool_selection: Vec<String>,
}

impl Default for ToolFeatures {
    fn default() -> Self {
        Self {
            todo: true,
            plan: true,
            workflow: false,
            workflow_max_depth: xylitol_plugin_sdk::session::DEFAULT_MAX_DEPTH,
            workflow_max_total_steps: xylitol_plugin_sdk::session::DEFAULT_MAX_TOTAL_STEPS,
            tool_selection: Vec::new(),
        }
    }
}

impl ToolFeatures {
    pub fn from_config(config: &xylitol_official_plugin_config::types::AppConfig) -> Self {
        Self {
            todo: config.todo.is_active(),
            plan: config.plan.is_active(),
            workflow: config.workflow.is_active(),
            workflow_max_depth: config.workflow.max_depth,
            workflow_max_total_steps: config.workflow.max_total_steps,
            tool_selection: config.tools.clone(),
        }
    }
}

pub fn assemble_driver(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> RuntimeDriver {
    assemble_driver_with_features(agent, store, ToolFeatures::default())
}

pub fn assemble_driver_with_features(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    features: ToolFeatures,
) -> RuntimeDriver {
    assemble_driver_with_features_and_context(agent, store, features, DriverContext::default())
}

pub fn assemble_driver_with_features_and_context(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    features: ToolFeatures,
    context: DriverContext,
) -> RuntimeDriver {
    assemble_driver_with_ops_and_features_and_context(
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

pub fn assemble_driver_with_ops(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
) -> RuntimeDriver {
    assemble_driver_with_ops_and_features(agent, store, bang, exporter, ToolFeatures::default())
}

pub fn assemble_driver_with_ops_and_features(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
    features: ToolFeatures,
) -> RuntimeDriver {
    assemble_driver_with_ops_and_features_and_context(
        agent,
        store,
        bang,
        exporter,
        features,
        DriverContext::default(),
    )
}

pub fn assemble_driver_with_ops_and_features_and_context(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
    features: ToolFeatures,
    context: DriverContext,
) -> RuntimeDriver {
    let todo = xylitol_official_plugin_todo::SessionAgentTodoGateway::new(store.clone());
    let plan = xylitol_official_plugin_plan::SessionPlanGateway::new(
        store.clone(),
        context.project_root.clone(),
        context.trusted,
    );
    let workflow = xylitol_official_plugin_workflow::SessionWorkflowGateway::new(store.clone());
    let mut inner = RuntimeDriver::with_surface_ops(agent, store, bang, exporter);
    inner.set_todo_gateway(todo.clone());
    inner.set_plan_gateway(plan.clone());
    inner.set_workflow_gateway(workflow.clone());
    let tools = compose_tools(features, &context, todo, plan, workflow, None);
    inner.set_base_tools(tools.clone());
    inner.set_tools(tools);
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
    inner
}

pub fn install_product_ask_tool(
    driver: &mut RuntimeDriver,
    gateway: Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>,
    features: ToolFeatures,
    context: DriverContext,
) {
    driver.install_ask_tool(gateway.clone());
    let todo = xylitol_official_plugin_todo::SessionAgentTodoGateway::new(driver.store.clone());
    let plan = xylitol_official_plugin_plan::SessionPlanGateway::new(
        driver.store.clone(),
        context.project_root.clone(),
        context.trusted,
    );
    let workflow =
        xylitol_official_plugin_workflow::SessionWorkflowGateway::new(driver.store.clone());
    driver.set_todo_gateway(todo.clone());
    driver.set_plan_gateway(plan.clone());
    driver.set_workflow_gateway(workflow.clone());
    let tools = compose_tools(features, &context, todo, plan, workflow, Some(gateway));
    driver.set_base_tools(tools.clone());
    driver.set_tools(tools);
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
    plan: Arc<xylitol_official_plugin_plan::SessionPlanGateway>,
    workflow: Arc<xylitol_official_plugin_workflow::SessionWorkflowGateway>,
    ask: Option<Arc<dyn xylitol_plugin_sdk::ask::AskUserGateway>>,
) -> xylitol_runtime::agent::tools::ToolSet {
    let mut tools = static_tool_base(context);
    if features.todo {
        tools.extend(xylitol_official_plugin_todo::todo_tools(todo.clone()));
    }
    if features.plan {
        let todo_handoff = features
            .todo
            .then_some(todo as Arc<dyn xylitol_plugin_sdk::AgentTodoGateway>);
        tools.extend(xylitol_official_plugin_plan::plan_tools(
            plan,
            todo_handoff,
            ask.clone(),
        ));
    }
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

pub fn assemble_driver_with_bang(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
) -> RuntimeDriver {
    assemble_driver_with_ops(
        agent,
        store,
        bang,
        SessionExporter::new(Some(Arc::new(crate::export::StdExportIo::new()))),
    )
}
