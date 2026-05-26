use std::sync::Arc;

use adk_agent::LlmAgentBuilder;
use adk_core::{Content, Llm};
use adk_runner::Runner;
use adk_session::{CreateRequest, SessionService};
use futures::StreamExt;
use tempfile::TempDir;

use crate::agent::r#loop::{AgentEvent, AgentEventStream};
use crate::agent::tools::ToolRegistry;

pub(crate) struct HarnessBuilder {
    model: Arc<dyn Llm>,
    tools: ToolRegistry,
    session_service: Arc<dyn SessionService>,
    app_name: String,
    max_iterations: u32,
    system_prompt: Option<String>,
}

impl HarnessBuilder {
    pub(crate) fn with_model(mut self, model: Arc<dyn Llm>) -> Self {
        self.model = model;
        self
    }

    pub(crate) fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    pub(crate) fn with_session_service(mut self, session_service: Arc<dyn SessionService>) -> Self {
        self.session_service = session_service;
        self
    }

    pub(crate) fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    pub(crate) fn with_max_iterations(mut self, max_iterations: u32) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub(crate) fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub(crate) async fn build(self) -> TestHarness {
        let mut builder = LlmAgentBuilder::new("xylitol-test-agent")
            .model(self.model)
            .description("xylitol test harness agent")
            .max_iterations(self.max_iterations);

        if let Some(prompt) = self.system_prompt {
            builder = builder.instruction(prompt);
        }

        for tool in self.tools.list().iter().cloned() {
            builder = builder.tool(tool);
        }

        let agent = builder.build().expect("build test agent");

        let runner = Runner::builder()
            .app_name(self.app_name.clone())
            .agent(Arc::new(agent))
            .session_service(self.session_service.clone())
            .build()
            .expect("build test runner");

        TestHarness {
            _tempdir: tempfile::tempdir().ok(),
            runner,
            session_service: self.session_service,
            app_name: self.app_name,
        }
    }
}

pub(crate) struct TestHarness {
    // Provide a per-harness tempdir to encourage isolation even if individual tests opt into
    // filesystem-backed components later.
    _tempdir: Option<TempDir>,
    runner: Runner,
    session_service: Arc<dyn SessionService>,
    app_name: String,
}

impl TestHarness {
    pub(crate) fn builder() -> HarnessBuilder {
        let model = Arc::new(
            crate::agent::provider::MockLlm::new("harness-default").with_response(
                adk_core::LlmResponse::new(Content::new("assistant").with_text("ok")),
            ),
        );

        HarnessBuilder {
            model,
            tools: ToolRegistry::new(),
            session_service: super::in_memory::SessionManager::in_memory(),
            app_name: "xylitol-test".to_string(),
            max_iterations: 5,
            system_prompt: None,
        }
    }

    pub(crate) async fn run(&mut self, prompt: &str, session_id: &str) -> Vec<AgentEvent> {
        // Load-or-create session so Runner's `get()` succeeds.
        let _ = self
            .session_service
            .create(CreateRequest {
                app_name: self.app_name.clone(),
                user_id: "default-user".into(),
                session_id: Some(session_id.into()),
                state: std::collections::HashMap::new(),
            })
            .await;

        let content = Content::new("user").with_text(prompt);
        let raw_stream = self
            .runner
            .run_str("default-user", session_id, content)
            .await
            .expect("runner run");

        let mut stream = AgentEventStream::new_raw(raw_stream, /*step*/ 1);
        let mut out = Vec::new();
        while let Some(event) = stream.next().await {
            out.push(event);
        }
        out
    }
}
