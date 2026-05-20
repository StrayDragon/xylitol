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
mod keyboard_modes;
mod keymap;
mod markdown;
mod slash;
mod status_bar;
mod terminal_modes;
mod terminal_probe;
mod tool_panel;

mod overlays;

#[cfg(test)]
mod input_tests;

pub(crate) use app::run_tui;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use chat::ChatComponent;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use component::Component;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use event::TuiEvent;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use markdown::MarkdownRenderer;
