//! Model management — configuration, registry, and resolution.
//!
//! Model config types ([`crate::protocol::model::XyModelConfig`],
//! [`crate::protocol::model::XyModelKind`]) live in [`crate::protocol::model`];
//! this module provides the runtime orchestration layer:
//!
//! - [`ModelRegistry`] — manages available models
//! - [`ModelResolver`](resolver) — resolves model IDs to configs

pub mod manager;
pub mod registry;
pub mod resolver;

#[cfg(test)]
pub use manager::ModelManager;
