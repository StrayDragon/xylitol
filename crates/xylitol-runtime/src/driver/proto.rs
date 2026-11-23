use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use serde_json::Value;

use super::XyDriverError;
use super::types::{
    CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo, ProjectTrustMode,
    ProjectTrustPersistReport, RuntimeReloadReport, SessionState, TurnRequest,
};

pub type BashRun =
    Pin<Box<dyn Future<Output = Result<xylitol_plugin_sdk::XyBashResult, XyDriverError>> + Send>>;

#[async_trait]
pub trait XyDriver: Send {
    async fn steer(&mut self, message: &str) -> Result<String, XyDriverError> {
        let _ = message;
        Err(XyDriverError::unsupported("steer"))
    }

    async fn follow_up(&mut self, message: &str) -> Result<String, XyDriverError> {
        let _ = message;
        Err(XyDriverError::unsupported("follow_up"))
    }

    async fn take_follow_up(&mut self) -> Option<String> {
        None
    }

    fn queue_items(&self) -> Vec<xylitol_plugin_sdk::session::QueueItem> {
        Vec::new()
    }

    fn revoke_queue_item(&self, id: &str) -> xylitol_plugin_sdk::RevokeQueueItemOutcome {
        let _ = id;
        xylitol_plugin_sdk::RevokeQueueItemOutcome {
            revoked: false,
            items: Vec::new(),
        }
    }

    async fn run(&mut self, prompt: &str) -> EventStream;

    async fn run_with_context(
        &mut self,
        prompt: &str,
        contexts: Vec<xylitol_plugin_sdk::mcp::McpResourceContext>,
    ) -> EventStream {
        self.run_with_context_impl(prompt, contexts).await
    }

    async fn run_with_context_impl(
        &mut self,
        prompt: &str,
        _contexts: Vec<xylitol_plugin_sdk::mcp::McpResourceContext>,
    ) -> EventStream {
        self.run(prompt).await
    }

    async fn run_turn(&mut self, request: TurnRequest) -> Result<EventStream, XyDriverError> {
        if request.context.is_empty() && request.system_prompt.is_none() {
            return Ok(self.run(&request.prompt_text()).await);
        }
        Err(XyDriverError::unsupported("run_turn request context"))
    }

    async fn mcp_catalog(&mut self) -> Result<xylitol_plugin_sdk::mcp::McpCatalog, XyDriverError> {
        Ok(xylitol_plugin_sdk::mcp::McpCatalog::default())
    }

