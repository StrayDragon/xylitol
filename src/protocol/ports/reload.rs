//! Hot-reload capability port — refresh from durable source without tearing down the host.

/// Compile-time constraint for types that can refresh owned state from disk/config.
///
/// Associated `Outcome` keeps diagnostics typed per implementor. This is **not** a
/// `dyn` plugin registry (xylitol is not an extension marketplace); product `/reload`
/// will orchestrate concrete reloads in a fixed order.
pub trait XyReloadable {
    /// Report / diagnostics from a reload attempt.
    type Outcome;

    /// Refresh from the durable source. Failure semantics are encoded in
    /// [`Self::Outcome`] (e.g. keep previous state and attach diagnostics) unless
    /// documented otherwise.
    fn reload(&mut self) -> Self::Outcome;
}
