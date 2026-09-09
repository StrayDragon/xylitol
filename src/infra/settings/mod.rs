//! SettingsManager — three-tier user preference merge and persistence.
//!
//! Three-tier deep merge (global < project < overrides), file locking with
//! retry. Settings changes apply on next bootstrap; there is no live hot-reload wiring.
//!
//! NOTE(c35): New module. Old `src/infra/config/` is project configuration
//! (models, tools, hooks). SettingsManager lives alongside it for user prefs.

pub mod manager;
pub mod storage;
pub mod types;

pub use manager::SettingsManager;
#[cfg(test)]
pub use types::{Settings, SteeringMode};
