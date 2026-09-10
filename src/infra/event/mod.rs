//! Channel-based event bus for side-channel lifecycle (compaction, settings).
//!
//! Product code consumes lifecycle events exclusively through the
//! [`XyEventSink`](crate::protocol::ports::XyEventSink) port (c2715) —
//! no product path subscribes by channel name (the test-only string-channel
//! subscription surface was purged in c2750).

pub mod lifecycle;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use serde_json::Value;

use self::lifecycle::XyEvent;

/// Extract the message from a caught panic payload (mirrors `JoinError` display).
fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic with a non-string payload".to_string()
    }
}

/// Type alias for async event handlers.
pub type Handler = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

/// Channel-based event bus with string channels and JSON Value payloads.
#[derive(Clone, Default)]
pub(crate) struct EventBus {
    listeners: Arc<Mutex<HashMap<String, Vec<ListenerEntry>>>>,
}

#[derive(Clone)]
struct ListenerEntry {
    handler: Handler,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self::default()
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
            // One task per handler; catch_unwind turns a handler panic into a log
            // line instead of poisoning the bus (no wrapper task needed).
            tokio::task::spawn(async move {
                let fut = handler(data);
                if let Err(payload) = std::panic::AssertUnwindSafe(fut).catch_unwind().await {
                    log::warn!(
                        "EventBus: handler panicked on channel '{}': {}",
                        channel_name,
                        panic_payload_message(&payload)
                    );
                }
            });
        }
    }

    /// Emit a typed lifecycle event.
    ///
    /// The event is serialized to JSON and dispatched on channel
    /// `lifecycle:<description>` (e.g. `lifecycle:turn_start`).
    /// All subscribers of `lifecycle:*` also receive it.
    pub fn emit_lifecycle(&self, event: &XyEvent) {
        let channel = format!("lifecycle:{}", event.description());
        let data = serde_json::to_value(event).unwrap_or_default();
        self.emit(&channel, data.clone());
        // Also emit on the wildcard channel so `on_lifecycle` subscribers
        // receive all lifecycle events.
        self.emit("lifecycle:*", data);
    }
}

#[async_trait::async_trait]
impl crate::protocol::ports::XyEventSink for EventBus {
    async fn emit(&self, event: &XyEvent) {
        self.emit_lifecycle(event);
    }
}
