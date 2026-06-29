//! Lifecycle event vocabulary.
//!
//! `AgentLifecycleEvent` lives in `domain::lifecycle`; `LifecycleHandler`
//! lives in `runtime_protocol::event`.
//!
//! This file remains as a thin re-export so `crate::infra::event::lifecycle`
//! references (the EventBus implementation here + tests) keep resolving.

pub use crate::domain::lifecycle::AgentLifecycleEvent;
pub use crate::runtime_protocol::event::LifecycleHandler;
