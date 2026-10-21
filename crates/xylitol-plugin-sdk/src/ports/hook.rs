use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XyHookOutcome {
    Allowed,
    Blocked { reason: String },
    Modified { args: Value },
}

#[async_trait]
pub trait XyHookBus: Send + Sync {
    async fn dispatch(&self, event_type: &str, phase: &str, context: Value) -> XyHookOutcome;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopHookBus;

#[async_trait]
impl XyHookBus for NoopHookBus {
    async fn dispatch(&self, _event_type: &str, _phase: &str, _context: Value) -> XyHookOutcome {
        XyHookOutcome::Allowed
    }
}
