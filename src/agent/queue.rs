//! Message queue management for steer/followUp/nextTurn.
//!
//! Aligns with pi's AgentSession steer/followUp/sendCustomMessage.

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
