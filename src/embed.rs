pub use xylitol_kernel::boot::{BootOptions, Kernel, Profile, boot};
pub use xylitol_kernel::bootstrap::{
    BootstrapError, BootstrapInput, BootstrapWarning, BootstrappedAgent, BootstrappedRuntime,
    bootstrap,
};
pub use xylitol_kernel::composition::{
    BuildAgentOptions, McpServerSpec, McpSession, McpTransportSpec, build_agent,
};
pub use xylitol_kernel::driver::{
    CommandInfo, EventStream, ModelInfo, SessionState, SessionStats, XyDriver, XyDriverError,
    XyInProcessDriver, assemble_driver,
};

pub fn with_loop_registry(
    mut driver: XyInProcessDriver,
    registry: std::sync::Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>,
) -> XyInProcessDriver {
    driver.set_loop_middleware(Some(registry));
    driver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_loop_registry_attaches_registry_without_rutis_types() {
        use xylitol_plugin_sdk::{LoopStage, LoopStep};

        struct Marker;

        #[async_trait::async_trait]
        impl xylitol_plugin_sdk::LoopMiddleware for Marker {
            fn id(&self) -> &str {
                "embed-marker"
            }

            fn stages(&self) -> &'static [LoopStage] {
                &[LoopStage::BeforeTurn]
            }

            async fn before_turn(
                &self,
                ctx: xylitol_plugin_sdk::BeforeTurnCtx,
            ) -> LoopStep<xylitol_plugin_sdk::BeforeTurnCtx> {
                LoopStep::continued(ctx)
            }
        }

        let registry = std::sync::Arc::new(xylitol_plugin_sdk::LoopMiddlewareRegistry::new());
        registry
            .register(std::sync::Arc::new(Marker))
            .expect("register marker");

        let driver = with_loop_registry_for_test(registry.clone());
        assert_eq!(
            driver.agent.loop_middleware_ids(),
            vec!["embed-marker".to_string()]
        );
    }

    fn with_loop_registry_for_test(
        registry: std::sync::Arc<xylitol_plugin_sdk::LoopMiddlewareRegistry>,
    ) -> XyInProcessDriver {
        let runtime = xylitol_kernel::composition::build_agent(
            xylitol_kernel::composition::BuildAgentOptions::default(),
        )
        .expect("build agent");
        let store: std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore> =
            std::sync::Arc::new(xylitol_official_plugin_session::SessionManager::in_memory());
        with_loop_registry(assemble_driver(runtime, store), registry)
    }
}
