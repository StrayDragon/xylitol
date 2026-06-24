//! Input layer 2: Keymap — pure function mapping from (InputKey, FocusCtx, AgentState) to Action.
//!
//! This is a PURE FUNCTION with no I/O or TUI dependencies.
//! Fully unit-testable by feeding (InputKey, FocusCtx, AgentState) tuples.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::interface::tui::input::action::Action;
use crate::interface::tui::input::decode::InputKey;
use crate::interface::tui::state::FocusCtx;

/// Minimal agent state for keymap decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgentState {
    /// Whether the agent is currently processing (streaming / tool running).
    pub(crate) is_running: bool,
    /// Whether the composer has a non-empty draft.
    pub(crate) has_draft: bool,
    /// Whether the draft starts with '!' (bang shell mode).
    pub(crate) is_bang: bool,
    /// Whether backtrack is primed (first Esc on empty).
    pub(crate) backtrack_primed: bool,
}

/// Resolve an input key + context into an Action.
///
/// Pure function — no side effects.
pub(crate) fn resolve(input: &InputKey, focus: FocusCtx, state: &AgentState) -> Action {
    match focus {
        FocusCtx::Composer => resolve_composer(input, state),
        FocusCtx::Transcript => resolve_transcript(input, state),
    }
}

/// Resolve keys when composer has focus.
fn resolve_composer(input: &InputKey, state: &AgentState) -> Action {
    match (input.modifiers, input.code) {
        // Quit
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('d')) => Action::Quit,

        // Submit: Enter when idle (handled in composer submit logic)
        // Tab: queue when running, submit when idle (non-bang), no-op when bang+idle
        (KeyModifiers::NONE, KeyCode::Tab) => {
            if state.is_running {
                Action::QueueInput
            } else if state.is_bang {
                Action::None
            } else {
                // Submit - handled by composer.submit
                Action::TextAreaInput
            }
        }

        // Esc: clear, prime backtrack, or load last message
        (KeyModifiers::NONE, KeyCode::Esc) => {
            if state.has_draft {
                Action::ClearComposer
            } else if state.backtrack_primed {
                Action::LoadLastUserMessage
            } else {
                Action::PrimeBacktrack
            }
        }

        // Enter: submit (handled in main loop)
        (KeyModifiers::NONE, KeyCode::Enter) => Action::TextAreaInput,

        // Tab with Shift: cycle focus (future: could insert tab)
        (KeyModifiers::SHIFT, KeyCode::Tab) => Action::CycleFocus,

        // Regular character input → pass through to TextArea
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(_)) => Action::TextAreaInput,

        // Backspace, Delete → pass through
        (_, KeyCode::Backspace) => Action::TextAreaInput,
        (_, KeyCode::Delete) => Action::TextAreaInput,

        // Home, End, arrows → pass through
        (_, KeyCode::Home) => Action::TextAreaInput,
        (_, KeyCode::End) => Action::TextAreaInput,
        (_, KeyCode::Left) => Action::TextAreaInput,
        (_, KeyCode::Right) => Action::TextAreaInput,
        (_, KeyCode::Up) => Action::TextAreaInput,
        (_, KeyCode::Down) => Action::TextAreaInput,

        // Ctrl+K/U/W/A/E = readline shortcuts → pass through
        (KeyModifiers::CONTROL, KeyCode::Char('k' | 'u' | 'w' | 'a' | 'e')) => {
            Action::TextAreaInput
        }

        // Everything else → no action
        _ => Action::None,
    }
}

