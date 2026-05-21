//! Event and action types for the TUI runtime.

use crate::agent::r#loop::AgentEvent;

/// Events consumed by the TUI update loop.
#[derive(Debug, Clone)]
pub(crate) enum TuiEvent {
    /// Agent execution event (text delta, tool call, etc.).
    Agent(AgentEvent),
    /// Crossterm keyboard input event.
    Key(crossterm::event::KeyEvent),
    /// Bracketed paste payload (requires `EnableBracketedPaste`).
    Paste(String),
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

    /// Queue-or-submit semantics (Codex-style composer Tab key).
    ///
    /// - When the app is running: queue the draft.
    /// - When idle: submit immediately (except for bang-shell drafts).
    ///
    /// This is handled at the app layer because the composer itself does not
    /// own the "running" state.
    QueueOrSubmit(String),
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

    /// Prime backtrack mode (Esc on empty composer).
    BacktrackPrime,

    /// Edit last user message (Esc Esc on empty composer).
    BacktrackEditLast,

    /// Open transcript overlay.
    OpenTranscript,

    /// Copy latest assistant response as Markdown.
    CopyLastResponse,

    /// Copy arbitrary text to the clipboard (used by overlays).
    CopyText(String),

    /// Toggle raw output mode.
    ToggleRawOutput,
}
