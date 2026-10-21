use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::agent::runtime::state::RunId;
use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::message::AgentMessage;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueMode {
    All,
    #[default]
    OneAtATime,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QueueStats {
    pub steer_count: usize,
    pub follow_up_count: usize,
}

pub type EventTx = tokio::sync::mpsc::UnboundedSender<XyEvent>;

#[derive(Debug, Clone)]
pub struct PendingMessageQueue {
    messages: VecDeque<AgentMessage>,
    mode: QueueMode,
}

impl PendingMessageQueue {
    pub fn new(mode: QueueMode) -> Self {
        Self {
            messages: VecDeque::new(),
            mode,
        }
    }

    pub fn enqueue(&mut self, message: AgentMessage) {
        self.messages.push_back(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn drain(&mut self) -> Vec<AgentMessage> {
        match self.mode {
            QueueMode::All => self.messages.drain(..).collect(),
            QueueMode::OneAtATime => self.messages.pop_front().into_iter().collect(),
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

pub struct AsyncQueueRuntime {
    pub steer: Arc<Mutex<PendingMessageQueue>>,
    pub follow_up: Arc<Mutex<PendingMessageQueue>>,
    event_tx: Arc<Mutex<Option<(RunId, EventTx)>>>,
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
        let steer_count = crate::utils::lock_mutex(&self.steer).len();
        let follow_up_count = crate::utils::lock_mutex(&self.follow_up).len();
        QueueStats {
            steer_count,
            follow_up_count,
        }
    }

    pub fn bind_event_tx(&self, run_id: RunId, tx: EventTx) {
        *crate::utils::lock_mutex(&self.event_tx) = Some((run_id, tx));
    }

    pub fn unbind_event_tx(&self, run_id: RunId) {
        let mut guard = crate::utils::lock_mutex(&self.event_tx);
        if guard.as_ref().is_some_and(|(id, _)| *id == run_id) {
            *guard = None;
        }
    }

    pub fn notify_queue_update(&self) {
        let stats = self.stats();
        let event = XyEvent::QueueUpdate {
            steer_count: stats.steer_count,
            follow_up_count: stats.follow_up_count,
        };
        let guard = crate::utils::lock_mutex(&self.event_tx);
        if let Some((_, tx)) = guard.as_ref() {
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
    fn default_mode_is_one_at_a_time() {
        assert_eq!(QueueMode::default(), QueueMode::OneAtATime);
        let mut q = PendingMessageQueue::new(QueueMode::default());
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        assert_eq!(q.drain().len(), 1);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn all_mode_drains_everything() {
        let mut q = PendingMessageQueue::new(QueueMode::All);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(q.len(), 0);
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
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn clear_empties_queue() {
        let mut q = PendingMessageQueue::new(QueueMode::All);
        q.enqueue(user("x"));
        q.clear();
        assert_eq!(q.len(), 0);
        assert!(q.drain().is_empty());
    }

    #[test]
    fn concurrent_enqueue_via_runtime() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let run = RunId::from_raw_for_test(1);
        rt.bind_event_tx(run, tx);
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
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        let run2 = RunId::from_raw_for_test(2);
        rt.bind_event_tx(run2, tx2);
        rt.unbind_event_tx(run);
        rt.notify_queue_update();
        assert!(rx2.try_recv().is_ok());
        rt.unbind_event_tx(run2);
        rt.notify_queue_update();
        assert!(rx2.try_recv().is_err());
    }
}
