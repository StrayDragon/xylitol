use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use serde_json::Value;
use xylitol_plugin_sdk::lifecycle::XyEvent;

fn panic_payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic with a non-string payload".to_string()
    }
}

pub type Handler = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
>;

#[derive(Clone, Default)]
pub struct EventBus {
    listeners: Arc<Mutex<HashMap<String, Vec<ListenerEntry>>>>,
}

#[derive(Clone)]
struct ListenerEntry {
    handler: Handler,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit(&self, channel: &str, data: Value) {
        let entries = {
            let guard = self.listeners.lock().expect("EventBus lock poisoned");
            guard.get(channel).cloned().unwrap_or_default()
        };

        for entry in &entries {
            let handler = entry.handler.clone();
            let data = data.clone();
            let channel_name = channel.to_string();
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

    pub fn emit_lifecycle(&self, event: &XyEvent) {
        let channel = format!("lifecycle:{}", event.description());
        let data = serde_json::to_value(event).unwrap_or_default();
        self.emit(&channel, data.clone());
        self.emit("lifecycle:*", data);
    }
}

#[async_trait::async_trait]
impl xylitol_plugin_sdk::XyEventSink for EventBus {
    async fn emit(&self, event: &XyEvent) {
        self.emit_lifecycle(event);
    }
}
