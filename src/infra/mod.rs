pub mod config;
pub mod hooks;

// ── session is always-on core ──
pub mod session;

#[cfg(feature = "infra-lsp")]
pub mod lsp;

#[cfg(feature = "infra-dap")]
pub mod dap;

#[cfg(feature = "infra-skills")]
pub mod skills;