    async fn mcp_resource_read(
        &mut self,
        server: &str,
        uri: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpResourceContent, XyDriverError> {
        let _ = (server, uri);
        Err(XyDriverError::unsupported("mcp_resource_read"))
    }

    async fn mcp_prompt_get(
        &mut self,
        server: &str,
        name: &str,
        arguments: Value,
    ) -> Result<Value, XyDriverError> {
        let _ = (server, name, arguments);
        Err(XyDriverError::unsupported("mcp_prompt_get"))
    }

    async fn mcp_oauth_start(
        &mut self,
        server: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthFlow, XyDriverError> {
        let _ = server;
        Err(XyDriverError::unsupported("mcp_oauth_start"))
    }

    async fn mcp_oauth_cancel(&mut self, flow_id: &str) -> Result<(), XyDriverError> {
        let _ = flow_id;
        Err(XyDriverError::unsupported("mcp_oauth_cancel"))
    }

    async fn mcp_oauth_status(
        &mut self,
        flow_id: &str,
    ) -> Result<xylitol_plugin_sdk::mcp::McpOAuthStatus, XyDriverError> {
        let _ = flow_id;
        Err(XyDriverError::unsupported("mcp_oauth_status"))
    }

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<BashRun, XyDriverError> {
        let _ = (command, exclude_from_context);
        Err(XyDriverError::unsupported("bash_run"))
    }

    fn abort(&self);

    fn current_model(&self) -> Option<ModelInfo>;

    fn active_turn(&self) -> Option<(String, String, bool)> {
        None
    }

    fn has_active_turn(&self) -> bool {
        false
    }

    fn available_models(&self) -> Vec<ModelInfo>;

    fn thinking_level(&self) -> String;

    fn session_id(&self) -> Option<String>;

    fn get_state(&self) -> SessionState {
        SessionState {
            session_id: self.session_id().unwrap_or_default(),
            model: self.current_model(),
            thinking_level: self.thinking_level(),
            leaf_entry_id: self.leaf_entry_id(),
        }
    }

    fn set_bash_run_sink(&mut self, _sink: Option<xylitol_plugin_sdk::BashOutputSink>) {}

    async fn estimate_context_tokens(
        &self,
    ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError>;

    fn capability_snapshot(&self) -> xylitol_plugin_sdk::session::CapabilitySnapshot {
        xylitol_plugin_sdk::session::CapabilitySnapshot::default()
    }

    fn capability_pending(&self) -> bool {
        false
    }

    async fn set_capability(
        &mut self,
        _id: &str,
        _enabled: bool,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        Err(XyDriverError::unsupported("set_capability"))
    }

    async fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        Err(XyDriverError::unsupported("clear_capabilities"))
    }

    fn get_commands(&self) -> Vec<CommandInfo>;

    fn leaf_entry_id(&self) -> Option<String>;

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot;

    fn loaded_resources_cached(&self) -> Option<LoadedResourcesSnapshot> {
        None
    }

    fn mcp_blocks_agent(&self) -> bool {
        false
    }

    fn is_tools_frozen(&self) -> bool {
        true
    }

    async fn arm_tool_freeze_gate(&mut self) {}

    async fn take_mcp_gate_notice(&mut self) -> Option<String> {
        None
    }

    async fn begin_session_prepare(&mut self) {}

    async fn poll_prepare_progress(&mut self) -> bool {
        false
    }

    async fn wait_session_prepare(&mut self) {
        while self.mcp_blocks_agent() {
            let _ = self.poll_prepare_progress().await;
            tokio::task::yield_now().await;
        }
    }

    async fn reload_runtime(
        &mut self,
        _cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<RuntimeReloadReport, XyDriverError> {
        Ok(RuntimeReloadReport::noop())
    }

    async fn persist_project_trust(
        &mut self,
        _mode: ProjectTrustMode,
    ) -> Result<ProjectTrustPersistReport, XyDriverError> {
        Err(XyDriverError::unsupported(
            "persist_project_trust not supported on this driver",
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::types::XyEvent;
    use futures::StreamExt;

    struct Stub;

    #[async_trait]
    impl XyDriver for Stub {
        async fn run(&mut self, prompt: &str) -> EventStream {
            Box::pin(futures::stream::iter(vec![XyEvent::TextDelta(
                prompt.to_string(),
            )]))
        }

        fn abort(&self) {}

        fn current_model(&self) -> Option<ModelInfo> {
            None
        }

        fn available_models(&self) -> Vec<ModelInfo> {
            Vec::new()
        }

        fn thinking_level(&self) -> String {
            "off".into()
        }

        fn session_id(&self) -> Option<String> {
            Some("stub".into())
        }

        fn get_commands(&self) -> Vec<CommandInfo> {
            Vec::new()
        }

        fn leaf_entry_id(&self) -> Option<String> {
            None
        }

        async fn loaded_resources_snapshot(&self) -> LoadedResourcesSnapshot {
            LoadedResourcesSnapshot::default()
        }

        async fn estimate_context_tokens(
            &self,
        ) -> Result<xylitol_plugin_sdk::model::ContextTokenEstimate, XyDriverError> {
            Ok(Default::default())
        }
    }

    #[tokio::test]
    async fn run_turn_falls_back_for_plain_prompt() {
        let mut driver = Stub;
        let mut stream = driver
            .run_turn(TurnRequest::text("hello"))
            .await
            .expect("plain prompt must fall back");
        let mut text = String::new();
        while let Some(event) = stream.next().await {
            if let XyEvent::TextDelta(delta) = event {
                text.push_str(&delta);
            }
        }
        assert_eq!(text, "hello");
    }

    #[tokio::test]
    async fn run_turn_rejects_context_on_unsupported_driver() {
        let mut driver = Stub;
        let request = TurnRequest::text("hello").with_context(vec![
            xylitol_plugin_sdk::message::XyLlmMessage::user("prior"),
        ]);
        let error = match driver.run_turn(request).await {
            Ok(_) => panic!("context must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("run_turn request context"));
    }

    #[tokio::test]
    async fn run_turn_rejects_system_on_unsupported_driver() {
        let mut driver = Stub;
        let request = TurnRequest::text("hello").with_system_prompt(Some("system".into()));
        let error = match driver.run_turn(request).await {
            Ok(_) => panic!("system must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("run_turn request context"));
    }
}
