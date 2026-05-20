//! TUI mode — ratatui-based interactive terminal UI.
//!
//! This module is feature-gated behind `ui-tui`.

mod app;
mod approval;
mod component;
mod event;

mod bottom_pane;

mod chat;
mod chat_style;
mod history;
mod input;
mod keymap;
mod markdown;
mod slash;
mod status_bar;
mod tool_panel;

mod overlays;

#[cfg(test)]
mod input_tests;

pub(crate) use app::run_tui;
