//! Event and action types for the TUI runtime.

use crate::agent::r#loop::AgentEvent;

/// Events consumed by the TUI update loop.
#[derive(Debug, Clone)]
pub(crate) enum TuiEvent {
    /// Agent execution event (text delta, tool call, etc.).
    Agent(AgentEvent),
    /// Crossterm keyboard input event.
    Key(crossterm::event::KeyEvent),
    /// Frame tick (used for periodic redraw / spinners).
    Tick,
    /// Shutdown request (e.g. terminal reader exited).
    Shutdown,
}

/// High-level actions requested by the [`super::app::App`] update step.
#[derive(Debug, Clone)]
pub(crate) enum AppAction {
    /// Submit a prompt to the agent immediately.
    RunPrompt(String),
    /// Interrupt the currently running agent task.
    Interrupt,
    /// Clear the chat / tool panels.
    Clear,
    /// Update the current diff preview payload.
    SetDiff(String),
    /// Queue a prompt while the agent is running.
    QueuePrompt(String),
}
