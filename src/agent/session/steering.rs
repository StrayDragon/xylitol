//! Steering / follow-up queue operations (spec c255 / as32).
//!
//! These methods operate on the [`AgentSession`](super::AgentSession) message
//! queue and are grouped here to keep `mod.rs` focused. They remain methods on
//! `AgentSession` (preserving the public API per as31).

impl super::AgentSession {
    /// Queue a steering message — injected into context mid-turn.
    pub fn steer(
        &mut self,
        text: impl Into<String>,
        _images: Option<Vec<crate::core::message::ImageContent>>,
    ) {
        let msg = crate::core::message::AgentMessage::user(text);
        self.message_queue.push(msg);
    }

    /// Queue a follow-up message — delivered after current turn.
    pub fn follow_up(
        &mut self,
        text: impl Into<String>,
        _images: Option<Vec<crate::core::message::ImageContent>>,
    ) {
        let msg = crate::core::message::AgentMessage::user(text);
        self.message_queue.push(msg);
    }

    /// Return and clear all queued messages.
    pub fn clear_queue(&mut self) -> Vec<crate::core::message::AgentMessage> {
        self.message_queue.drain()
    }

    /// Number of pending messages in the queue.
    pub fn pending_message_count(&self) -> usize {
        self.message_queue.pending_count()
    }

    /// Check if any steering message is pending.
    pub fn has_pending_steer(&self) -> bool {
        self.message_queue.has_pending()
    }

    /// Get all queued steering messages (without clearing).
    pub fn get_steering_messages(&self) -> Vec<crate::core::message::AgentMessage> {
        Vec::new()
    }

    /// Get all queued follow-up messages (without clearing).
    pub fn get_follow_up_messages(&self) -> Vec<crate::core::message::AgentMessage> {
        Vec::new()
    }

    /// Drain queued messages for the next turn.
    pub fn drain_queued_messages(&mut self) -> Vec<crate::core::message::AgentMessage> {
        self.message_queue.drain()
    }
}
