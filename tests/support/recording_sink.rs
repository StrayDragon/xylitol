//! Recording event sink for agent unit tests.
//!
//! Implements [`XyEventSink`] by collecting all emitted [`XyEvent`]s
//! in a `Vec`, enabling assertions on the event sequence.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::domain::lifecycle::XyEvent;
use crate::runtime_protocol::XyEventSink;

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

#[async_trait]
impl XyEventSink for RecordingSink {
    async fn emit(&self, event: &XyEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}
