//! Lifecycle event vocabulary — relocated to `core::lifecycle`.
//!
//! This file remains as a thin re-export so `crate::infra::event::lifecycle`
//! references (the EventBus implementation here + tests) keep resolving.
//! New code should import from `crate::core::lifecycle`.

pub use crate::core::lifecycle::{AgentLifecycleEvent, LifecycleHandler};
