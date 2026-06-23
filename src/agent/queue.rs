//! Message queue management for steer/followUp/nextTurn.
//!
//! Supports steer (interrupting) and follow-up (non-interrupting) message queues.

#![allow(dead_code)]

use std::collections::VecDeque;

use crate::core::message::AgentMessage;

/// Manages queued messages during streaming and idle periods.
#[derive(Debug, Clone, Default)]
pub(crate) struct MessageQueue {
    /// Steering messages — delivered after current tool execution round.
    steering: VecDeque<AgentMessage>,
    /// Follow-up messages — delivered when agent has no pending operations.
    follow_up: VecDeque<AgentMessage>,
}

impl MessageQueue {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Push a message to the queue.
    pub(crate) fn push(&mut self, msg: AgentMessage) {
        self.follow_up.push_back(msg);
    }

    /// Queue a steering message (interrupting).
    pub(crate) fn steer(&mut self, text: AgentMessage) {
        self.steering.push_back(text);
    }

    /// Queue a follow-up message (non-interrupting).
    pub(crate) fn follow_up(&mut self, text: AgentMessage) {
        self.follow_up.push_back(text);
    }

    /// Drain all messages.
    pub(crate) fn drain(&mut self) -> Vec<AgentMessage> {
        self.follow_up.drain(..).collect()
    }

    /// Drain all steering messages and return them.
    pub(crate) fn drain_steering(&mut self) -> Vec<AgentMessage> {
        self.steering.drain(..).collect()
    }

    /// Drain all follow-up messages and return them.
    pub(crate) fn drain_follow_up(&mut self) -> Vec<AgentMessage> {
        self.follow_up.drain(..).collect()
    }

    /// Get read-only references to steering messages.
    pub(crate) fn get_steering(&self) -> &VecDeque<AgentMessage> {
        &self.steering
    }

    /// Get read-only references to follow-up messages.
    pub(crate) fn get_followup(&self) -> &VecDeque<AgentMessage> {
        &self.follow_up
    }

    /// Clear all queues and return contents.
    pub(crate) fn clear(&mut self) -> ClearResult {
        ClearResult {
            steering: self.steering.drain(..).collect(),
            follow_up: self.follow_up.drain(..).collect(),
        }
    }

    /// Total pending messages.
    pub(crate) fn pending_count(&self) -> usize {
        self.steering.len() + self.follow_up.len()
    }

    /// Whether either queue is non-empty.
    pub(crate) fn has_pending(&self) -> bool {
        !self.steering.is_empty() || !self.follow_up.is_empty()
    }
}

/// Result from clearing queues.
#[derive(Debug)]
pub(crate) struct ClearResult {
    pub(crate) steering: Vec<AgentMessage>,
    pub(crate) follow_up: Vec<AgentMessage>,
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
        assert!(q.drain_steering().is_empty());
        assert!(q.drain_follow_up().is_empty());
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
        assert!(q.drain_steering().is_empty());
        assert!(q.drain_follow_up().is_empty());
    }

    // ── Steering (interrupting) ─────────────────────────────────────

    #[test]
    fn steer_and_drain_steering() {
        let mut q = MessageQueue::new();
        q.steer(msg("interrupt"));
        q.push(msg("normal"));
        assert_eq!(q.pending_count(), 2);

        let steering = q.drain_steering();
        assert_eq!(steering.len(), 1);
        assert_eq!(steering[0].text(), "interrupt");

        // Follow-up still there
        let follow_up = q.drain_follow_up();
        assert_eq!(follow_up.len(), 1);
        assert_eq!(follow_up[0].text(), "normal");
    }

    #[test]
    fn steer_ordering() {
        let mut q = MessageQueue::new();
        q.steer(msg("steer-1"));
        q.steer(msg("steer-2"));
        q.follow_up(msg("follow-1"));

        // Steering comes first
        let steering = q.drain_steering();
        assert_eq!(steering.len(), 2);
        assert_eq!(steering[0].text(), "steer-1");
        assert_eq!(steering[1].text(), "steer-2");

        let follow_up = q.drain_follow_up();
        assert_eq!(follow_up.len(), 1);
        assert_eq!(follow_up[0].text(), "follow-1");
    }

    // ── Follow-up (non-interrupting) ────────────────────────────────

    #[test]
    fn follow_up_and_drain() {
        let mut q = MessageQueue::new();
        q.follow_up(msg("later"));
        assert_eq!(q.pending_count(), 1);

        let drained = q.drain_follow_up();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].text(), "later");
    }

    // ── Mixed steering + follow-up ──────────────────────────────────

    #[test]
    fn mixed_steer_and_followup() {
        let mut q = MessageQueue::new();
        q.steer(msg("urgent"));
        q.follow_up(msg("normal"));
        q.steer(msg("also-urgent"));

        assert_eq!(q.pending_count(), 3);
        assert!(q.has_pending());

        // drain_all should drain only follow-up by default
        let all = q.drain();
        assert_eq!(all.len(), 1); // only follow-up
        assert_eq!(all[0].text(), "normal");

        // Drain steering separately
        let steering = q.drain_steering();
        assert_eq!(steering.len(), 2);
    }

    // ── Clear ───────────────────────────────────────────────────────

    #[test]
    fn clear_returns_all_messages() {
        let mut q = MessageQueue::new();
        q.steer(msg("s1"));
        q.follow_up(msg("f1"));
        q.follow_up(msg("f2"));

        let result = q.clear();
        assert_eq!(result.steering.len(), 1);
        assert_eq!(result.steering[0].text(), "s1");
        assert_eq!(result.follow_up.len(), 2);
        assert_eq!(result.follow_up[1].text(), "f2");

        // After clear, empty
        assert!(!q.has_pending());
        assert_eq!(q.pending_count(), 0);
    }

    #[test]
    fn clear_empty_queue() {
        let mut q = MessageQueue::new();
        let result = q.clear();
        assert!(result.steering.is_empty());
        assert!(result.follow_up.is_empty());
    }

    // ── Pending count / has_pending ─────────────────────────────────

    #[test]
    fn pending_count_reflects_both_queues() {
        let mut q = MessageQueue::new();
        assert_eq!(q.pending_count(), 0);
        assert!(!q.has_pending());

        q.steer(msg("s"));
        assert_eq!(q.pending_count(), 1);
        assert!(q.has_pending());

        q.follow_up(msg("f"));
        assert_eq!(q.pending_count(), 2);
    }

    // ── Get read-only references ────────────────────────────────────

    #[test]
    fn get_steering_and_followup_views() {
        let mut q = MessageQueue::new();
        q.steer(msg("s"));
        q.follow_up(msg("f"));

        assert_eq!(q.get_steering().len(), 1);
        assert_eq!(q.get_steering()[0].text(), "s");
        assert_eq!(q.get_followup().len(), 1);
        assert_eq!(q.get_followup()[0].text(), "f");
    }

    // ── Drain idempotency ───────────────────────────────────────────

    #[test]
    fn drain_twice_returns_empty() {
        let mut q = MessageQueue::new();
        q.push(msg("once"));
        assert_eq!(q.drain().len(), 1);
        assert!(q.drain().is_empty());
    }

    #[test]
    fn drain_steering_twice_returns_empty() {
        let mut q = MessageQueue::new();
        q.steer(msg("once"));
        assert_eq!(q.drain_steering().len(), 1);
        assert!(q.drain_steering().is_empty());
    }
}
