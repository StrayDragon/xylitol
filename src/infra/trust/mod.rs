//! Project trust management — single source of truth (spec c255 / t1–t4).
//!
//! Trust decisions for project directories: persistence (pi-style JSON +
//! parent inheritance + file locking) and resolution (fixed-precedence
//! pipeline). Consumers call this module; interactive prompting is injected
//! via callback (spec t4) so this module stays UI-agnostic.
//!
//! Submodules:
//! - [`store`] — persistence, locking, inheritance, and trust-input detection
//! - [`resolve`] — the fixed-precedence resolution pipeline

pub(crate) mod resolve;
pub(crate) mod store;

pub use resolve::{
    DefaultProjectTrust, TrustReason, TrustResolution, format_trust_prompt, resolve_project_trusted,
};
pub use store::{TrustDecision, TrustManager, TrustOption, TrustUpdate};
