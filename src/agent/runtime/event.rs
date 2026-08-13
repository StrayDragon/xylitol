//! Agent event stream types.
//!
//! [`XyEventStream`] wraps the underlying stream with terminal-event
//! tracking so consumers poll until `AgentEnd`. Optional [`RunLease`]
//! finishes single-flight coordination on `AgentEnd` / drop.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

use crate::protocol::lifecycle::XyEvent;

use super::state::RunLease;

// ── XyEventStream ─────────────────────────────────────────────────

pub struct XyEventStream {
    pub(crate) inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
    pub(crate) done: bool,
    pub(crate) lease: Option<RunLease>,
}

impl XyEventStream {
    pub(crate) fn error(msg: impl Into<crate::protocol::lifecycle::XyEventError>) -> Self {
        let err = msg.into();
        let inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>> = Box::pin(async_stream::stream! {
            yield XyEvent::Error(err);
        });
        Self {
            inner,
            done: false,
            lease: None,
        }
    }

    pub(crate) fn busy() -> Self {
        Self::error(crate::protocol::lifecycle::XyEventError::new(
            crate::agent::runtime::state::RuntimeControlError::Busy.kind(),
            "agent run already active",
        ))
    }

    pub(crate) fn with_lease(
        inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
        lease: RunLease,
    ) -> Self {
        Self {
            inner,
            done: false,
            lease: Some(lease),
        }
    }

    fn finish_lease(&mut self) {
        if let Some(lease) = self.lease.as_mut() {
            lease.finish();
        }
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
                    self.finish_lease();
                }
                Poll::Ready(Some(event))
            }
            Poll::Ready(None) => {
                self.done = true;
                self.finish_lease();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for XyEventStream {
    fn drop(&mut self) {
        self.finish_lease();
    }
}
