pub mod bash_exec;
pub mod config;
pub mod event;
pub mod export;
pub mod hooks;
pub mod mcp;
pub(crate) mod observability;
pub mod process;
pub mod provider;
pub mod timing;
pub mod trust;

// ── settings manager (c35) ──
pub mod settings;

// ── resource loading ──
pub(crate) mod resource;

// ── session is always-on core ──
pub mod session;

pub mod clipboard;
pub mod image;
pub mod permission;
pub mod tools;
