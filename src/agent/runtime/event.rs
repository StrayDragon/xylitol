//! Agent event types — the output stream vocabulary of the ReAct loop.
//!
//! [`AgentEvent`] is emitted by [`AgentLoop`](super::react::AgentLoop) during a
//! turn; [`AgentEventStream`] wraps the underlying stream with terminal-event
//! tracking so consumers poll until `AgentEnd`.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use serde_json::Value;

use crate::domain::message::AgentMessage;

// ── AgentEvent ──────────────────────────────────────────────────────

/// Events emitted during agent execution.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Turn started.
    TurnStart { turn_index: u32 },
    /// Message started (a new message block).
    MessageStart { role: String },
    /// Streaming text delta from the LLM.
    TextDelta(String),
    /// Streaming thinking delta.
    ThinkingDelta(String),
    /// Message content updated.
    MessageUpdate {
        text: String,
        thinking: Option<String>,
    },
    /// Message completed.
    MessageEnd { role: String },
    /// Tool execution started.
    ToolExecutionStart {
        id: String,
        name: String,
        args: Value,
    },
    /// Tool execution update (streaming partial output).
    ToolExecutionUpdate { id: String, output: String },
    /// Tool execution completed.
    ToolExecutionEnd {
        id: String,
        name: String,
        result: String,
    },
    /// Turn ended.
    TurnEnd { turn_index: u32 },
    /// Error occurred.
    Error(String),
    /// Agent loop completed.
    AgentEnd { messages: Vec<AgentMessage> },
    /// Compaction started.
    CompactionStart { reason: String },
    /// Compaction ended.
    CompactionEnd {
        result: Option<String>,
        aborted: bool,
    },
    /// Model switched.
    ModelSelect { provider: String, model_id: String },
    /// Thinking level changed.
    ThinkingLevelChanged { level: String },
}

// ── AgentEventStream ────────────────────────────────────────────────

pub struct AgentEventStream {
    pub(crate) inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>>,
    pub(crate) done: bool,
    /// Track turn number (set externally via event wrapping).
    #[allow(dead_code)]
    pub(crate) turn_index: u32,
}

impl AgentEventStream {
    pub(crate) fn error(msg: String) -> Self {
        let inner: Pin<Box<dyn Stream<Item = AgentEvent> + Send>> =
            Box::pin(async_stream::stream! {
                yield AgentEvent::Error(msg);
            });
        Self {
            inner,
            done: false,
            turn_index: 0,
        }
    }
}

impl Stream for AgentEventStream {
    type Item = AgentEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                if matches!(event, AgentEvent::AgentEnd { .. }) {
                    self.done = true;
                }
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
