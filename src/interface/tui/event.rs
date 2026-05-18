//! Event types and dispatcher for the TUI event loop.

use crate::agent::r#loop::AgentEvent;

/// Events handled by the TUI event loop.
#[derive(Debug)]
pub(crate) enum AppEvent {
    /// Agent execution event (text delta, tool call, etc.).
    Agent(AgentEvent),
    /// Crossterm key event.
    Key(crossterm::event::KeyEvent),
    /// Tick timer fired (250 ms).
    Tick,
    /// Shutdown the TUI.
    Shutdown,
}
