//! Agent facade — the single public entry point for interactive layers.
//!
//! Interactive code (cli/print/rpc) should import only from here. Reaching into
//! `agent::runtime` / `agent::session` / `agent::tools` directly is a layering
//! violation, except in the composition root (`app::cli`) which wires
//! concrete adapters at construction.
//!
//! This facade is the in-process half of the Driver abstraction (see c265);
//! the remote half is `app::server::ws` / `app::server::rest` (ClientFrame/ServerFrame
//! speak the same protocol over the wire).

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::agent::compaction::CompactionSettings;
use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::AgentLoop;
use crate::agent::session::AgentSession;
use crate::agent::tools::ToolRegistry;
use crate::runtime_protocol::{
    XyBashExecutor, XyEventSink, XyExportIo, XySessionStore, XyToolExecutionMode,
};

pub use crate::agent::runtime::{AgentHooks, XyEventStream};
pub use crate::domain::lifecycle::XyEvent;

/// The agent — owns the session and the runtime loop.
///
/// Two construction paths:
/// - [`Agent::new`] — takes a fully-configured [`AgentSession`] (legacy).
/// - [`Agent::with_ports`] — takes [`XySessionStore`] / [`XyEventSink`] ports
///   (HC-2 route, testable without file I/O).
pub struct Agent {
    loop_: AgentLoop,
}

impl Agent {
    /// Construct from a fully-configured session (legacy).
    pub fn new(session: AgentSession) -> Self {
        Self {
            loop_: AgentLoop::new(session),
        }
    }

    /// Construct from ports (HC-2 route).
    ///
    /// Takes [`XySessionStore`] / [`XyEventSink`] port trait objects (the agent
    /// holds no concrete infra session type), plus an injected model builder
    /// and sandbox engine (HC-1: agent must not construct infra
    /// providers/sandboxes itself; the composition root supplies them).
    #[allow(clippy::too_many_arguments)]
    pub fn with_ports(
        model_registry: ModelRegistry,
        tool_registry: ToolRegistry,
        store: Arc<dyn XySessionStore>,
        sink: Arc<dyn XyEventSink>,
        system_prompt: Option<String>,
        context_files: Vec<(String, String)>,
        append_system_prompt: Vec<String>,
        max_iterations: u32,
        compaction_threshold: f64,
        cwd: String,
        compaction_settings: Option<CompactionSettings>,
        model_builder: crate::runtime_protocol::XyModelBuilder,
        sandbox: Arc<dyn crate::runtime_protocol::XySandboxEngine>,
        bash_executor: Arc<dyn XyBashExecutor>,
        export_io: Arc<dyn XyExportIo>,
    ) -> Self {
        let session = AgentSession::new(
            model_registry,
            tool_registry,
            store,
            sink,
            system_prompt,
            context_files,
            append_system_prompt,
            max_iterations,
            compaction_threshold,
            cwd,
            compaction_settings,
            model_builder,
            sandbox,
            bash_executor,
            export_io,
        );
        Self {
            loop_: AgentLoop::new(session),
        }
    }

    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.loop_ = self.loop_.with_hooks(hooks);
        self
    }

    pub fn with_tool_mode(mut self, mode: XyToolExecutionMode) -> Self {
        self.loop_ = self.loop_.with_tool_mode(mode);
        self
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.loop_.cancel_token()
    }

    pub fn abort(&self) {
        self.loop_.abort();
    }

    pub fn session(&self) -> &AgentSession {
        self.loop_.session()
    }

    pub fn session_mut(&mut self) -> &mut AgentSession {
        self.loop_.session_mut()
    }

    /// Run a turn (port-based, session_id auto-generated).
    pub async fn run(&mut self, prompt: &str) -> XyEventStream {
        self.run_with_id(prompt, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id (legacy).
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> XyEventStream {
        self.loop_.run(prompt, session_id).await
    }
}
