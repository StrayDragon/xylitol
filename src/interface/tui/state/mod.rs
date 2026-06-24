//! TUI state layer — pure domain objects, no ratatui dependency.
//!
//! This module MUST NOT import from ratatui, crossterm, or any TUI crate.
//! It only depends on `crate::core` and `crate::agent::loop::AgentEvent`.

pub(crate) mod composer;
pub(crate) mod focus;
pub(crate) mod transcript;

#[allow(unused_imports)]
pub(crate) use composer::{Composer, EscOutcome};
pub(crate) use focus::FocusCtx;
pub(crate) use transcript::{ToolStatus, Transcript, TranscriptEntry};

/// Application aggregate root — holds all TUI state.
#[derive(Debug)]
pub(crate) struct App {
    /// Conversation transcript.
    pub(crate) transcript: Transcript,
    /// Input composer.
    pub(crate) composer: Composer,
    /// Current focus.
    pub(crate) focus: FocusCtx,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            transcript: Transcript::new(),
            composer: Composer::new(),
            focus: FocusCtx::Composer,
        }
    }
}
