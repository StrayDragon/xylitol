//! Model management — configuration, registry, resolution, and manifest loading.
//!
//! Model config types ([`crate::domain::model::XyModelConfig`],
//! [`crate::domain::model::XyModelKind`]) live in [`crate::domain::model`];
//! this module provides the runtime orchestration layer:
//!
//! - [`ModelRegistry`] — manages available models
//! - [`ModelResolver`](resolver) — resolves model IDs to configs
//! - [`ModelManifest`] — loads model definitions from files

pub mod manager;
pub mod manifest;
pub mod registry;
pub mod resolver;

pub use manager::ModelManager;
pub use manifest::ModelManifest;
pub use registry::*;
