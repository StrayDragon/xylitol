//! Agent builder — construct-time assembly for [`AgentRuntime`].
//!
//! [`AgentBuilder`] takes only the minimal runtime-protocol ports in its
//! constructor. Every other capability is attached via consuming builder
//! methods, with safe defaults (empty tool set, no optional surface I/O). This keeps the agent layer free of concrete `infra/` types.
//!
//! [`AgentBuilder::build_ports`] yields a clonable [`RuntimePorts`] baseline;
//! [`AgentBuilder::build`] materializes one [`AgentRuntime`] from it.

use std::sync::Arc;

use crate::agent::capabilities::QueueMode;
use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::{AgentRuntime, RuntimePorts};
use crate::agent::tools::ToolSet;
use crate::protocol::ports::{
    XyBatchMode, XyEventSink, XyHookBus, XyModelBuilder, XyPermission, XySessionStore,
};

/// Builder for [`crate::agent::AgentCapabilities`] / [`crate::agent::AgentRuntime`].
pub struct AgentBuilder {
    // Required ports for a minimal conversation agent.
    model_registry: ModelRegistry,
    model_builder: XyModelBuilder,
    store: Arc<dyn XySessionStore>,
    sink: Arc<dyn XyEventSink>,
    permission: Arc<dyn XyPermission>,

    // Optional capability slots.
    tools: ToolSet,
    system_prompt: Option<String>,
    context_files: Vec<(String, String)>,
    append_system_prompt: Vec<String>,
    skills: Vec<crate::protocol::resource::SkillInfo>,
    compaction_settings: Option<CompactionSettings>,
    cwd: String,
    steering_mode: QueueMode,
    follow_up_mode: QueueMode,
    hook_bus: Option<Arc<dyn XyHookBus>>,
    batch_mode: XyBatchMode,
}

impl AgentBuilder {
    /// Create a builder with the minimal required ports.
    pub fn new(
        model_registry: ModelRegistry,
        model_builder: XyModelBuilder,
        store: Arc<dyn XySessionStore>,
        sink: Arc<dyn XyEventSink>,
        permission: Arc<dyn XyPermission>,
    ) -> Self {
        Self {
            model_registry,
            model_builder,
            store,
            sink,
            permission,
            tools: ToolSet::empty(),
            system_prompt: None,
            context_files: Vec::new(),
            append_system_prompt: Vec::new(),
            skills: Vec::new(),
            compaction_settings: None,
            cwd: ".".into(),
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            hook_bus: None,
            batch_mode: XyBatchMode::Sequential,
        }
    }

    /// Set the tool set (default: empty).
    pub fn tools(mut self, tools: ToolSet) -> Self {
        self.tools = tools;
        self
    }

    /// Set the system prompt.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set project context files (path => content).
    pub fn context_files(mut self, files: Vec<(String, String)>) -> Self {
        self.context_files = files;
        self
    }

    /// Set append-system-prompt lines.
    pub fn append_system_prompt(mut self, lines: Vec<String>) -> Self {
        self.append_system_prompt = lines;
        self
    }

    /// Set skills catalog for `<available_skills>` in the system prompt (c1085).
    pub fn skills(mut self, skills: Vec<crate::protocol::resource::SkillInfo>) -> Self {
        self.skills = skills;
        self
    }

    /// Set compaction settings.
    pub fn compaction_settings(mut self, settings: Option<CompactionSettings>) -> Self {
        self.compaction_settings = settings;
        self
    }

    /// Set the working directory used in the session header.
    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Override the permission port (default: the one passed to [`new`](Self::new)).
    pub fn permission(mut self, permission: Arc<dyn XyPermission>) -> Self {
        self.permission = permission;
        self
    }

    /// Set steering queue drain mode (default: [`QueueMode::OneAtATime`]).
    pub fn steering_mode(mut self, mode: QueueMode) -> Self {
        self.steering_mode = mode;
        self
    }

    /// Set follow-up queue drain mode (default: [`QueueMode::OneAtATime`]).
    pub fn follow_up_mode(mut self, mode: QueueMode) -> Self {
        self.follow_up_mode = mode;
        self
    }

    /// Attach the script hook bus (default: none).
    pub fn hook_bus(mut self, bus: Option<Arc<dyn XyHookBus>>) -> Self {
        self.hook_bus = bus;
        self
    }

    /// Set tool batch scheduling mode (default: [`XyBatchMode::BarrierParallel`]).
    pub fn batch_mode(mut self, mode: XyBatchMode) -> Self {
        self.batch_mode = mode;
        self
    }

    /// Build a clonable [`RuntimePorts`] baseline (skills / batch_mode baked in).
    ///
    /// Call [`RuntimePorts::materialize_runtime`] (or clone then materialize) to
    /// obtain isolated session actors that do not share ModelManager selection,
    /// queues, session id, coordinator, or compaction orchestrator state.
    pub fn build_ports(self) -> RuntimePorts {
        RuntimePorts {
            store: self.store,
            sink: self.sink,
            permission: self.permission,
            hook_bus: self.hook_bus,
            model_registry: self.model_registry,
            model_builder: self.model_builder,
            cwd: self.cwd,
            steering_mode: self.steering_mode,
            follow_up_mode: self.follow_up_mode,
            compaction_settings: self.compaction_settings,
            tools: self.tools,
            system_prompt: self.system_prompt,
            context_files: self.context_files,
            append_system_prompt: self.append_system_prompt,
            skills: self.skills,
            batch_mode: self.batch_mode,
        }
    }

    /// Build the [`AgentRuntime`] (ReAct-loop runtime over capabilities).
    ///
    /// Internally builds [`RuntimePorts`] then materializes one actor.
    pub fn build(self) -> AgentRuntime {
        self.build_ports().materialize_runtime()
    }
}
