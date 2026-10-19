//! Compaction failures: store IO vs product-copy policy gates.

use crate::error::{XyError, XySessionError, XySessionStoreError};

#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error(transparent)]
    Store(#[from] XySessionStoreError),
    #[error(transparent)]
    Session(#[from] XySessionError),
    /// Model construction / selection failed; keeps the typed `XyError` (kind/source)
    /// instead of being flattened into a `Policy` string (c2270 family).
    #[error(transparent)]
    Model(#[from] XyError),
    #[error("{0}")]
    Policy(String),
}

impl CompactionError {
    pub fn policy(message: impl Into<String>) -> Self {
        Self::Policy(message.into())
    }
}

impl From<String> for CompactionError {
    fn from(message: String) -> Self {
        Self::Policy(message)
    }
}

impl From<&str> for CompactionError {
    fn from(message: &str) -> Self {
        Self::Policy(message.to_string())
    }
}