/// Resolve keys when transcript has focus.
fn resolve_transcript(input: &InputKey, _state: &AgentState) -> Action {
    let _ = _state;
    match (input.modifiers, input.code) {
        // Quit
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('d')) => Action::Quit,

        // Tab/Shift+Tab: cycle focus back to composer
        (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::SHIFT, KeyCode::Tab) => {
            Action::CycleFocus
        }

        // Escape: go back to composer
        (KeyModifiers::NONE, KeyCode::Esc) => Action::CycleFocus,

        // Scrolling
        (KeyModifiers::NONE, KeyCode::PageUp) => Action::ScrollUp(5),
        (KeyModifiers::NONE, KeyCode::PageDown) => Action::ScrollDown(5),
        (KeyModifiers::NONE, KeyCode::Up) => Action::ScrollUp(1),
        (KeyModifiers::NONE, KeyCode::Down) => Action::ScrollDown(1),

        // Everything else → no action
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> InputKey {
        InputKey {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    fn ctrl(code: KeyCode) -> InputKey {
        InputKey {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    fn shift(code: KeyCode) -> InputKey {
        InputKey {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    fn idle_state() -> AgentState {
        AgentState {
            is_running: false,
            has_draft: false,
            is_bang: false,
            backtrack_primed: false,
        }
    }

    fn running_state() -> AgentState {
        AgentState {
            is_running: true,
            has_draft: false,
            is_bang: false,
            backtrack_primed: false,
        }
    }

    fn draft_state() -> AgentState {
        AgentState {
            is_running: false,
            has_draft: true,
            is_bang: false,
            backtrack_primed: false,
        }
    }

    fn bang_state() -> AgentState {
        AgentState {
            is_running: false,
            has_draft: true,
            is_bang: true,
            backtrack_primed: false,
        }
    }

    fn primed_state() -> AgentState {
        AgentState {
            is_running: false,
            has_draft: false,
            is_bang: false,
            backtrack_primed: true,
        }
    }

    // ── Composer focus tests ──

    #[test]
    fn test_ctrl_c_quits() {
        assert_eq!(
            resolve(&ctrl(KeyCode::Char('c')), FocusCtx::Composer, &idle_state()),
            Action::Quit
        );
    }

    #[test]
    fn test_ctrl_d_quits() {
        assert_eq!(
            resolve(&ctrl(KeyCode::Char('d')), FocusCtx::Composer, &idle_state()),
            Action::Quit
        );
    }

    #[test]
    fn test_tab_running_queues() {
        assert_eq!(
            resolve(&key(KeyCode::Tab), FocusCtx::Composer, &running_state()),
            Action::QueueInput
        );
    }

    #[test]
    fn test_tab_idle_submit() {
        // idle + non-bang → Tab submits (returns TextAreaInput to let composer handle it)
        assert_eq!(
            resolve(&key(KeyCode::Tab), FocusCtx::Composer, &idle_state()),
            Action::TextAreaInput
        );
    }

    #[test]
    fn test_tab_idle_bang_noop() {
        assert_eq!(
            resolve(&key(KeyCode::Tab), FocusCtx::Composer, &bang_state()),
            Action::None
        );
    }

    #[test]
    fn test_esc_clears_draft() {
        assert_eq!(
            resolve(&key(KeyCode::Esc), FocusCtx::Composer, &draft_state()),
            Action::ClearComposer
        );
    }

    #[test]
    fn test_esc_empty_primes() {
        assert_eq!(
            resolve(&key(KeyCode::Esc), FocusCtx::Composer, &idle_state()),
            Action::PrimeBacktrack
        );
    }

    #[test]
    fn test_esc_primed_loads() {
        assert_eq!(
            resolve(&key(KeyCode::Esc), FocusCtx::Composer, &primed_state()),
            Action::LoadLastUserMessage
        );
    }

    #[test]
    fn test_enter_textarea() {
        assert_eq!(
            resolve(&key(KeyCode::Enter), FocusCtx::Composer, &idle_state()),
            Action::TextAreaInput
        );
    }

    #[test]
    fn test_char_input_textarea() {
        assert_eq!(
            resolve(&key(KeyCode::Char('a')), FocusCtx::Composer, &idle_state()),
            Action::TextAreaInput
        );
    }

    #[test]
    fn test_backspace_textarea() {
        assert_eq!(
            resolve(&key(KeyCode::Backspace), FocusCtx::Composer, &idle_state()),
            Action::TextAreaInput
        );
    }

    #[test]
    fn test_ctrl_k_textarea() {
        assert_eq!(
            resolve(&ctrl(KeyCode::Char('k')), FocusCtx::Composer, &idle_state()),
            Action::TextAreaInput
        );
    }

    #[test]
    fn test_shift_tab_cycles_focus() {
        assert_eq!(
            resolve(&shift(KeyCode::Tab), FocusCtx::Composer, &idle_state()),
            Action::CycleFocus
        );
    }

    // ── Transcript focus tests ──

    #[test]
    fn test_transcript_ctrl_c_quits() {
        assert_eq!(
            resolve(
                &ctrl(KeyCode::Char('c')),
                FocusCtx::Transcript,
                &idle_state()
            ),
            Action::Quit
        );
    }

    #[test]
    fn test_transcript_tab_cycles() {
        assert_eq!(
            resolve(&key(KeyCode::Tab), FocusCtx::Transcript, &idle_state()),
            Action::CycleFocus
        );
    }

    #[test]
    fn test_transcript_esc_cycles() {
        assert_eq!(
            resolve(&key(KeyCode::Esc), FocusCtx::Transcript, &idle_state()),
            Action::CycleFocus
        );
    }

    #[test]
    fn test_transcript_page_up_scrolls_up() {
        assert_eq!(
            resolve(&key(KeyCode::PageUp), FocusCtx::Transcript, &idle_state()),
            Action::ScrollUp(5)
        );
    }

    #[test]
    fn test_transcript_page_down_scrolls_down() {
        assert_eq!(
            resolve(&key(KeyCode::PageDown), FocusCtx::Transcript, &idle_state()),
            Action::ScrollDown(5)
        );
    }

    #[test]
    fn test_transcript_up_scrolls_line_up() {
        assert_eq!(
            resolve(&key(KeyCode::Up), FocusCtx::Transcript, &idle_state()),
            Action::ScrollUp(1)
        );
    }

    #[test]
    fn test_transcript_down_scrolls_line_down() {
        assert_eq!(
            resolve(&key(KeyCode::Down), FocusCtx::Transcript, &idle_state()),
            Action::ScrollDown(1)
        );
    }

    // ── Edge cases ──

    #[test]
    fn test_unknown_key_none() {
        assert_eq!(
            resolve(&key(KeyCode::F(1)), FocusCtx::Composer, &idle_state()),
            Action::None
        );
    }

    #[test]
    fn test_running_esc_clears() {
        let running_draft = AgentState {
            is_running: true,
            has_draft: true,
            ..idle_state()
        };
        assert_eq!(
            resolve(&key(KeyCode::Esc), FocusCtx::Composer, &running_draft),
            Action::ClearComposer
        );
    }

    #[test]
    fn test_all_ctrl_letter_keys() {
        // All Ctrl+letter combos should either be quit or textarea or none
        for c in 'a'..='z' {
            let result = resolve(&ctrl(KeyCode::Char(c)), FocusCtx::Composer, &idle_state());
            match c {
                'c' | 'd' => assert_eq!(result, Action::Quit, "Ctrl+{c}"),
                'k' | 'u' | 'w' | 'a' | 'e' => {
                    assert_eq!(result, Action::TextAreaInput, "Ctrl+{c}")
                }
                _ => assert_eq!(result, Action::None, "Ctrl+{c}"),
            }
        }
    }
}
