//! Agent facade — the single public entry point for interactive layers.
//!
//! Interactive code (cli/print/rpc) should import only from here. Reaching into
//! `agent::runtime` / `agent::session` / `agent::tools` directly is a layering
//! violation, except in the composition root (`interactive::cli`) which wires
//! concrete adapters at construction.
//!
//! This facade is the in-process half of the Driver abstraction (see c265);
//! a future `RemoteDriver` will mirror it over the wire.

use crate::agent::model::registry::ModelRegistry;
use crate::agent::runtime::AgentLoop;
use crate::agent::session::AgentSession;
use crate::agent::tools::ToolRegistry;
use crate::core::ports::ToolExecutionMode;
use crate::infra::session::SessionManager;
use tokio_util::sync::CancellationToken;

pub use crate::agent::runtime::{AgentEvent, AgentEventStream, AgentHooks};

/// The agent — owns the session and the runtime loop.
///
/// Two construction paths:
/// - [`Agent::new`] — takes a fully-configured [`AgentSession`] (legacy).
/// - [`Agent::with_ports`] — takes [`SessionStore`] / [`EventSink`] ports
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
    /// Creates an in-memory AgentSession, holds the ports for future use
    /// (server backends, test doubles). The session_id is auto-generated when
    /// not provided.
    pub fn with_ports(
        model_registry: ModelRegistry,
        tool_registry: ToolRegistry,
        system_prompt: Option<String>,
        max_iterations: u32,
    ) -> Self {
        let session_mgr = SessionManager::in_memory();
        let session = AgentSession::new(
            model_registry,
            tool_registry,
            session_mgr,
            system_prompt,
            max_iterations,
            0.8,
            ".".into(),
            None,
        );
        Self {
            loop_: AgentLoop::new(session),
        }
    }

    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.loop_ = self.loop_.with_hooks(hooks);
        self
    }

    pub fn with_tool_mode(mut self, mode: ToolExecutionMode) -> Self {
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
    pub async fn run(&mut self, prompt: &str) -> AgentEventStream {
        self.run_with_id(prompt, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id (legacy).
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> AgentEventStream {
        self.loop_.run(prompt, session_id).await
    }
}
