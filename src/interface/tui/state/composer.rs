//! Composer state — manages the input buffer, queue, and Codex semantics.

use std::collections::VecDeque;

use ratatui_textarea::TextArea;

/// The composer manages user input and message queuing.
///
/// # Codex semantics
///
/// - Tab during agent running → queue the draft
/// - Tab when idle + non-bang → submit
/// - Tab when idle + bang (`!`) → no-op
/// - Esc with draft → clear
/// - Esc when empty → prime backtrack
/// - Esc+Esc when empty → load last user message
#[derive(Debug)]
pub(crate) struct Composer {
    /// The underlying text area (ratatui-textarea).
    pub(crate) textarea: TextArea<'static>,
    /// Queued drafts (waiting for agent to finish).
    pub(crate) queue: VecDeque<String>,
    /// Whether the current draft starts with `!` (bang-shell mode).
    pub(crate) bang_shell: bool,
    /// Whether backtrack prime is active (first Esc when empty).
    pub(crate) backtrack_primed: bool,
    /// Copy of the last submitted user message (for backtrack).
    last_user_message: Option<String>,
}

impl Composer {
    /// Create a new composer.
    pub(crate) fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message...");
        Self {
            textarea,
            queue: VecDeque::new(),
            bang_shell: false,
            backtrack_primed: false,
            last_user_message: None,
        }
    }

    /// Get the current draft text.
    pub(crate) fn draft(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Set the draft text (for backtrack / loading).
    pub(crate) fn set_draft(&mut self, text: impl Into<String>) {
        let t = text.into();
        self.textarea = TextArea::from(t.lines());
        self.textarea.set_placeholder_text("Type a message...");
        self.bang_shell = t.starts_with('!');
    }

    /// Clear the composer.
    pub(crate) fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.textarea.set_placeholder_text("Type a message...");
        self.bang_shell = false;
        self.backtrack_primed = false;
    }

    /// Submit the current draft — returns the text, queues it if agent is running.
    pub(crate) fn submit(&mut self, agent_running: bool) -> Option<String> {
        let draft = self.draft();
        if draft.trim().is_empty() {
            return None;
        }
        self.last_user_message = Some(draft.clone());
        self.clear();
        if agent_running {
            self.queue.push_back(draft);
            return None; // queued, not returned as submission
        }
        Some(draft)
    }

    /// Dequeue the next waiting message.
    pub(crate) fn dequeue(&mut self) -> Option<String> {
        self.queue.pop_front()
    }

    /// Handle Tab press. Returns `Some(text)` if the draft should be submitted.
    pub(crate) fn handle_tab(&mut self, agent_running: bool) -> Option<String> {
        if self.bang_shell && !agent_running {
            // bang + idle → no-op
            return None;
        }
        if agent_running {
            let draft = self.draft();
            if !draft.trim().is_empty() {
                self.queue.push_back(draft);
                self.clear();
            }
            return None;
        }
        // idle, non-bang → submit
        self.submit(false)
    }

    /// Handle Esc press.
    pub(crate) fn handle_esc(&mut self) -> EscOutcome {
        if !self.draft().is_empty() {
            self.clear();
            return EscOutcome::Cleared;
        }
        if self.backtrack_primed {
            self.backtrack_primed = false;
            if let Some(ref msg) = self.last_user_message {
                self.set_draft(msg.clone());
                return EscOutcome::BacktrackLoaded;
            }
        }
        self.backtrack_primed = true;
        EscOutcome::Primed
    }

    /// Queue count for display.
    pub(crate) fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Whether there are queued messages.
    pub(crate) fn has_queue(&self) -> bool {
        !self.queue.is_empty()
    }
}

/// Outcome of an Esc key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EscOutcome {
    /// Draft was cleared.
    Cleared,
    /// Backtrack was primed (first Esc on empty).
    Primed,
    /// Last user message was loaded into composer (second Esc).
    BacktrackLoaded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_composer_empty() {
        let c = Composer::new();
        assert!(c.draft().is_empty());
        assert!(!c.bang_shell);
        assert_eq!(c.queue_len(), 0);
    }

    #[test]
    fn test_submit_idle() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        let result = c.submit(false);
        assert_eq!(result, Some("hello".into()));
        assert!(c.draft().is_empty());
    }

    #[test]
    fn test_submit_running_queues() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        let result = c.submit(true);
        assert_eq!(result, None); // queued
        assert_eq!(c.queue_len(), 1);
        assert!(c.draft().is_empty());
    }

    #[test]
    fn test_dequeue() {
        let mut c = Composer::new();
        c.queue.push_back("first".into());
        c.queue.push_back("second".into());
        assert_eq!(c.dequeue(), Some("first".into()));
        assert_eq!(c.dequeue(), Some("second".into()));
        assert_eq!(c.dequeue(), None);
    }

    #[test]
    fn test_tab_running_queues() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        let result = c.handle_tab(true);
        assert_eq!(result, None);
        assert_eq!(c.queue_len(), 1);
        assert!(c.draft().is_empty());
    }

    #[test]
    fn test_tab_idle_submits() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        let result = c.handle_tab(false);
        assert_eq!(result, Some("hello".into()));
        assert!(c.draft().is_empty());
    }

    #[test]
    fn test_tab_idle_bang_noop() {
        let mut c = Composer::new();
        c.textarea.insert_str("!ls");
        c.bang_shell = true;
        let result = c.handle_tab(false);
        assert_eq!(result, None); // no-op
        assert!(!c.draft().is_empty()); // draft preserved
    }

    #[test]
    fn test_esc_clears_draft() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        assert_eq!(c.handle_esc(), EscOutcome::Cleared);
        assert!(c.draft().is_empty());
    }

    #[test]
    fn test_esc_empty_primes() {
        let mut c = Composer::new();
        assert_eq!(c.handle_esc(), EscOutcome::Primed);
        assert!(c.backtrack_primed);
    }

    #[test]
    fn test_esc_esc_backtrack() {
        let mut c = Composer::new();
        // First, submit a message so we have a last_user_message
        c.textarea.insert_str("previous message");
        c.submit(false);

        // First Esc primes
        assert_eq!(c.handle_esc(), EscOutcome::Primed);
        assert!(c.backtrack_primed);

        // Second Esc loads
        assert_eq!(c.handle_esc(), EscOutcome::BacktrackLoaded);
        assert_eq!(c.draft(), "previous message");
        assert!(!c.backtrack_primed);
    }

    #[test]
    fn test_clear_resets_state() {
        let mut c = Composer::new();
        c.textarea.insert_str("hello");
        c.backtrack_primed = true;
        c.bang_shell = true;
        c.clear();
        assert!(c.draft().is_empty());
        assert!(!c.bang_shell);
        assert!(!c.backtrack_primed);
    }

    #[test]
    fn test_empty_submit_returns_none() {
        let mut c = Composer::new();
        assert_eq!(c.submit(false), None);
    }
}
