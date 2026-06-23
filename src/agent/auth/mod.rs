//! Authentication and authorization — guidance messages and credential storage.
//!
//! Provides user-facing guidance for model/auth configuration and
//! OAuth credential persistence.

mod guidance;
mod storage;

pub use guidance::*;
pub use storage::*;
