//! Auto-retry for transient LLM errors.
//!
//! Uses regex-based pattern matching for retryable error detection,
//! plus exponential backoff with cancellation support.

use regex::Regex;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::watch;

// ── Pattern matching ───────────────────────────────────────────────

static RETRYABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)overloaded|provider.?returned.?error|rate.?limit|too many requests|429|500|502|503|504|service.?unavailable|server.?error|internal.?error|network.?error|connection.?refused|connection.?lost|websocket.?closed|websocket.?error|fetch failed|upstream.?connect|reset before headers|socket hang up|ended without|stream ended before|timed? out|timeout|terminated|retry delay")
        .expect("static regex literal for retryable errors")
});

static NON_RETRYABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)usage.?limit|insufficient_quota|out of budget|quota exceeded|billing")
        .expect("static regex literal for non-retryable errors")
});

/// Check if an error message indicates a retryable transient error.
/// Returns false for context overflow (handled by compaction Case1).
pub(crate) fn is_retryable_error(error_msg: &str) -> bool {
    if crate::agent::compaction::error_message_is_context_overflow(error_msg) {
        return false;
    }
    if NON_RETRYABLE_RE.is_match(error_msg) {
        return false;
    }
    RETRYABLE_RE.is_match(error_msg)
}

// ── RetryState ─────────────────────────────────────────────────────

pub(crate) struct RetryState {
    max_retries: u32,
    base_delay_ms: u64,
    attempt: AtomicU32,
    #[allow(dead_code)]
    abort_tx: watch::Sender<bool>,
    abort_rx: watch::Receiver<bool>,
}

impl RetryState {
    pub(crate) fn new(max_retries: u32, base_delay_ms: u64) -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            max_retries,
            base_delay_ms,
            attempt: AtomicU32::new(0),
            abort_tx: tx,
            abort_rx: rx,
        }
    }

    pub(crate) fn can_retry(&self) -> bool {
        self.attempt.load(Ordering::Acquire) < self.max_retries
    }

    pub(crate) fn next_delay(&self) -> Duration {
        let attempt = self.attempt.fetch_add(1, Ordering::AcqRel) + 1;
        let delay_ms = self.base_delay_ms * 2u64.pow(attempt.saturating_sub(1));
        Duration::from_millis(delay_ms)
    }

    #[allow(dead_code)]
    pub(crate) fn attempt(&self) -> u32 {
        self.attempt.load(Ordering::Acquire)
    }

    /// Abort any in-progress backoff wait.
    #[allow(dead_code)]
    pub(crate) fn abort(&self) {
        let _ = self.abort_tx.send(true);
    }

    /// Wait for the backoff duration, or return immediately if aborted.
    pub async fn backoff(&self, delay: Duration) -> bool {
        let mut rx = self.abort_rx.clone();
        if *rx.borrow() {
            return true; // Already aborted
        }
        tokio::select! {
            _ = tokio::time::sleep(delay) => false,
            _ = rx.changed() => *rx.borrow(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_patterns() {
        assert!(is_retryable_error("rate limit exceeded"));
        assert!(is_retryable_error("HTTP 503 Service Unavailable"));
        assert!(is_retryable_error("The server returned error 500"));
        assert!(is_retryable_error("connection refused"));
        assert!(is_retryable_error("network error: timeout"));
        assert!(is_retryable_error("The request terminated unexpectedly"));
    }

    #[test]
    fn test_non_retryable_patterns() {
        assert!(!is_retryable_error("insufficient_quota"));
        assert!(!is_retryable_error("Monthly usage limit reached"));
        assert!(!is_retryable_error("billing error"));
        assert!(!is_retryable_error(
            "GoUsageLimitError: daily limit reached"
        ));
    }

    #[test]
    fn test_context_overflow_not_retryable() {
        assert!(!is_retryable_error(
            "Input length too long: exceeds 200000 token limit"
        ));
        assert!(!is_retryable_error(
            "prompt is too long: 213462 tokens > 200000 maximum"
        ));
        assert!(!is_retryable_error(
            "Your input exceeds the context window of this model"
        ));
    }

    #[tokio::test]
    async fn test_retry_state_exponential_backoff() {
        let state = RetryState::new(3, 1000);
        assert!(state.can_retry());

        let d1 = state.next_delay();
        assert_eq!(d1, Duration::from_millis(1000)); // attempt 1: base

        let d2 = state.next_delay();
        assert_eq!(d2, Duration::from_millis(2000)); // attempt 2: base*2

        let d3 = state.next_delay();
        assert_eq!(d3, Duration::from_millis(4000)); // attempt 3: base*4

        let d4 = state.next_delay();
        assert_eq!(d4, Duration::from_millis(8000)); // attempt 4: base*8
    }

    #[tokio::test]
    async fn test_retry_abort() {
        let state = RetryState::new(3, 1000);
        state.abort();
        // When aborted, backoff should return true (aborted)
        let aborted = state.backoff(Duration::from_millis(10)).await;
        assert!(aborted);
    }
}
