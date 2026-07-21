//! Runtime boundary for lifecycle event delivery.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::protocol::lifecycle::XyEvent;

/// Event emission port — abstracts lifecycle event delivery so the
/// loop can emit lifecycle events without knowing the concrete bus.
#[async_trait]
pub trait XyEventSink: Send + Sync {
    /// Emit a lifecycle event.
    async fn emit(&self, event: &XyEvent);
}

/// A type-safe handler for [`XyEvent`].
pub type LifecycleHandler =
    Arc<dyn Fn(XyEvent) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;
