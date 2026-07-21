//! Lifecycle event vocabulary.
//!
//! `XyEvent` lives in `domain::lifecycle`; `LifecycleHandler`
//! lives in `protocol::ports::event`.
//!
//! This file remains as a thin re-export so `crate::infra::event::lifecycle`
//! references (the EventBus implementation here + tests) keep resolving.

pub use crate::protocol::lifecycle::XyEvent;
pub use crate::protocol::ports::event::LifecycleHandler;
