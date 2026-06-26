//! Driver abstraction — the single dependency of interactive layers.
//!
//! All interactive clients (cli/print/rpc) interact with the core through a
//! [`Driver`]; they never import `agent::facade` or `infra` directly.
//!
//! - [`InProcessDriver`]: wraps the local agent facade (composition root wires
//!   ports and agent together).
//! - `RemoteDriver` (future): speaks the protocol over WS/REST to a server.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use tokio_util::sync::CancellationToken;

use crate::agent::facade::{Agent, AgentEvent, AgentHooks};
use crate::core::ports::ToolExecutionMode;

/// A stream of [`AgentEvent`] items.
pub type EventStream = Pin<Box<dyn Stream<Item = AgentEvent> + Send>>;

/// Driver — interact with the core without knowing its internals.
///
/// [`InProcessDriver`] keeps a cached `Agent` and is the local (single-process)
/// implementation. A future `RemoteDriver` will speak the protocol over WS/REST.
#[async_trait]
pub trait Driver {
    /// Submit a prompt and receive a stream of events.
    async fn run(&mut self, prompt: &str) -> EventStream;

    /// Cancel the current turn.
    fn abort(&self);
}

/// In-process driver wrapping the local agent facade.
///
/// Constructed at the composition root (`interactive::cli`) which wires ports
/// and agent together. This is the **only** place in `interactive/` that
/// imports `agent::facade`.
pub struct InProcessDriver {
    agent: Agent,
}

impl InProcessDriver {
    pub fn new(agent: Agent) -> Self {
        Self { agent }
    }

    pub fn with_hooks(mut self, hooks: AgentHooks) -> Self {
        self.agent = self.agent.with_hooks(hooks);
        self
    }

    pub fn with_tool_mode(mut self, mode: ToolExecutionMode) -> Self {
        self.agent = self.agent.with_tool_mode(mode);
        self
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.agent.cancel_token()
    }
}

#[async_trait]
impl Driver for InProcessDriver {
    async fn run(&mut self, prompt: &str) -> EventStream {
        let stream = self.agent.run(prompt).await;
        Box::pin(stream)
    }

    fn abort(&self) {
        self.agent.abort();
    }
}
