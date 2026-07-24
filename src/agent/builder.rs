//! Agent builder — construct-time assembly for [`AgentRuntime`].
//!
//! [`AgentBuilder`] takes only the minimal runtime-protocol ports in its
//! constructor. Every other capability is attached via consuming builder
//! methods, with safe defaults (empty tool set, no bash executor, no export
//! I/O). This keeps the agent layer free of concrete `infra/` types.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::AgentRuntime;
use crate::agent::session::{AgentCapabilities, QueueMode};
use crate::agent::tools::ToolSet;
use crate::protocol::ports::{
    XyBashExecutor, XyEventSink, XyExportIo, XyHookBus, XyModelBuilder, XyPermission,
    XySessionStore,
};

/// Builder for [`AgentCapabilities`].
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
    compaction_threshold: f64,
    compaction_settings: Option<CompactionSettings>,
    cwd: String,
    bash_executor: Option<Arc<dyn XyBashExecutor>>,
    export_io: Option<Arc<dyn XyExportIo>>,
    steering_mode: QueueMode,
    follow_up_mode: QueueMode,
    hook_bus: Option<Arc<dyn XyHookBus>>,
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
            compaction_threshold: 0.8,
            compaction_settings: None,
            cwd: ".".into(),
            bash_executor: None,
            export_io: None,
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            hook_bus: None,
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

    /// Set compaction threshold.
    pub fn compaction_threshold(mut self, threshold: f64) -> Self {
        self.compaction_threshold = threshold;
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

    /// Set the bash executor port (default: none).
    pub fn bash(mut self, executor: Arc<dyn XyBashExecutor>) -> Self {
        self.bash_executor = Some(executor);
        self
    }

    /// Set the export I/O port (default: none).
    pub fn export_io(mut self, io: Arc<dyn XyExportIo>) -> Self {
        self.export_io = Some(io);
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

    /// Build the [`AgentRuntime`] (ReAct-loop runtime over [`AgentCapabilities`]).
    pub fn build(self) -> Result<AgentRuntime, String> {
        let mut session = AgentCapabilities::new(
            self.model_registry,
            self.tools,
            self.store,
            self.sink,
            self.system_prompt,
            self.context_files,
            self.append_system_prompt,
            self.compaction_threshold,
            self.cwd,
            self.compaction_settings,
            self.model_builder,
            self.permission,
            self.bash_executor,
            self.export_io,
            self.steering_mode,
            self.follow_up_mode,
            self.hook_bus,
        );
        if !self.skills.is_empty() {
            session.apply_skills(self.skills);
        }
        Ok(AgentRuntime::new(session))
    }
}
