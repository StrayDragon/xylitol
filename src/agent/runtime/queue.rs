//! Message queue management for queued user messages (steer / follow-up).
//!
//! Both steer and follow-up messages share a single FIFO queue; the historical
//! dual-queue distinction was never exercised (no interrupting delivery path
//! exists), so the queue collapsed to one buffer.

use std::collections::VecDeque;

use crate::core::message::AgentMessage;

/// Manages queued messages during streaming and idle periods.
#[derive(Debug, Clone, Default)]
pub(crate) struct MessageQueue {
    messages: VecDeque<AgentMessage>,
}

impl MessageQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Push a message to the queue.
    pub(crate) fn push(&mut self, msg: AgentMessage) {
        self.messages.push_back(msg);
    }

    /// Drain all messages.
    pub(crate) fn drain(&mut self) -> Vec<AgentMessage> {
        self.messages.drain(..).collect()
    }

    /// Total pending messages.
    pub(crate) fn pending_count(&self) -> usize {
        self.messages.len()
    }

    /// Whether the queue is non-empty.
    pub(crate) fn has_pending(&self) -> bool {
        !self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(text: &str) -> AgentMessage {
        AgentMessage::user(text)
    }

    // ── Empty queue ─────────────────────────────────────────────────

    #[test]
    fn new_queue_is_empty() {
        let mut q = MessageQueue::new();
        assert!(!q.has_pending());
        assert_eq!(q.pending_count(), 0);
        assert!(q.drain().is_empty());
    }

    // ── Push / drain ────────────────────────────────────────────────

    #[test]
    fn push_and_drain() {
        let mut q = MessageQueue::new();
        q.push(msg("hi"));
        q.push(msg("there"));
        assert!(q.has_pending());
        assert_eq!(q.pending_count(), 2);

        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].text(), "hi");
        assert_eq!(drained[1].text(), "there");

        // After drain, queue is empty
        assert_eq!(q.pending_count(), 0);
    }

    #[test]
    fn drain_empty_returns_empty_vec() {
        let mut q = MessageQueue::new();
        assert!(q.drain().is_empty());
    }

    #[test]
    fn drain_twice_returns_empty() {
        let mut q = MessageQueue::new();
        q.push(msg("once"));
        assert_eq!(q.drain().len(), 1);
        assert!(q.drain().is_empty());
    }
}
