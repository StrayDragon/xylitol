use std::sync::Arc;

use crate::agent::capabilities::QueueMode;
use crate::agent::compaction::{CompactionSettings, XyCompactionFactory, noop_compaction_factory};
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::RuntimeBaseline;
use crate::agent::tools::ToolSet;
use xylitol_plugin_sdk::{
    XyBatchMode, XyEventSink, XyHookBus, XyModelBuilder, XyPermission, XySessionStore,
};

pub struct AgentBuilder {
    model_registry: ModelRegistry,
    model_builder: XyModelBuilder,
    store: Arc<dyn XySessionStore>,
    sink: Arc<dyn XyEventSink>,
    permission: Arc<dyn XyPermission>,

    tools: ToolSet,
    system_prompt: Option<String>,
    context_files: Vec<(String, String)>,
    append_system_prompt: Vec<String>,
    skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>,
    compaction_settings: Option<CompactionSettings>,
    compaction_factory: XyCompactionFactory,
    cwd: String,
    steering_mode: QueueMode,
    follow_up_mode: QueueMode,
    hook_bus: Option<Arc<dyn XyHookBus>>,
    batch_mode: XyBatchMode,
}

impl AgentBuilder {
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
            compaction_factory: noop_compaction_factory(),
            cwd: ".".into(),
            steering_mode: QueueMode::default(),
            follow_up_mode: QueueMode::default(),
            hook_bus: None,
            batch_mode: XyBatchMode::Sequential,
        }
    }

    pub fn tools(mut self, tools: ToolSet) -> Self {
        self.tools = tools;
        self
    }

    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn context_files(mut self, files: Vec<(String, String)>) -> Self {
        self.context_files = files;
        self
    }

    pub fn append_system_prompt(mut self, lines: Vec<String>) -> Self {
        self.append_system_prompt = lines;
        self
    }

    pub fn skills(mut self, skills: Vec<xylitol_plugin_sdk::resource::SkillInfo>) -> Self {
        self.skills = skills;
        self
    }

    pub fn compaction_settings(mut self, settings: Option<CompactionSettings>) -> Self {
        self.compaction_settings = settings;
        self
    }

    pub fn compaction_factory(mut self, factory: XyCompactionFactory) -> Self {
        self.compaction_factory = factory;
        self
    }

    pub fn cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = cwd.into();
        self
    }

    pub fn steering_mode(mut self, mode: QueueMode) -> Self {
        self.steering_mode = mode;
        self
    }

    pub fn follow_up_mode(mut self, mode: QueueMode) -> Self {
        self.follow_up_mode = mode;
        self
    }

    pub fn hook_bus(mut self, bus: Option<Arc<dyn XyHookBus>>) -> Self {
        self.hook_bus = bus;
        self
    }

    pub fn batch_mode(mut self, mode: XyBatchMode) -> Self {
        self.batch_mode = mode;
        self
    }

    pub fn build_baseline(self) -> RuntimeBaseline {
        RuntimeBaseline {
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
            compaction_factory: self.compaction_factory,
            tools: self.tools,
            system_prompt: self.system_prompt,
            context_files: self.context_files,
            append_system_prompt: self.append_system_prompt,
            skills: self.skills,
            batch_mode: self.batch_mode,
        }
    }
}
