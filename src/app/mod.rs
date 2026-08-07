pub(crate) mod core;

/// Product slash-command catalog (app-surface SSOT; c1175).
pub use core::product_commands;

/// Shared tool call/result presentation (TUI + Print; c1460).
pub(crate) mod tool_display;

/// Hand-test `/debug <scene>` fixtures (c710). Delete this module to remove them.
pub(crate) mod debug_fixtures;

#[cfg(feature = "cli")]
pub mod cli;

pub mod server;

#[cfg(feature = "tui")]
pub mod tui;

#[cfg(feature = "gui")]
pub mod gui;
