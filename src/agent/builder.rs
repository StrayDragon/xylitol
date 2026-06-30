//! Agent builder — construct-time assembly for [`Agent`].
//!
//! [`AgentBuilder`] takes only the minimal runtime-protocol ports in its
//! constructor. Every other capability is attached via consuming builder
//! methods, with safe defaults (empty tool set, no bash executor, no export
//! I/O). This keeps the agent layer free of concrete `infra/` types.

use std::sync::Arc;

use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::ReActAgent;
use crate::agent::session::Agent;
use crate::agent::tools::ToolSet;
use crate::runtime_protocol::{
    XyBashExecutor, XyEventSink, XyExportIo, XyModelBuilder, XyPermission, XySessionStore,
};

/// Builder for [`Agent`].
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
    max_iterations: u32,
    compaction_threshold: f64,
    compaction_settings: Option<CompactionSettings>,
    cwd: String,
    bash_executor: Option<Arc<dyn XyBashExecutor>>,
    export_io: Option<Arc<dyn XyExportIo>>,
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
            max_iterations: 50,
            compaction_threshold: 0.8,
            compaction_settings: None,
            cwd: ".".into(),
            bash_executor: None,
            export_io: None,
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

    /// Set the maximum ReAct iterations per turn (default: 50).
    pub fn max_iterations(mut self, n: u32) -> Self {
        self.max_iterations = n;
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

    /// Build the [`ReActAgent`] (the ReAct-strategy driver over an [`Agent`]).
    pub fn build(self) -> Result<ReActAgent, String> {
        let session = Agent::new(
            self.model_registry,
            self.tools,
            self.store,
            self.sink,
            self.system_prompt,
            self.context_files,
            self.append_system_prompt,
            self.max_iterations,
            self.compaction_threshold,
            self.cwd,
            self.compaction_settings,
            self.model_builder,
            self.permission,
            self.bash_executor,
            self.export_io,
        );
        Ok(ReActAgent::new(session))
    }
}
