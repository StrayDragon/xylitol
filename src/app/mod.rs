pub mod core;

/// Product slash-command catalog (app-surface SSOT; c1175).
pub use core::product_commands;

/// Interactive bang (`!` / `!!`) executor (app-surface).
pub use core::bang_exec;

/// Session HTML/JSONL export-import (app-surface; stable seam is crate-root `SessionExporter`).
pub use core::session_export;

/// Shared tool call/result presentation (TUI + Print; c1460).
pub(crate) mod tool_display;

/// Hand-test `/debug <scene>` fixtures (c710). Delete this module to remove them.
pub(crate) mod debug_fixtures;

#[cfg(feature = "cli")]
pub mod cli;

pub mod server;

// Test-support contract (spec `package-tui-testing` @req:r1668): headless frame
// mounting for BDD / integration tests driving real product rendering.
#[cfg(feature = "tui")]
pub mod tui;
