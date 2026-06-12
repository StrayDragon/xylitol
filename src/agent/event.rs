//! Agent event bus — multi-listener subscription model.
//!
//! Aligns with pi's AgentSession subscribe/unsubscribe pattern.
//! Uses tokio::sync::broadcast for multi-consumer event delivery.
//! Drop-based unsubscribe via UnsubscribeHandle.

#![allow(dead_code)]
#[allow(dead_code)]
use tokio::sync::broadcast;

use crate::agent::r#loop::AgentEvent;

/// A bus for publishing AgentEvents to multiple subscribers.
pub(crate) struct AgentEventBus {
    sender: broadcast::Sender<AgentEvent>,
}

impl Clone for AgentEventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl AgentEventBus {
    /// Create a new event bus with the given buffer capacity.
    pub(crate) fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit an event to all active subscribers.
    /// Ignores errors when no receivers exist.
    pub(crate) fn emit(&self, event: AgentEvent) {
        let _ = self.sender.send(event);
    }

    /// Subscribe to events.
    /// Returns a handle that unsubscribes when dropped.
    pub(crate) fn subscribe(&self) -> UnsubscribeHandle {
        UnsubscribeHandle {
            receiver: self.sender.subscribe(),
        }
    }

    /// Get the number of active subscribers.
    pub(crate) fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// An unsubscribe handle that receives events from the bus.
/// When dropped, the subscriber is automatically removed.
pub(crate) struct UnsubscribeHandle {
    receiver: broadcast::Receiver<AgentEvent>,
}

impl UnsubscribeHandle {
    /// Receive the next event (async).
    /// Returns None when the sender is dropped.
    pub async fn recv(&mut self) -> Option<AgentEvent> {
        self.receiver.recv().await.ok()
    }

    /// Try to receive an event without blocking.
    pub(crate) fn try_recv(&mut self) -> Option<AgentEvent> {
        self.receiver.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_subscriber() {
        let bus = AgentEventBus::new(32);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        bus.emit(AgentEvent::TurnStart { turn_index: 0 });
        bus.emit(AgentEvent::TurnEnd { turn_index: 0 });

        // Both subscribers receive the events
        let ev1 = sub1.recv().await.unwrap();
        let ev2 = sub2.recv().await.unwrap();
        assert!(matches!(ev1, AgentEvent::TurnStart { .. }));
        assert!(matches!(ev2, AgentEvent::TurnStart { .. }));

        let ev1 = sub1.recv().await.unwrap();
        let ev2 = sub2.recv().await.unwrap();
        assert!(matches!(ev1, AgentEvent::TurnEnd { .. }));
        assert!(matches!(ev2, AgentEvent::TurnEnd { .. }));
    }

    #[tokio::test]
    async fn test_unsubscribe_on_drop() {
        let bus = AgentEventBus::new(16);
        let mut sub = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        bus.emit(AgentEvent::TurnStart { turn_index: 0 });
        let _ = sub.recv().await;

        drop(sub);
        bus.emit(AgentEvent::TurnStart { turn_index: 1 });
        // Should not block/payload
    }
}
