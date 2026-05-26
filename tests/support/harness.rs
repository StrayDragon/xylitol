use std::sync::Arc;

use futures::StreamExt;
use tempfile::TempDir;

use crate::agent::r#loop::{AgentEvent, AgentLoop};
use crate::agent::session::{InMemorySession, XySession};
use crate::agent::tools::ToolRegistry;
use crate::agent::traits::XyModel;

pub(crate) struct HarnessBuilder {
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,
    session: Arc<dyn XySession>,
    app_name: String,
    max_iterations: usize,
    system_prompt: Option<String>,
}

impl HarnessBuilder {
    pub(crate) fn with_model(mut self, model: Arc<dyn XyModel>) -> Self {
        self.model = model;
        self
    }

    pub(crate) fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    pub(crate) fn with_session(mut self, session: Arc<dyn XySession>) -> Self {
        self.session = session;
        self
    }

    pub(crate) fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    pub(crate) fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub(crate) fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub(crate) async fn build(self) -> TestHarness {
        TestHarness {
            _tempdir: tempfile::tempdir().ok(),
            model: self.model,
            tools: self.tools,
            session: self.session,
            app_name: self.app_name,
            max_iterations: self.max_iterations,
            system_prompt: self.system_prompt,
        }
    }
}

pub(crate) struct TestHarness {
    _tempdir: Option<TempDir>,
    model: Arc<dyn XyModel>,
    tools: ToolRegistry,
    session: Arc<dyn XySession>,
    app_name: String,
    max_iterations: usize,
    system_prompt: Option<String>,
}

impl TestHarness {
    pub(crate) fn builder() -> HarnessBuilder {
        use crate::agent::provider::MockXyModel;

        let model: Arc<dyn XyModel> = Arc::new(MockXyModel::new("harness-default").with_text("ok"));

        HarnessBuilder {
            model,
            tools: ToolRegistry::new(),
            session: Arc::new(InMemorySession::new()),
            app_name: "xylitol-test".to_string(),
            max_iterations: 5,
            system_prompt: None,
        }
    }

    pub(crate) async fn run(&mut self, prompt: &str, session_id: &str) -> Vec<AgentEvent> {
        if !self.session.exists(session_id).await {
            let _ = self.session.create(session_id).await;
        }

        let agent_loop = AgentLoop::with_model(
            self.model.clone(),
            &self.tools,
            self.session.clone(),
            self.system_prompt.clone(),
            self.max_iterations,
        );

        let mut stream = agent_loop
            .run(prompt, session_id, None)
            .await
            .expect("run agent loop");

        let mut out = Vec::new();
        while let Some(event) = stream.next().await {
            out.push(event);
        }
        out
    }
}
