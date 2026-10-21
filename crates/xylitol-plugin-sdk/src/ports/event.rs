use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::lifecycle::XyEvent;

#[async_trait]
pub trait XyEventSink: Send + Sync {
    async fn emit(&self, event: &XyEvent);
}

pub type LifecycleHandler =
    Arc<dyn Fn(XyEvent) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;
