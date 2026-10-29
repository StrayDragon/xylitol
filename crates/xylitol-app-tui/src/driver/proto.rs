use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;

use super::XyDriverError;
use super::types::{
    ClipboardCopyOutcome, CommandInfo, EventStream, LoadedResourcesSnapshot, ModelInfo,
    ProjectTrustMode, ProjectTrustPersistReport, RuntimeReloadReport, SessionState, XyEvent,
};

pub type BashRun =
    Pin<Box<dyn Future<Output = Result<xylitol_plugin_sdk::XyBashResult, XyDriverError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkHealth {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttachObserve {
    #[default]
    Live,
    Inflight,
    Ended,
}

#[async_trait]
pub trait XyDriver: crate::dispatch::SessionCommandExecutor + Send {
    async fn run(&mut self, prompt: &str) -> EventStream;

    async fn bash_run(
        &mut self,
        command: &str,
        exclude_from_context: bool,
    ) -> Result<BashRun, XyDriverError> {
        let _ = (command, exclude_from_context);
        Err(XyDriverError::unsupported("bash_run"))
    }

    async fn attach_session(&mut self) -> Result<(), XyDriverError> {
        Ok(())
    }

    async fn refresh_surface_caches(&mut self) -> Result<(), XyDriverError> {
        Ok(())
    }

    fn drain_idle_events(&mut self) -> Vec<XyEvent> {
        Vec::new()
    }

    fn link_health(&self) -> LinkHealth {
        LinkHealth::Up
    }

    fn attach_observe(&self) -> AttachObserve {
        AttachObserve::Live
    }

    fn take_resync_rebuild(&mut self) -> bool {
        false
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

    fn set_capability(
        &mut self,
        _id: &str,
        _enabled: bool,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        Err(XyDriverError::unsupported("set_capability"))
    }

    fn clear_capabilities(
        &mut self,
    ) -> Result<xylitol_plugin_sdk::session::CapabilitySnapshot, XyDriverError> {
        Err(XyDriverError::unsupported("clear_capabilities"))
    }

    fn get_commands(&self) -> Vec<CommandInfo>;

    fn leaf_entry_id(&self) -> Option<String>;

    fn dollar_skill_catalog(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    fn session_store(&self) -> Option<std::sync::Arc<dyn xylitol_plugin_sdk::XySessionStore>> {
        None
    }

    fn exposes_session_store(&self) -> bool {
        self.session_store().is_some()
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

    fn take_mcp_gate_notice(&mut self) -> Option<String> {
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

    async fn copy_text_to_clipboard(
        &mut self,
        _text: &str,
    ) -> Result<ClipboardCopyOutcome, XyDriverError> {
        Err(XyDriverError::unsupported(
            "copy_text_to_clipboard not supported on this driver",
        ))
    }

    async fn stage_clipboard_image(&mut self) -> Result<Option<std::path::PathBuf>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "stage_clipboard_image not supported on this driver",
        ))
    }

    async fn read_clipboard_text(&mut self) -> Result<Option<String>, XyDriverError> {
        Err(XyDriverError::unsupported(
            "read_clipboard_text not supported on this driver",
        ))
    }

    fn bind_debug_scene(&mut self, _load: &super::types::DebugSceneLoad) {}
}
