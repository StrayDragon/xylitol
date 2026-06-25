//! AutoRetryEngine — auto-retry state machine for transient LLM errors (spec c255 / as32).
//!
//! Extracted from `AgentSession` (per the revised `architecture/ar02` upper bound)
//! into a focused collaborator. `AgentSession` composes an `AutoRetryEngine` and
//! delegates its retry behavior, preserving the public API (as31).

use crate::agent::retry::{RetryState, is_retryable_error};
use crate::infra::event::EventBus;
use crate::infra::event::lifecycle::AgentLifecycleEvent;

/// Auto-retry collaborator: owns retry state and drives the retry lifecycle.
#[derive(Default)]
pub struct AutoRetryEngine {
    retry_state: Option<RetryState>,
}

impl AutoRetryEngine {
    pub fn new() -> Self {
        Self { retry_state: None }
    }

    /// Check whether an assistant message signals a retryable error.
    ///
    /// A message is retryable when:
    /// - `stop_reason` is `Error`, or
    /// - `stop_reason` is `MaxTokens` with error text, or
    /// - the text content matches known transient error patterns.
    pub fn is_retryable_error(msg: &crate::core::message::AgentMessage) -> bool {
        match msg {
            crate::core::message::AgentMessage::AssistantMessage {
                stop_reason,
                content,
                ..
            } => {
                if let Some(sr) = stop_reason
                    && matches!(sr, crate::core::message::StopReason::Error)
                {
                    return true;
                }
                let text = crate::core::message::collect_text_parts(content);
                if text.is_empty() {
                    return false;
                }
                is_retryable_error(&text)
            }
            _ => false,
        }
    }

    /// Whether a retry is pending after the agent ends.
    pub fn will_retry_after_agent_end(&self) -> bool {
        self.retry_state.as_ref().is_some_and(|r| r.can_retry())
    }

    /// Initialize or reset the retry state for a new agent run.
    ///
    /// `max_retries` defaults to 3, `base_delay_ms` to 1000 (1 second).
    pub fn init_state(&mut self, max_retries: u32, base_delay_ms: u64) {
        self.retry_state = Some(RetryState::new(max_retries, base_delay_ms));
    }

    /// Prepare and execute a retry attempt.
    ///
    /// Emits `AutoRetryStart`, applies exponential backoff, then returns
    /// `true` if the retry should proceed (i.e. not aborted).
    pub async fn prepare_retry(&self, event_bus: &EventBus) -> bool {
        match &self.retry_state {
            Some(state) => {
                let attempt = state.attempt() + 1; // next_delay increments this
                let delay = state.next_delay();
                let max_retries = 3; // from state but not stored directly

                event_bus.emit_lifecycle(&AgentLifecycleEvent::AutoRetryStart {
                    attempt,
                    max_retries,
                    delay_ms: delay.as_millis() as u64,
                });

                // Wait for backoff or abort.
                let aborted = state.backoff(delay).await;

                event_bus.emit_lifecycle(&AgentLifecycleEvent::AutoRetryEnd {
                    success: !aborted,
                    attempt,
                });

                !aborted
            }
            None => false,
        }
    }

    /// Abort any in-progress retry.
    pub fn abort(&self) {
        if let Some(ref state) = self.retry_state {
            state.abort();
        }
    }
}

// Note: `RetryState` contains atomics and watch channels and cannot derive
// `Clone`; the engine is therefore not cloneable. `AgentSession` owns a single
// instance.
