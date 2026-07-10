//! Pending message queues for steer / follow-up injection during a ReAct run.
//!
//! Aligns with pi's `PendingMessageQueue`: enqueue concurrent user text, then
//! `drain` according to [`QueueMode`] at ReAct iteration boundaries.

use crate::domain::message::AgentMessage;

/// Drain policy for a pending-message queue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueMode {
    /// Drain every queued message in one call.
    #[default]
    All,
    /// Drain only the oldest message; leave the rest queued.
    OneAtATime,
}

/// FIFO queue of [`AgentMessage`] with mode-aware drain.
#[derive(Debug, Clone)]
pub struct PendingMessageQueue {
    messages: Vec<AgentMessage>,
    mode: QueueMode,
}

impl PendingMessageQueue {
    /// Create an empty queue with the given drain mode.
    pub fn new(mode: QueueMode) -> Self {
        Self {
            messages: Vec::new(),
            mode,
        }
    }

    /// Current drain mode.
    pub fn mode(&self) -> QueueMode {
        self.mode
    }

    /// Replace the drain mode (does not affect already-queued messages).
    pub fn set_mode(&mut self, mode: QueueMode) {
        self.mode = mode;
    }

    /// Append a message to the end of the queue.
    pub fn enqueue(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }

    /// Whether the queue holds at least one message.
    pub fn has_items(&self) -> bool {
        !self.messages.is_empty()
    }

    /// Number of queued messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Drain according to [`QueueMode`]: all messages, or only the first.
    pub fn drain(&mut self) -> Vec<AgentMessage> {
        match self.mode {
            QueueMode::All => std::mem::take(&mut self.messages),
            QueueMode::OneAtATime => {
                if self.messages.is_empty() {
                    Vec::new()
                } else {
                    vec![self.messages.remove(0)]
                }
            }
        }
    }

    /// Remove all messages without returning them.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> AgentMessage {
        AgentMessage::user(text)
    }

    #[test]
    fn all_mode_drains_everything() {
        let mut q = PendingMessageQueue::new(QueueMode::All);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert!(q.is_empty());
    }

    #[test]
    fn one_at_a_time_drains_single_message() {
        let mut q = PendingMessageQueue::new(QueueMode::OneAtATime);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        let drained = q.drain();
        assert_eq!(drained.len(), 1);
        assert_eq!(q.len(), 1);
        let rest = q.drain();
        assert_eq!(rest.len(), 1);
        assert!(q.is_empty());
    }

    #[test]
    fn clear_empties_queue() {
        let mut q = PendingMessageQueue::new(QueueMode::All);
        q.enqueue(user("x"));
        q.clear();
        assert!(!q.has_items());
        assert!(q.drain().is_empty());
    }
}
