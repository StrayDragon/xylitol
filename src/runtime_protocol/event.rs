//! Runtime boundary for lifecycle event delivery.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

/// Lifecycle event emitted by the agent runtime.
///
/// Core-level representation so EventSink and SessionStore ports don't
/// depend on infra::event. The enriched infra-level event types live in
/// `infra::event::lifecycle::AgentLifecycleEvent`.
///
/// NOTE: Currently covers only compaction events — the events that the
/// compaction orchestrator emits. Turn/agent lifecycle events remain on
/// `EventBus` directly until the port migration is completed.
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    CompactionStarted {
        session_id: String,
        reason: String,
    },
    CompactionEnded {
        session_id: String,
        result: Option<String>,
        aborted: bool,
    },
}

/// Event emission port — abstracts lifecycle event delivery so the
/// loop can emit lifecycle events without knowing the concrete bus.
#[async_trait]
pub trait EventSink: Send + Sync {
    /// Emit a lifecycle event.
    async fn emit(&self, event: &LifecycleEvent);
}

/// A type-safe handler for [`crate::domain::lifecycle::AgentLifecycleEvent`].
pub type LifecycleHandler = Arc<
    dyn Fn(
            crate::domain::lifecycle::AgentLifecycleEvent,
        ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;
