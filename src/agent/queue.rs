//! Message queue management for steer/followUp/nextTurn.
//!
//! Aligns with pi's AgentSession steer/followUp/sendCustomMessage.

use std::collections::VecDeque;

/// Manages queued messages during streaming and idle periods.
#[derive(Debug, Clone, Default)]
pub struct MessageQueue {
    /// Steering messages — delivered after current tool execution round.
    steering: VecDeque<String>,
    /// Follow-up messages — delivered when agent has no pending operations.
    follow_up: VecDeque<String>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a steering message (interrupting).
    pub fn steer(&mut self, text: String) {
        self.steering.push_back(text);
    }

    /// Queue a follow-up message (non-interrupting).
    pub fn follow_up(&mut self, text: String) {
        self.follow_up.push_back(text);
    }

    /// Drain all steering messages and return them.
    pub fn drain_steering(&mut self) -> Vec<String> {
        self.steering.drain(..).collect()
    }

    /// Drain all follow-up messages and return them.
    pub fn drain_follow_up(&mut self) -> Vec<String> {
        self.follow_up.drain(..).collect()
    }

    /// Get read-only references to steering messages.
    pub fn get_steering(&self) -> &VecDeque<String> {
        &self.steering
    }

    /// Get read-only references to follow-up messages.
    pub fn get_followup(&self) -> &VecDeque<String> {
        &self.follow_up
    }

    /// Clear all queues and return contents.
    pub fn clear(&mut self) -> ClearResult {
        ClearResult {
            steering: self.steering.drain(..).collect(),
            follow_up: self.follow_up.drain(..).collect(),
        }
    }

    /// Total pending messages.
    pub fn pending_count(&self) -> usize {
        self.steering.len() + self.follow_up.len()
    }

    /// Whether either queue is non-empty.
    pub fn has_pending(&self) -> bool {
        !self.steering.is_empty() || !self.follow_up.is_empty()
    }
}

/// Result from clearing queues.
#[derive(Debug)]
pub struct ClearResult {
    pub steering: Vec<String>,
    pub follow_up: Vec<String>,
}
