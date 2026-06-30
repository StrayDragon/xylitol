//! Project trust decision helper (spec c255 / c320 T25).
//!
//! Demoted from a method on the [`crate::agent::session::Agent`] capability
//! aggregate to a free function: it reads only `cwd` and does not hold a trust
//! store, so it had no cohesive reason to live on the aggregate (design §5).

use crate::runtime_protocol::XyTrustStore;

/// Persist a project trust decision for the given CWD via the trust store
/// (single source of truth). Returns the persisted decision.
///
/// NOTE: pre-wired for the `/trust` and `/no-trust` slash commands, which are
/// not yet dispatched here. ceiling: until those commands route through this
/// function it has no production caller. upgrade: wire `/trust` dispatch to
/// call this with the session's CWD and injected trust store.
pub fn save_trust_decision(
    cwd: &str,
    trust_store: &dyn XyTrustStore,
    trusted: bool,
) -> Result<bool, String> {
    trust_store.set_trust(cwd, Some(trusted))?;
    Ok(trusted)
}
