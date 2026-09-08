//! Clonable construction baseline for session-bound [`AgentRuntime`] actors.
//!
//! [`RuntimePorts`] holds shared Arc ports and cloneable config snapshots so a
//! future factory can materialize multiple isolated runtimes from one baseline.
//! Each [`RuntimePorts::materialize_runtime`] call gets fresh ModelManager
//! selection, queues, session binding, coordinator / active turn, and
//! compaction orchestrator — never shared history / pending / cancel / active
//! turn.

use std::sync::Arc;

use crate::agent::capabilities::{AgentCapabilities, QueueMode};
use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::AgentRuntime;
use crate::agent::tools::ToolSet;
use crate::protocol::ports::{
    XyBatchMode, XyEventSink, XyHookBus, XyModelBuilder, XyPermission, XySessionStore,
};
use crate::protocol::resource::SkillInfo;

/// Clonable construction inputs for materializing isolated [`AgentRuntime`]s.
///
/// Arc ports are cheaply shared; owned baselines (`ModelRegistry`, `ToolSet`,
/// prompt / skills / compaction / queue modes) are cloned per materialization.
#[derive(Clone)]
pub struct RuntimePorts {
    pub store: Arc<dyn XySessionStore>,
    pub sink: Arc<dyn XyEventSink>,
    pub permission: Arc<dyn XyPermission>,
    pub hook_bus: Option<Arc<dyn XyHookBus>>,
    /// Already [`Clone`] (owned maps + `Arc` secret resolver).
    pub model_registry: ModelRegistry,
    /// `XyModelBuilder` is `Arc`-backed.
    pub model_builder: XyModelBuilder,
    pub cwd: String,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    pub compaction_settings: Option<CompactionSettings>,
    /// Already [`Clone`] (`Vec<Arc<dyn XyTool>>`).
    pub tools: ToolSet,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    pub skills: Vec<SkillInfo>,
    pub batch_mode: XyBatchMode,
}

impl RuntimePorts {
    /// Materialize a fresh [`crate::agent::AgentCapabilities`] from this baseline.
    ///
    /// Always creates a new ModelManager (selection unset), new queue Arc,
    /// `session_id = None`, midturn probe false initially, and a new
    /// compaction orchestrator instance.
    pub(crate) fn materialize_capabilities(&self) -> AgentCapabilities {
        let mut caps = AgentCapabilities::new(
            self.model_registry.clone(),
            self.tools.clone(),
            Arc::clone(&self.store),
            Arc::clone(&self.sink),
            self.system_prompt.clone(),
            self.context_files.clone(),
            self.append_system_prompt.clone(),
            self.cwd.clone(),
            self.compaction_settings.clone(),
            Arc::clone(&self.model_builder),
            Arc::clone(&self.permission),
            self.steering_mode,
            self.follow_up_mode,
            self.hook_bus.clone(),
        );
        caps.set_tool_mode(self.batch_mode);
        if !self.skills.is_empty() {
            caps.apply_skills(self.skills.clone());
        }
        caps
    }

    /// Materialize a fresh session-bound [`AgentRuntime`] actor.
    pub fn materialize_runtime(&self) -> AgentRuntime {
        AgentRuntime::new(self.materialize_capabilities())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::runtime::RunPolicy;
    use crate::infra::session::SessionManager;
    use crate::protocol::error::XyError;
    use crate::protocol::message::LlmMessage;
    use crate::protocol::model::{XyChunk, XyModelConfig, XyModelKind, XyModelMeta, XyToolSchema};
    use crate::protocol::ports::{XyGenerateOptions, XyModel, XyStream};
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn mock_registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new();
        reg.register(XyModelMeta {
            id: "mock".into(),
            config: XyModelConfig {
                kind: XyModelKind::Fake,
                api_key: String::new(),
                model: "mock".into(),
                base_url: None,
                api: None,
                compat: None,
            },
            display_name: "Mock".into(),
            thinking: false,
            context_window: 128000,
            api: String::new(),
            provider: String::new(),
            cost_input: 0.0,
            cost_output: 0.0,
            cost_cache_read: 0.0,
            cost_cache_write: 0.0,
            max_tokens: 0,
            thinking_levels: Vec::new(),
            thinking_level_map: Default::default(),
        });
        reg
    }

    fn baseline_ports(model_builder: XyModelBuilder) -> RuntimePorts {
        let session_mgr = SessionManager::new(tempfile::tempdir().unwrap().path().join("sessions"));
        let store: Arc<dyn XySessionStore> = Arc::new(session_mgr);
        let sink: Arc<dyn XyEventSink> = Arc::new(crate::infra::event::EventBus::new());
        RuntimePorts {
            store,
            sink,
            permission: crate::infra::permission::allow_all_permission(),
            hook_bus: None,
            model_registry: mock_registry(),
            model_builder,
            cwd: ".".into(),
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            compaction_settings: None,
            tools: ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            batch_mode: XyBatchMode::Sequential,
        }
    }

