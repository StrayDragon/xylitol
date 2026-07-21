//! Steer / follow-up pending-message queues (c525).
//!
//! Short-lock [`VecDeque`] buffers + optional [`EventTx`] so enqueue/clear can
//! push [`XyEvent::QueueUpdate`] onto the **active** EventStream (not EventBus).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::protocol::lifecycle::XyEvent;
use crate::protocol::message::AgentMessage;

/// Drain policy for a pending-message queue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueMode {
    /// Drain every queued message in one call.
    #[default]
    All,
    /// Drain only the oldest message; leave the rest queued.
    OneAtATime,
}

/// Snapshot of queue depths for XyDriver / dispatch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueStats {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

/// Sender bound to the active `XyDriver::run` / ReAct EventStream (may be absent).
pub type EventTx = tokio::sync::mpsc::UnboundedSender<XyEvent>;

/// FIFO queue of [`AgentMessage`] with mode-aware drain ([`QueueChannel`]).
#[derive(Debug, Clone)]
pub struct PendingMessageQueue {
    messages: VecDeque<AgentMessage>,
    mode: QueueMode,
}

/// Alias matching c525 design naming.
pub type QueueChannel = PendingMessageQueue;

impl PendingMessageQueue {
    /// Create an empty queue with the given drain mode.
    pub fn new(mode: QueueMode) -> Self {
        Self {
            messages: VecDeque::new(),
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
        self.messages.push_back(message);
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
            QueueMode::All => self.messages.drain(..).collect(),
            QueueMode::OneAtATime => self.messages.pop_front().into_iter().collect(),
        }
    }

    /// Remove all messages without returning them.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

/// Steer + follow-up channels and the optional active-run event sender.
pub struct AsyncQueueRuntime {
    pub steer: Arc<Mutex<PendingMessageQueue>>,
    pub follow_up: Arc<Mutex<PendingMessageQueue>>,
    event_tx: Arc<Mutex<Option<EventTx>>>,
}

impl AsyncQueueRuntime {
    pub fn new(steering_mode: QueueMode, follow_up_mode: QueueMode) -> Self {
        Self {
            steer: Arc::new(Mutex::new(PendingMessageQueue::new(steering_mode))),
            follow_up: Arc::new(Mutex::new(PendingMessageQueue::new(follow_up_mode))),
            event_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn stats(&self) -> QueueStats {
        let steer_count = self.steer.lock().unwrap_or_else(|e| e.into_inner()).len();
        let follow_up_count = self
            .follow_up
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len();
        QueueStats {
            steer_count,
            follow_up_count,
        }
    }

    /// Bind the active run's EventStream sender (replaces any previous).
    pub fn bind_event_tx(&self, tx: EventTx) {
        *self.event_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
    }

    /// Clear the active-run sender (no-op enqueue notify after this).
    pub fn unbind_event_tx(&self) {
        *self.event_tx.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Notify the active EventStream with current depths (no-op if unbound).
    pub fn notify_queue_update(&self) {
        let stats = self.stats();
        let event = XyEvent::QueueUpdate {
            steer_count: stats.steer_count,
            follow_up_count: stats.follow_up_count,
        };
        let guard = self.event_tx.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = guard.as_ref() {
            let _ = tx.send(event);
        }
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

    #[test]
    fn concurrent_enqueue_via_runtime() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        rt.bind_event_tx(tx);
        {
            let mut q = rt.steer.lock().unwrap();
            q.enqueue(user("a"));
            q.enqueue(user("b"));
        }
        rt.notify_queue_update();
        let ev = rx.try_recv().expect("QueueUpdate");
        match ev {
            XyEvent::QueueUpdate {
                steer_count,
                follow_up_count,
            } => {
                assert_eq!(steer_count, 2);
                assert_eq!(follow_up_count, 0);
            }
            other => panic!("unexpected {other:?}"),
        }
        rt.unbind_event_tx();
        rt.notify_queue_update();
        assert!(rx.try_recv().is_err());
    }
}
