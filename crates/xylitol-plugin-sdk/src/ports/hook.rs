//! Script hook bus port — agent layer dispatches lifecycle/provider events
//! without depending on infra hook implementations.

use async_trait::async_trait;
use serde_json::Value;

/// Outcome of a script hook dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XyHookOutcome {
    /// All matching hooks allowed the operation.
    Allowed,
    /// A hook blocked the operation.
    Blocked { reason: String },
    /// A hook modified the operation arguments.
    Modified { args: Value },
}

/// Event bus for config-driven script hooks.
///
/// Implemented by `infra::hooks::HookDispatcher` at the composition
/// root. When no hooks are configured, callers should pass `None` rather than
/// a bus instance (zero-cost no-op).
#[async_trait]
pub trait XyHookBus: Send + Sync {
    /// Dispatch an event to matching hooks.
    ///
    /// `event_type` uses pi-aligned names (e.g. `before_provider_request`,
    /// `tool_call`, `agent_start`). `phase` is `"pre"`, `"post"`, or `""`.
    async fn dispatch(&self, event_type: &str, phase: &str, context: Value) -> XyHookOutcome;
}

/// No-op hook bus — always returns [`XyHookOutcome::Allowed`].
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopHookBus;

#[async_trait]
impl XyHookBus for NoopHookBus {
    async fn dispatch(&self, _event_type: &str, _phase: &str, _context: Value) -> XyHookOutcome {
        XyHookOutcome::Allowed
    }
}
