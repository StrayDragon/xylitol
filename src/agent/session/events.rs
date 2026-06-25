//! Event bus access and lifecycle event emission (spec c255 / as32).
//!
//! Grouped here to keep `mod.rs` focused. These remain methods on
//! [`AgentSession`](super::AgentSession) (preserving the public API per as31).

use crate::infra::event::EventBus;
use crate::infra::event::lifecycle::AgentLifecycleEvent;

impl super::AgentSession {
    /// Get a reference to the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Subscribe to all lifecycle events.
    ///
    /// The handler receives every [`AgentLifecycleEvent`] emitted during agent
    /// execution. Returns an [`UnsubscribeHandle`] — drop it to unsubscribe.
    pub fn subscribe<F, Fut>(&mut self, handler: F)
    where
        F: Fn(AgentLifecycleEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let handle = self.event_bus.on_lifecycle(handler);
        self.lifecycle_handle = Some(handle);
    }

    /// Remove the lifecycle subscription.
    pub fn unsubscribe(&mut self) {
        self.lifecycle_handle.take();
    }

    /// Emit a turn_start lifecycle event.
    pub fn begin_turn(&self, turn_index: u32) {
        self.event_bus
            .emit_lifecycle(&AgentLifecycleEvent::TurnStart { turn_index });
    }

    /// Emit a turn_end lifecycle event.
    pub fn end_turn(&self, turn_index: u32) {
        self.event_bus
            .emit_lifecycle(&AgentLifecycleEvent::TurnEnd { turn_index });
    }

    /// Emit an agent_start lifecycle event.
    pub fn emit_agent_start(&self, session_id: &str, model: &str) {
        self.event_bus
            .emit_lifecycle(&AgentLifecycleEvent::AgentStart {
                session_id: session_id.to_string(),
                model: model.to_string(),
            });
    }

    /// Emit an agent_end lifecycle event.
    pub fn emit_agent_end(&self, session_id: &str, reason: &str) {
        self.event_bus
            .emit_lifecycle(&AgentLifecycleEvent::AgentEnd {
                session_id: session_id.to_string(),
                reason: reason.to_string(),
            });
    }

    /// Emit a model_select lifecycle event.
    pub fn emit_model_select(&self, provider: &str, model_id: &str) {
        self.event_bus
            .emit_lifecycle(&AgentLifecycleEvent::ModelSelect {
                provider: provider.to_string(),
                model_id: model_id.to_string(),
            });
    }
}
