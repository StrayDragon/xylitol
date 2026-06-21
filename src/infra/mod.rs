pub mod config;
pub mod process;
pub mod event;
pub mod hooks;
pub mod source_info;
pub mod timing;
pub mod trust;

// ── settings manager (c35) ──
pub mod settings;

// ── resource loading ──
pub(crate) mod resource;

// ── session is always-on core ──
pub mod session;

#[cfg(feature = "infra-clipboard")]
pub mod clipboard;

#[cfg(feature = "infra-image")]
pub mod image;

#[cfg(feature = "infra-skills")]
pub(crate) mod skills;
