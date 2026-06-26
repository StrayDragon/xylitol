//! Model management — configuration, registry, resolution, and manifest loading.
//!
//! Model config types ([`ModelConfig`], [`ModelKind`]) live in [`crate::core::model`];
//! this module provides the runtime orchestration layer:
//!
//! - [`ModelRegistry`](registry::ModelRegistry) — manages available models
//! - [`ModelResolver`](resolver) — resolves model IDs to configs
//! - [`ModelManifest`](manifest::ModelManifest) — loads model definitions from files

pub mod config;
pub mod manager;
pub mod manifest;
pub mod registry;
pub mod resolver;

pub use manager::ModelManager;
pub use manifest::ModelManifest;
pub use registry::*;
