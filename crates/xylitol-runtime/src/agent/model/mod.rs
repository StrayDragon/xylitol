//! Model management — configuration, registry, and resolution.
//!
//! Model config types ([`xylitol_plugin_sdk::model::XyModelConfig`],
//! [`xylitol_plugin_sdk::model::XyModelKind`]) live in [`xylitol_plugin_sdk::model`];
//! this module provides the runtime orchestration layer:
//!
//! - [`registry::ModelRegistry`] — manages available models
//! - [`ModelResolver`](resolver) — resolves model IDs to configs

pub mod manager;
pub mod registry;
pub mod resolver;
pub mod task_model;

pub use manager::ModelManager;
