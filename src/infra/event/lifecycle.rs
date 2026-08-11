//! Lifecycle event vocabulary re-export for the EventBus implementation.
//!
//! Prefer `crate::protocol::lifecycle` / `crate::protocol::ports::event` in new code.

pub use crate::protocol::lifecycle::XyEvent;
pub use crate::protocol::ports::event::LifecycleHandler;
