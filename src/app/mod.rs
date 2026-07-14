pub(crate) mod core;

/// Hand-test `/debug <scene>` fixtures (c710). Delete this module to remove them.
pub(crate) mod debug_fixtures;

#[cfg(feature = "cli")]
pub mod cli;

pub mod server;

#[cfg(feature = "tui")]
pub mod tui;

#[cfg(feature = "gui")]
pub mod gui;
