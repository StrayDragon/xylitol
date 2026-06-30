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

use crate::agent::runtime::AgentLoop;
use crate::agent::tools::ToolSet;
use crate::runtime_protocol::{XyPermission, XyToolExecutionMode};

pub use crate::agent::builder::AgentBuilder;
pub use crate::agent::runtime::hooks::BeforeToolHook;
pub use crate::agent::runtime::{AgentHooks, XyEventStream};
pub use crate::domain::lifecycle::XyEvent;

/// The agent — owns the session and the runtime loop.
pub struct Agent {
    pub(crate) loop_: AgentLoop,
}

impl Agent {
    pub fn cancel_token(&self) -> CancellationToken {
        self.loop_.cancel_token()
    }

    pub fn abort(&self) {
        self.loop_.abort();
    }

    pub fn session(&self) -> &crate::agent::session::AgentSession {
        self.loop_.session()
    }

    pub fn session_mut(&mut self) -> &mut crate::agent::session::AgentSession {
        self.loop_.session_mut()
    }

    /// Replace the tool set. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tools(&mut self, tools: ToolSet) {
        self.loop_.session_mut().set_tools(tools);
    }

    /// Replace the hook set. Takes effect on the next [`run`](Self::run) call.
    pub fn replace_hooks(&mut self, hooks: AgentHooks) {
        self.loop_.session_mut().replace_hooks(hooks);
    }

    /// Add a before-tool hook. Takes effect on the next [`run`](Self::run) call.
    pub fn add_hook(&mut self, hook: BeforeToolHook) {
        self.loop_.session_mut().hooks_mut().add_before(hook);
    }

    /// Set the permission port. Takes effect on the next [`run`](Self::run) call.
    pub fn set_permission(&mut self, permission: Arc<dyn XyPermission>) {
        self.loop_.session_mut().set_permission(permission);
    }

    /// Set the tool execution mode. Takes effect on the next [`run`](Self::run) call.
    pub fn set_tool_mode(&mut self, mode: XyToolExecutionMode) {
        self.loop_.session_mut().set_tool_mode(mode);
    }

    /// Set the system prompt. Takes effect on the next [`run`](Self::run) call.
    pub fn set_system_prompt(&mut self, prompt: Option<String>) {
        self.loop_.session_mut().set_system_prompt(prompt);
    }

    /// Run a turn with an auto-generated session_id.
    pub async fn run(&mut self, prompt: &str) -> XyEventStream {
        self.run_with_id(prompt, &uuid::Uuid::new_v4().to_string())
            .await
    }

    /// Run a turn with an explicit session_id.
    pub async fn run_with_id(&mut self, prompt: &str, session_id: &str) -> XyEventStream {
        self.loop_.run(prompt, session_id).await
    }
}