    struct SlowMock {
        polled: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl XyModel for SlowMock {
        fn name(&self) -> &str {
            "slow-mock"
        }
        async fn generate_stream(
            &self,
            _messages: Vec<LlmMessage>,
            _tools: &[XyToolSchema],
            _stream: bool,
            _options: XyGenerateOptions,
        ) -> Result<XyStream, XyError> {
            let polled = self.polled.clone();
            Ok(Box::pin(async_stream::stream! {
                for i in 0..80u32 {
                    tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                    polled.fetch_add(1, Ordering::SeqCst);
                    yield Ok(XyChunk::TextDelta(format!("c{i}")));
                }
                yield Ok(XyChunk::Done {
                    finish_reason: crate::protocol::message::XyStopReason::Stop,
                    usage: None,
                });
            }))
        }
    }

    #[tokio::test]
    async fn cloned_ports_yield_independent_sessions_and_model_selection() {
        let ports = baseline_ports(Arc::new(crate::infra::provider::factory::build_provider));
        let mut a = ports.clone().materialize_runtime();
        let mut b = ports.materialize_runtime();

        // Select before bind: persistence only runs once a session id is set.
        a.select_model("mock").await.expect("select on a");
        assert!(a.current_model().is_some());
        assert!(
            b.current_model().is_none(),
            "model selection must not leak across materialized runtimes"
        );

        a.bind_session("sess-a").expect("bind a");
        b.bind_session("sess-b").expect("bind b");
        assert_eq!(a.session_id(), Some("sess-a"));
        assert_eq!(b.session_id(), Some("sess-b"));

        a.steer("a-steer");
        b.steer("b-steer");
        b.follow_up("b-fu");
        assert_eq!(a.queue_stats().steer_count, 1);
        assert_eq!(b.queue_stats().steer_count, 1);
        assert_eq!(b.queue_stats().follow_up_count, 1);

        a.abort();
        assert_eq!(a.queue_stats().steer_count, 0);
        assert_eq!(
            b.queue_stats().steer_count,
            1,
            "abort on one runtime must not clear the other's steer queue"
        );
        assert_eq!(b.queue_stats().follow_up_count, 1);
        assert!(!a.has_active_turn());
        assert!(!b.has_active_turn());
    }

    #[tokio::test]
    async fn abort_one_runtime_does_not_cancel_sibling() {
        use crate::protocol::lifecycle::XyEvent;

        let polled = Arc::new(AtomicUsize::new(0));
        let polled_for_builder = polled.clone();
        let builder: XyModelBuilder = Arc::new(move |_| {
            Arc::new(SlowMock {
                polled: polled_for_builder.clone(),
            }) as Arc<dyn XyModel>
        });
        let ports = baseline_ports(builder);
        let mut a = ports.clone().materialize_runtime();
        let mut b = ports.materialize_runtime();
        a.select_model("mock").await.expect("select a");
        b.select_model("mock").await.expect("select b");
        a.bind_session("sess-a").expect("bind a");
        b.bind_session("sess-b").expect("bind b");

        let mut stream_a = a.submit_root("go-a", RunPolicy::Reject).await;
        let mut stream_b = b.submit_root("go-b", RunPolicy::Reject).await;

        // Wait until both actors report an active turn.
        let mut saw_a = false;
        let mut saw_b = false;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            saw_a = a.has_active_turn();
            saw_b = b.has_active_turn();
            if saw_a && saw_b {
                break;
            }
        }
        assert!(saw_a && saw_b, "both runtimes should have an active turn");

        let token_b_before = b.cancel_token();
        assert!(!token_b_before.is_cancelled());

        a.abort();

        let mut a_aborted = false;
        while let Some(evt) = stream_a.next().await {
            if matches!(&evt, XyEvent::Error(err) if err.is_aborted()) {
                a_aborted = true;
            }
        }
        assert!(a_aborted, "aborted runtime must surface aborted");
        assert!(!a.has_active_turn());
        assert!(
            b.has_active_turn(),
            "sibling runtime must keep its own active turn"
        );
        assert!(
            !token_b_before.is_cancelled(),
            "abort must not cancel sibling cancel token"
        );
        assert!(!b.cancel_token().is_cancelled());

        // Let sibling finish cleanly.
        while stream_b.next().await.is_some() {}
        assert!(!b.has_active_turn());
    }
}
