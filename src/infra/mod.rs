pub mod bash_exec;
pub mod config;
pub mod constants;
pub mod event;
pub mod export;
pub mod hooks;
pub mod mcp;
pub mod process;
pub mod provider;
pub mod source_info;
pub mod timing;
pub mod trust;

// ── settings manager (c35) ──
pub mod settings;

// ── resource loading ──
pub(crate) mod resource;

// ── session is always-on core ──
pub mod session;

pub mod browser;
pub mod clipboard;
pub mod fs_watch;
pub mod git;
pub mod image;
pub mod sandbox;
pub mod tools;
pub mod update;
