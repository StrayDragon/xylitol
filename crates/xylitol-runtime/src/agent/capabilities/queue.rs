use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use xylitol_plugin_sdk::lifecycle::XyEvent;
use xylitol_plugin_sdk::message::AgentMessage;
use xylitol_plugin_sdk::session::{QueueItem, QueueItemKind};

use crate::agent::runtime::state::TurnId;

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

#[derive(Debug, Clone)]
pub struct QueuedItem {
    pub id: String,
    pub message: AgentMessage,
}

impl QueuedItem {
    pub fn to_queue_item(&self, kind: QueueItemKind) -> QueueItem {
        QueueItem {
            id: self.id.clone(),
            kind,
            preview: QueueItem::preview_of(&self.message.text()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeOutcome {
    pub revoked: bool,
    pub items: Vec<QueueItem>,
}

pub type EventTx = tokio::sync::mpsc::UnboundedSender<XyEvent>;

#[derive(Debug, Clone)]
pub struct PendingMessageQueue {
    items: VecDeque<QueuedItem>,
    mode: QueueMode,
    kind: QueueItemKind,
    next_id: Arc<AtomicU64>,
}

impl PendingMessageQueue {
    pub fn new(mode: QueueMode, kind: QueueItemKind) -> Self {
        Self::with_id_source(mode, kind, Arc::new(AtomicU64::new(1)))
    }

    pub fn with_id_source(mode: QueueMode, kind: QueueItemKind, next_id: Arc<AtomicU64>) -> Self {
        Self {
            items: VecDeque::new(),
            mode,
            kind,
            next_id,
        }
    }

    fn allocate_id(&self) -> String {
        format!("q{}", self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    pub fn enqueue(&mut self, message: AgentMessage) -> QueuedItem {
        let item = QueuedItem {
            id: self.allocate_id(),
            message,
        };
        self.items.push_back(item.clone());
        item
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn drain(&mut self) -> Vec<AgentMessage> {
        self.drain_items()
            .into_iter()
            .map(|item| item.message)
            .collect()
    }

    pub fn drain_items(&mut self) -> Vec<QueuedItem> {
        match self.mode {
            QueueMode::All => self.items.drain(..).collect(),
            QueueMode::OneAtATime => self.items.pop_front().into_iter().collect(),
        }
    }

    pub fn pop_one(&mut self) -> Option<AgentMessage> {
        self.pop_item().map(|item| item.message)
    }

    pub fn pop_item(&mut self) -> Option<QueuedItem> {
        self.items.pop_front()
    }

    pub fn revoke(&mut self, id: &str) -> bool {
        match self.items.iter().position(|item| item.id == id) {
            Some(index) => {
                self.items.remove(index);
                true
            }
            None => false,
        }
    }

    pub fn items(&self) -> Vec<QueueItem> {
        self.items
            .iter()
            .map(|item| item.to_queue_item(self.kind))
            .collect()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

pub struct AsyncQueueRuntime {
    pub steer: Arc<Mutex<PendingMessageQueue>>,
    pub follow_up: Arc<Mutex<PendingMessageQueue>>,
    next_id: Arc<AtomicU64>,
    event_tx: Arc<Mutex<Option<(TurnId, EventTx)>>>,
}

impl AsyncQueueRuntime {
    pub fn new(steering_mode: QueueMode, follow_up_mode: QueueMode) -> Self {
        let next_id = Arc::new(AtomicU64::new(1));
        Self {
            steer: Arc::new(Mutex::new(PendingMessageQueue::with_id_source(
                steering_mode,
                QueueItemKind::Steer,
                next_id.clone(),
            ))),
            follow_up: Arc::new(Mutex::new(PendingMessageQueue::with_id_source(
                follow_up_mode,
                QueueItemKind::FollowUp,
                next_id.clone(),
            ))),
            next_id,
            event_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub fn allocate_id(&self) -> String {
        format!("q{}", self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    pub fn queue_item(&self, id: &str) -> Option<QueueItem> {
        self.items().into_iter().find(|item| item.id == id)
    }

    pub fn revoke(&self, id: &str) -> RevokeOutcome {
        let revoked = {
            let mut steer = crate::utils::lock_mutex(&self.steer);
            let mut follow_up = crate::utils::lock_mutex(&self.follow_up);
            if steer.revoke(id) {
                true
            } else {
                follow_up.revoke(id)
            }
        };
        RevokeOutcome {
            revoked,
            items: self.items(),
        }
    }

    pub fn items(&self) -> Vec<QueueItem> {
        let steer = crate::utils::lock_mutex(&self.steer);
        let follow_up = crate::utils::lock_mutex(&self.follow_up);
        let mut items: Vec<QueueItem> = steer.items();
        items.extend(follow_up.items());
        items
    }

    pub fn stats(&self) -> QueueStats {
        let steer_count = crate::utils::lock_mutex(&self.steer).len();
        let follow_up_count = crate::utils::lock_mutex(&self.follow_up).len();
        QueueStats {
            steer_count,
            follow_up_count,
        }
    }

    pub fn bind_event_tx(&self, turn_id: TurnId, tx: EventTx) {
        *crate::utils::lock_mutex(&self.event_tx) = Some((turn_id, tx));
    }

    pub fn unbind_event_tx(&self, turn_id: TurnId) {
        let mut guard = crate::utils::lock_mutex(&self.event_tx);
        if guard.as_ref().is_some_and(|(id, _)| *id == turn_id) {
            *guard = None;
        }
    }

    pub fn notify_queue_update(&self) {
        let event = XyEvent::QueueUpdate {
            items: self.items(),
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
        let mut q = PendingMessageQueue::new(QueueMode::default(), QueueItemKind::Steer);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        assert_eq!(q.drain().len(), 1);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn pop_one_takes_only_the_front_message_in_all_mode() {
        let mut q = PendingMessageQueue::new(QueueMode::All, QueueItemKind::Steer);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        assert_eq!(q.pop_one().map(|m| m.text()), Some("a".to_string()));
        assert_eq!(q.len(), 1, "pop_one must not drop the rest in All mode");
        assert_eq!(q.pop_one().map(|m| m.text()), Some("b".to_string()));
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn all_mode_drains_everything() {
        let mut q = PendingMessageQueue::new(QueueMode::All, QueueItemKind::Steer);
        q.enqueue(user("a"));
        q.enqueue(user("b"));
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn one_at_a_time_drains_single_message() {
        let mut q = PendingMessageQueue::new(QueueMode::OneAtATime, QueueItemKind::FollowUp);
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
        let mut q = PendingMessageQueue::new(QueueMode::All, QueueItemKind::Steer);
        q.enqueue(user("x"));
        q.clear();
        assert_eq!(q.len(), 0);
        assert!(q.drain().is_empty());
    }

    #[test]
    fn enqueue_assigns_stable_ids_across_both_queues() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let steer = rt.steer.lock().unwrap().enqueue(user("s"));
        let follow = rt.follow_up.lock().unwrap().enqueue(user("f"));
        assert_eq!(steer.id, "q1");
        assert_eq!(follow.id, "q2");
        assert_ne!(
            steer.id, follow.id,
            "queue item ids must be globally unique"
        );
    }

    #[test]
    fn revoke_removes_pending_item_only() {
        let rt = AsyncQueueRuntime::new(QueueMode::OneAtATime, QueueMode::OneAtATime);
        let first = rt.follow_up.lock().unwrap().enqueue(user("f1"));
        let second = rt.follow_up.lock().unwrap().enqueue(user("f2"));
        let outcome = rt.revoke(&first.id);
        assert!(outcome.revoked);
        assert_eq!(outcome.items.len(), 1);
        assert_eq!(outcome.items[0].id, second.id);
        assert_eq!(outcome.items[0].kind, QueueItemKind::FollowUp);
        let missing = rt.revoke(&first.id);
        assert!(
            !missing.revoked,
            "revoking a dequeued or unknown id must fail"
        );
    }

    #[test]
    fn revoke_after_pop_is_not_found() {
        let rt = AsyncQueueRuntime::new(QueueMode::OneAtATime, QueueMode::OneAtATime);
        let item = rt.follow_up.lock().unwrap().enqueue(user("f1"));
        let popped = rt.follow_up.lock().unwrap().pop_item();
        assert_eq!(popped.map(|i| i.id), Some(item.id.clone()));
        assert!(!rt.revoke(&item.id).revoked);
    }

    #[test]
    fn queue_items_carry_kind_and_truncated_preview() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let long = "x".repeat(200);
        rt.steer.lock().unwrap().enqueue(user(&long));
        rt.follow_up.lock().unwrap().enqueue(user("short"));
        let items = rt.items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].kind, QueueItemKind::Steer);
        assert_eq!(items[0].preview.chars().count(), 80);
        assert_eq!(items[1].kind, QueueItemKind::FollowUp);
        assert_eq!(items[1].preview, "short");
        assert_eq!(rt.stats().steer_count, 1);
        assert_eq!(rt.stats().follow_up_count, 1);
    }

    #[test]
    fn concurrent_revoke_succeeds_exactly_once() {
        let rt = std::sync::Arc::new(AsyncQueueRuntime::new(
            QueueMode::OneAtATime,
            QueueMode::OneAtATime,
        ));
        let item = rt.follow_up.lock().unwrap().enqueue(user("f1"));
        let attempts = 8;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(attempts));
        let mut handles = Vec::new();
        for _ in 0..attempts {
            let rt = std::sync::Arc::clone(&rt);
            let barrier = std::sync::Arc::clone(&barrier);
            let id = item.id.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                rt.revoke(&id).revoked
            }));
        }
        let revoked = handles
            .into_iter()
            .map(|h| h.join().expect("revoke thread"))
            .filter(|ok| *ok)
            .count();
        assert_eq!(
            revoked, 1,
            "concurrent revoke of one id must succeed exactly once"
        );
        assert!(
            rt.items().is_empty(),
            "the revoked item must leave the queue"
        );
    }

    #[test]
    fn revoke_is_identical_for_steer_and_follow_up_queues() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let steer = rt.steer.lock().unwrap().enqueue(user("s"));
        let follow = rt.follow_up.lock().unwrap().enqueue(user("f"));
        assert!(rt.revoke(&steer.id).revoked, "steer items are revocable");
        assert!(
            rt.revoke(&follow.id).revoked,
            "follow_up items are revocable"
        );
        assert_eq!(
            rt.items(),
            Vec::new(),
            "both queues must expose the same revoke semantics"
        );
    }

    #[test]
    fn concurrent_enqueue_via_runtime() {
        let rt = AsyncQueueRuntime::new(QueueMode::All, QueueMode::All);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let run = TurnId::from_raw_for_test(1);
        rt.bind_event_tx(run, tx);
        {
            let mut q = rt.steer.lock().unwrap();
            q.enqueue(user("a"));
            q.enqueue(user("b"));
        }
        rt.notify_queue_update();
        let ev = rx.try_recv().expect("QueueUpdate");
        match ev {
            XyEvent::QueueUpdate { items } => {
                assert_eq!(items.len(), 2);
                assert!(items.iter().all(|i| i.kind == QueueItemKind::Steer));
            }
            other => panic!("unexpected {other:?}"),
        }
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();
        let run2 = TurnId::from_raw_for_test(2);
        rt.bind_event_tx(run2, tx2);
        rt.unbind_event_tx(run);
        rt.notify_queue_update();
        assert!(rx2.try_recv().is_ok());
        rt.unbind_event_tx(run2);
        rt.notify_queue_update();
        assert!(rx2.try_recv().is_err());
    }
}
