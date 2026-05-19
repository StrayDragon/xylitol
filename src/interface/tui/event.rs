//! Event and action types for the TUI runtime.

use crate::agent::r#loop::AgentEvent;

/// Events consumed by the TUI update loop.
#[derive(Debug, Clone)]
pub(crate) enum TuiEvent {
    /// Agent execution event (text delta, tool call, etc.).
    Agent(AgentEvent),
    /// Crossterm keyboard input event.
    Key(crossterm::event::KeyEvent),
    /// Crossterm mouse input event.
    Mouse(crossterm::event::MouseEvent),
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
    /// Switch the active agent profile for future runs.
    SelectProfile(String),
    /// Switch the active session and load its history.
    SelectSession(String),
    /// Switch the active syntax highlighting theme.
    SelectTheme(String),
    /// Open the current input buffer in $EDITOR and read it back.
    OpenEditor(String),
    /// Show interactive history search overlay.
    ShowHistorySearch,
    /// Load a string into the input buffer (without submitting).
    LoadInput(String),
}
