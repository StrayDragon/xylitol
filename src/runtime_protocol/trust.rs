//! Runtime boundary for trust decision persistence.

/// Trust store port — abstracts persistence of project trust decisions.
pub trait TrustStore: Send + Sync {
    /// Persist a trust decision for a path.
    fn set_trust(&self, path: &str, trusted: Option<bool>) -> Result<(), String>;
}
