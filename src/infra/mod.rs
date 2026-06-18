pub mod config;
pub mod event;
pub mod hooks;
pub mod trust;

// ── resource loading ──
pub(crate) mod resource;

// ── session is always-on core ──
pub mod session;

#[cfg(feature = "infra-skills")]
pub(crate) mod skills;
