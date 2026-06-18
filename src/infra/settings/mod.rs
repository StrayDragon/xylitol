//! SettingsManager — user preference management mirroring pi's settings-manager.ts.
//!
//! Three-tier deep merge (global < project < overrides), file locking with
//! retry, hot reload via EventBus `settings:changed` event.
//!
//! NOTE(c35): New module. Old `src/infra/config/` is project configuration
//! (models, tools, hooks). SettingsManager lives alongside it for user prefs.

pub mod manager;
pub mod storage;
pub mod types;

pub use manager::SettingsManager;
pub use types::*;
