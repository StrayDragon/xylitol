//! Recording event sink for agent unit tests.
//!
//! Implements [`XyEventSink`] by collecting all emitted [`XyEvent`]s
//! in a `Vec`, enabling assertions on the event sequence.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::protocol::lifecycle::XyEvent;
use crate::protocol::ports::XyEventSink;

/// Collects lifecycle events into a `Vec` for later assertion.
pub struct RecordingSink {
    events: Mutex<Vec<XyEvent>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Drain all recorded events.
    pub fn drain(&self) -> Vec<XyEvent> {
        self.events.lock().unwrap().drain(..).collect()
    }
}

impl Default for RecordingSink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl XyEventSink for RecordingSink {
    async fn emit(&self, event: &XyEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn records_and_drains_lifecycle_events() {
        let sink = RecordingSink::new();
        sink.emit(&XyEvent::TextDelta("hi".into())).await;
        sink.emit(&XyEvent::AgentEnd {
            messages: Vec::new(),
        })
        .await;
        let drained = sink.drain();
        assert_eq!(drained.len(), 2);
        assert!(matches!(drained[0], XyEvent::TextDelta(ref t) if t == "hi"));
        assert!(sink.drain().is_empty());
    }
}
