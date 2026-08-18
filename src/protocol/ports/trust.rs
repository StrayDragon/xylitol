//! Runtime boundary for trust decision persistence.

use crate::protocol::error::XyTrustError;

/// Trust store port — abstracts persistence of project trust decisions.
pub trait XyTrustStore: Send + Sync {
    /// Persist a trust decision for a path.
    fn set_trust(&self, path: &str, trusted: Option<bool>) -> Result<(), XyTrustError>;
}
