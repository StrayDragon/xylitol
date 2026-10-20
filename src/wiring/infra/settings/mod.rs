//! SettingsManager — three-tier user preference merge and persistence.
//!
//! Three-tier deep merge (global < project < overrides), file locking with
//! retry. Settings changes apply on next bootstrap; there is no live hot-reload wiring.
//! Project YAML (`config.yaml`) is `infra::config`. This module is user prefs
//! (`settings.json`).

pub mod manager;
pub mod storage;
pub mod types;

pub use manager::SettingsManager;
pub use types::{Settings, SteeringMode};
