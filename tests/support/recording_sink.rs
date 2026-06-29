//! Recording event sink for agent unit tests.
//!
//! Implements [`EventSink`] by collecting all emitted [`LifecycleEvent`]s
//! in a `Vec`, enabling assertions on the event sequence.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::runtime_protocol::{EventSink, LifecycleEvent};

/// Collects lifecycle events into a `Vec` for later assertion.
pub struct RecordingSink {
    events: Mutex<Vec<LifecycleEvent>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Drain all recorded events.
    pub fn drain(&self) -> Vec<LifecycleEvent> {
        self.events.lock().unwrap().drain(..).collect()
    }
}

#[async_trait]
impl EventSink for RecordingSink {
    async fn emit(&self, event: &LifecycleEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}
