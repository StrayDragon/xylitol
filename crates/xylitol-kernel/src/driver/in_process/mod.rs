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

pub fn assemble_driver(agent: AgentRuntime, store: Arc<dyn XySessionStore>) -> RuntimeDriver {
    assemble_driver_with_ops(
        agent,
        store,
        BangExecHandler::new(Some(Arc::new(
            xylitol_official_plugin_tools::bash_exec::InfraBashExecutor::new(),
        ))),
        SessionExporter::new(Some(Arc::new(crate::export::StdExportIo::new()))),
    )
}

pub fn assemble_driver_with_ops(
    agent: AgentRuntime,
    store: Arc<dyn XySessionStore>,
    bang: BangExecHandler,
    exporter: SessionExporter,
) -> RuntimeDriver {
    let todo = xylitol_official_plugin_tools::SessionAgentTodoGateway::new(store.clone());
    let mut inner = RuntimeDriver::with_surface_ops(agent, store, bang, exporter);
    inner.set_todo_gateway(todo.clone());
    inner.set_tools(xylitol_runtime::agent::tools::ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools_with_todo(todo),
    ));
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
) {
    driver.install_ask_tool(gateway.clone());
    let todo = xylitol_official_plugin_tools::SessionAgentTodoGateway::new(driver.store.clone());
    driver.set_todo_gateway(todo.clone());
    driver.set_tools(xylitol_runtime::agent::tools::ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools_with_ask_and_todo(gateway, todo),
    ));
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
