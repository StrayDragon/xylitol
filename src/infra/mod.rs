pub mod config;
pub mod hooks;

// ── resource loading ──
pub(crate) mod resource;

// ── session is always-on core ──
pub mod session;

#[cfg(feature = "infra-lsp")]
pub(crate) mod lsp;

#[cfg(feature = "infra-dap")]
pub(crate) mod dap;

#[cfg(feature = "infra-skills")]
pub(crate) mod skills;
