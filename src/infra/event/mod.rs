//! Channel-based event bus — aligns with pi's EventBus (event-bus.ts).
//!
//! Supports:
//! - `emit(channel, data)` — string channel + JSON Value payload
//! - `on(channel, handler)` → `UnsubscribeHandle` (drop to unsubscribe)
//! - `clear()` — remove all listeners
//! - Error isolation — panicking handlers don't break the bus (tokio task boundary)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

/// Type alias for async event handlers.
pub type Handler = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

/// A handle that unsubscribes when dropped.
pub struct UnsubscribeHandle {
    bus: EventBus,
    channel: String,
    id: u64,
}

impl Drop for UnsubscribeHandle {
    fn drop(&mut self) {
        self.bus.remove_listener(&self.channel, self.id);
    }
}

#[derive(Clone)]
struct ListenerEntry {
    id: u64,
    handler: Handler,
}

/// Channel-based event bus with string channels and JSON Value payloads.
#[derive(Clone, Default)]
pub struct EventBus {
    listeners: Arc<Mutex<HashMap<String, Vec<ListenerEntry>>>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Emit data to all subscribers of a channel.
    /// Each handler runs in its own tokio task; panics in one handler
    /// do not prevent other handlers from receiving events.
    pub fn emit(&self, channel: &str, data: Value) {
        let entries = {
            let guard = self.listeners.lock().expect("EventBus lock poisoned");
            guard.get(channel).cloned().unwrap_or_default()
        };

        for entry in &entries {
            let handler = entry.handler.clone();
            let data = data.clone();
            let channel_name = channel.to_string();
            // tokio::spawn isolates panics at task boundary
            tokio::task::spawn(async move {
                let join_handle = tokio::task::spawn(async move {
                    handler(data).await;
                });
                if let Err(e) = join_handle.await {
                    tracing::warn!(
                        "EventBus: handler panicked on channel '{}': {}",
                        channel_name,
                        e
                    );
                }
            });
        }
    }

    /// Subscribe to a channel. Returns an UnsubscribeHandle.
    pub fn on<F, Fut>(&self, channel: &str, handler: F) -> UnsubscribeHandle
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let wrapped: Handler = Arc::new(move |data: Value| {
            let fut = handler(data);
            Box::pin(fut)
        });

        self.listeners
            .lock()
            .expect("EventBus lock poisoned")
            .entry(channel.to_string())
            .or_default()
            .push(ListenerEntry {
                id,
                handler: wrapped,
            });

        UnsubscribeHandle {
            bus: self.clone(),
            channel: channel.to_string(),
            id,
        }
    }

    /// Remove a specific listener by channel and id.
    fn remove_listener(&self, channel: &str, id: u64) {
        if let Ok(mut guard) = self.listeners.lock()
            && let Some(list) = guard.get_mut(channel)
        {
            list.retain(|e| e.id != id);
        }
    }

    /// Remove all listeners from all channels.
    pub fn clear(&self) {
        self.listeners
            .lock()
            .expect("EventBus lock poisoned")
            .clear();
    }
}

// ── Standard channel names (align with pi) ────────────────────────

/// Standard event channel names used across the system.
#[allow(dead_code)]
pub mod channels {
    pub const TOOL_EXECUTION_START: &str = "tool_execution_start";
    pub const TOOL_EXECUTION_END: &str = "tool_execution_end";
    pub const TOOL_EXECUTION_UPDATE: &str = "tool_execution_update";
    pub const TURN_START: &str = "turn_start";
    pub const TURN_END: &str = "turn_end";
    pub const MESSAGE_START: &str = "message_start";
    pub const MESSAGE_END: &str = "message_end";
    pub const MESSAGE_UPDATE: &str = "message_update";
    pub const COMPACTION_START: &str = "compaction_start";
    pub const COMPACTION_END: &str = "compaction_end";
    pub const SETTINGS_CHANGED: &str = "settings:changed";
    pub const AGENT_START: &str = "agent_start";
    pub const AGENT_END: &str = "agent_end";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_and_receive() {
        let bus = EventBus::new();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let r = received.clone();

        let _handle = bus.on("test", move |data| {
            r.lock().unwrap().push(data);
            async {}
        });

        bus.emit("test", Value::String("hello".into()));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], Value::String("hello".into()));
    }

    #[tokio::test]
    async fn test_unsubscribe_on_drop() {
        let bus = EventBus::new();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let r = received.clone();

        let handle = bus.on("test", move |data| {
            r.lock().unwrap().push(data);
            async {}
        });
        drop(handle);

        bus.emit("test", Value::String("hello".into()));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(received.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_clear() {
        let bus = EventBus::new();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let r = received.clone();

        let _h1 = bus.on("a", {
            let r = r.clone();
            move |data| {
                r.lock().unwrap().push(data);
                async {}
            }
        });
        let _h2 = bus.on("b", {
            let r = r.clone();
            move |data| {
                r.lock().unwrap().push(data);
                async {}
            }
        });

        bus.clear();
        bus.emit("a", Value::String("x".into()));
        bus.emit("b", Value::String("y".into()));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(received.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_handler_panic_doesnt_break_other_handlers() {
        let bus = EventBus::new();
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let r = received.clone();

        // Handler A panics
        let _h1 = bus.on("test", move |_data| async {
            panic!("handler A panic");
        });
        // Handler B is normal
        let _h2 = bus.on("test", move |data| {
            r.lock().unwrap().push(data);
            async {}
        });

        bus.emit("test", Value::String("should reach B".into()));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], Value::String("should reach B".into()));
    }
}
