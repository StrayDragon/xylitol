//! TUI mode — ratatui-based interactive terminal UI.
//!
//! Provides a full-screen terminal UI with:
//! - Chat message display with markdown rendering
//! - Streaming text delta output
//! - Tool execution result panel
//! - Diff preview overlay
//! - Command approval overlay
//! - Session/model/theme selectors
//! - Keyboard shortcut system
//!
//! ## Feature flag
//!
//! * `ui-tui` — enables this entire module (default on).
//!
//! ## Entry point
//!
//! [`run_tui`] is called from the CLI dispatcher when no prompt argument is given.

mod app;
mod approval;
mod chat;
mod diff_preview;
mod event;
mod help;
mod input;
mod markdown;
mod selectors;
mod status_bar;
mod tool_output;

pub(crate) use app::run_tui;
