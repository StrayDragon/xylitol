//! Agent event stream types.
//!
//! [`XyEventStream`] wraps the underlying stream with terminal-event
//! tracking so consumers poll until `AgentEnd`.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

use crate::protocol::lifecycle::XyEvent;

// ── XyEventStream ─────────────────────────────────────────────────

pub struct XyEventStream {
    pub(crate) inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
    pub(crate) done: bool,
}

impl XyEventStream {
    pub(crate) fn error(msg: impl Into<crate::protocol::lifecycle::XyEventError>) -> Self {
        let err = msg.into();
        let inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>> = Box::pin(async_stream::stream! {
            yield XyEvent::Error(err);
        });
        Self { inner, done: false }
    }
}

impl Stream for XyEventStream {
    type Item = XyEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(event)) => {
                if matches!(event, XyEvent::AgentEnd { .. }) {
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
