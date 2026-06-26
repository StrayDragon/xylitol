//! Agent facade — the single public entry point for interactive layers.
//!
//! Interactive code (cli/print/rpc) should import only from here. Reaching into
//! `agent::runtime` / `agent::session` / `agent::tools` directly is a layering
//! violation, except in the composition root (`interactive::cli`) which wires
//! concrete adapters at construction.
//!
//! This facade is the in-process half of the Driver abstraction (see plan D);
//! a future `RemoteDriver` will mirror it over the wire.
//!
//! NOTE: `Agent::new` still takes an `AgentSession` and `run` takes a
//! `session_id`. ceiling: this couples the orchestration entry to Session.
//! upgrade: P4 swaps these for injected `Arc<dyn Port>` trait objects and moves
//! `session_id` to the SessionStore query key, so the facade stops owning
//! session lifecycle.

use crate::agent::runtime::AgentLoop;
use crate::agent::session::AgentSession;
use crate::core::traits::ToolExecutionMode;
use tokio_util::sync::CancellationToken;

// Public types interactive layers need (events, hooks, construction types).
pub use crate::agent::runtime::{AgentEvent, AgentEventStream, AgentHooks};

/// The agent — owns the session and the runtime loop.
///
/// Constructed at the composition root (`interactive::cli`) with concrete
/// adapters; interactive layers only see this type and [`AgentEvent`].
pub struct Agent {
    loop_: AgentLoop,
}

impl Agent {
    pub fn new(session: AgentSession) -> Self {
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

    /// Run a turn. Returns a stream of events for the interface to render.
    pub async fn run(&mut self, prompt: &str, session_id: &str) -> AgentEventStream {
        self.loop_.run(prompt, session_id).await
    }
}
