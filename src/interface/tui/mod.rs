//! TUI mode — ratatui-based interactive terminal UI.
//!
//! This module is feature-gated behind `ui-tui`.

mod app;
mod component;
mod event;

mod chat;
mod history;
mod input;
mod markdown;
mod slash;
mod status_bar;
mod tool_panel;

mod overlays;

pub(crate) use app::run_tui;
