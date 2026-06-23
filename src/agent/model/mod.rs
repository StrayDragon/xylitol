//! Model management — configuration, registry, resolution, and manifest loading.
//!
//! Provides the core model abstraction layer: [`ModelConfig`] defines how to connect
//! to a provider, [`ModelRegistry`] manages available models, [`ModelResolver`] resolves
//! model IDs to configs, and [`ModelManifest`] loads model definitions from files.

pub mod config;
pub mod manifest;
pub mod registry;
pub mod resolver;

pub use config::*;
pub use manifest::ModelManifest;
pub use registry::*;

