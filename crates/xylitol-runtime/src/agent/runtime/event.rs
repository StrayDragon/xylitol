use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

use xylitol_plugin_sdk::lifecycle::XyEvent;

use super::state::TurnLease;

pub struct XyEventStream {
    pub(crate) inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
    pub(crate) done: bool,
    pub(crate) lease: Option<TurnLease>,
}

impl XyEventStream {
    pub(crate) fn error(msg: impl Into<xylitol_plugin_sdk::lifecycle::XyEventError>) -> Self {
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

    pub(crate) fn empty() -> Self {
        Self {
            inner: Box::pin(futures::stream::empty()),
            done: false,
            lease: None,
        }
    }

    pub(crate) fn busy() -> Self {
        Self::error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
            crate::agent::runtime::state::RuntimeControlError::Busy.kind(),
            "agent run already active",
        ))
    }

    pub(crate) fn with_lease(
        inner: Pin<Box<dyn Stream<Item = XyEvent> + Send>>,
        lease: TurnLease,
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
            Poll::Ready(Some(event)) => Poll::Ready(Some(event)),
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
        if let Some(lease) = self.lease.as_mut() {
            lease.cancel();
        }
    }
}

#[cfg(test)]
mod drop_lease_tests {
    use super::*;
    use crate::agent::capabilities::{AsyncQueueRuntime, QueueMode};
    use crate::agent::runtime::state::{SharedTurnCoordinator, TurnLease};

    fn lease_with_queues() -> (
        SharedTurnCoordinator,
        std::sync::Arc<AsyncQueueRuntime>,
        TurnLease,
    ) {
        let coordinator = SharedTurnCoordinator::new();
        let queues = std::sync::Arc::new(AsyncQueueRuntime::new(QueueMode::All, QueueMode::All));
        let turn_id = coordinator.with_mut(|c| c.try_begin_immediate().expect("idle").0);
        let lease = TurnLease::new(coordinator.clone(), turn_id, Some(queues.clone()));
        (coordinator, queues, lease)
    }

    #[test]
    fn dropping_a_live_stream_cancels_the_turn() {
        let (coordinator, _queues, lease) = lease_with_queues();
        let stream = XyEventStream::with_lease(
            Box::pin(async_stream::stream! {
                futures::pending!();
                yield XyEvent::Error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
                    "unreachable",
                    "stream never yields",
                ));
            }),
            lease,
        );
        assert!(coordinator.with(|c| c.has_work()));
        drop(stream);
        assert!(
            !coordinator.with(|c| c.has_work()),
            "mid-drop must cancel the in-flight turn"
        );
    }

    #[test]
    fn dropping_a_live_stream_clears_steer_but_keeps_follow_up() {
        let (_coordinator, queues, lease) = lease_with_queues();
        let stream = XyEventStream::with_lease(
            Box::pin(async_stream::stream! {
                futures::pending!();
                yield XyEvent::Error(xylitol_plugin_sdk::lifecycle::XyEventError::new(
                    "unreachable",
                    "stream never yields",
                ));
            }),
            lease,
        );
        crate::utils::lock_mutex(&queues.steer)
            .enqueue(xylitol_plugin_sdk::message::AgentMessage::user("steer"));
        crate::utils::lock_mutex(&queues.follow_up)
            .enqueue(xylitol_plugin_sdk::message::AgentMessage::user("follow"));
        drop(stream);
        assert!(
            crate::utils::lock_mutex(&queues.steer).is_empty(),
            "steer queue must be cleared on mid-drop"
        );
        assert_eq!(
            crate::utils::lock_mutex(&queues.follow_up).len(),
            1,
            "follow_up must survive mid-drop"
        );
    }
}
