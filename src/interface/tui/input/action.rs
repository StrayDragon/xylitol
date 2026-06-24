//! Input layer 3: Action enum — the result of keymap resolution.
//!
//! Actions are applied to the App state by the main event loop.
//! Some actions also require access to the AgentLoop handle.

use crate::interface::tui::state::App;

/// Actions that can be dispatched from keyboard input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// No action needed.
    None,
    /// Submit the current composer draft as a user message.
    Submit(String),
    /// Queue the current draft (agent is running).
    QueueInput,
    /// Clear the composer.
    ClearComposer,
    /// Prime backtrack (first Esc on empty composer).
    PrimeBacktrack,
    /// Load the last user message into composer.
    LoadLastUserMessage,
    /// Dequeue the next queued message.
    Dequeue,
    /// Quit the TUI.
    Quit,
    /// Cycle focus.
    CycleFocus,
    /// Insert a character into the composer.
    InsertChar(char),
    /// Special: pass-through to the TextArea for its own handling.
    TextAreaInput,
    /// Scroll the transcript up (back in history) by N entries.
    ScrollUp(usize),
    /// Scroll the transcript down (towards latest) by N entries.
    ScrollDown(usize),
}

impl Action {
    /// Apply the action to the App state.
    ///
    /// Returns `true` if the app should continue, `false` to quit.
    pub(crate) fn apply(self, app: &mut App) -> bool {
        match self {
            Action::None => true,
            Action::Submit(text) => {
                // Update transcript with user message
                app.transcript
                    .apply(crate::agent::r#loop::AgentEvent::MessageStart {
                        role: "user".into(),
                    });
                app.transcript
                    .apply(crate::agent::r#loop::AgentEvent::TextDelta(text.clone()));
                app.transcript
                    .apply(crate::agent::r#loop::AgentEvent::MessageEnd {
                        role: "user".into(),
                    });
                true
            }
            Action::QueueInput => {
                // Composer handles queuing internally
                true
            }
            Action::ClearComposer => {
                app.composer.clear();
                true
            }
            Action::PrimeBacktrack => {
                // Handled in composer's handle_esc
                true
            }
            Action::LoadLastUserMessage => {
                // Handled in composer's handle_esc
                true
            }
            Action::Dequeue => {
                // Will be handled by main loop
                true
            }
            Action::Quit => false,
            Action::CycleFocus => {
                app.focus = app.focus.cycle();
                true
            }
            Action::InsertChar(_) => true,
            Action::TextAreaInput => true,
            Action::ScrollUp(n) => {
                app.transcript.scroll_up(n);
                true
            }
            Action::ScrollDown(n) => {
                app.transcript.scroll_down(n);
                true
            }
        }
    }
}
