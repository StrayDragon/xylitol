use regex::Regex;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use xylitol_plugin_sdk::error::XyError;

static RETRYABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)overloaded|provider.?returned.?error|rate.?limit|too many requests|429|500|502|503|504|service.?unavailable|server.?error|internal.?error|network.?error|connection.?refused|connection.?lost|websocket.?closed|websocket.?error|fetch failed|upstream.?connect|reset before headers|socket hang up|ended without|stream ended before|timed? out|timeout|terminated|retry delay")
        .expect("static regex literal for retryable errors")
});

static NON_RETRYABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)usage.?limit|insufficient_quota|out of budget|quota exceeded|billing")
        .expect("static regex literal for non-retryable errors")
});

pub(crate) fn is_retryable_error(error_msg: &str) -> bool {
    if crate::agent::compaction::error_message_is_context_overflow(error_msg) {
        return false;
    }
    if NON_RETRYABLE_RE.is_match(error_msg) {
        return false;
    }
    RETRYABLE_RE.is_match(error_msg)
}

pub(crate) fn is_retryable_xy_error(err: &XyError) -> bool {
    if let Some(detail) = err.provider_detail() {
        if let Some(status) = detail.status {
            if status == 429 || (500..600).contains(&status) {
                return true;
            }
            if (400..500).contains(&status) {
                return false;
            }
        }
        for value in [detail.code.as_deref(), detail.error_type.as_deref()]
            .into_iter()
            .flatten()
        {
            let lower = value.to_ascii_lowercase();
            if lower.contains("insufficient_quota") || lower.contains("billing") {
                return false;
            }
            if lower.contains("rate_limit")
                || lower.contains("server_error")
                || lower.contains("overloaded")
                || lower.contains("api_error")
                || lower.contains("timeout")
                || lower.contains("internal_error")
            {
                return true;
            }
        }
    }
    is_retryable_error(&err.to_string())
}

pub(crate) struct RetryState {
    max_retries: u32,
    base_delay_ms: u64,
    attempt: AtomicU32,
}

impl RetryState {
    pub(crate) fn new(max_retries: u32, base_delay_ms: u64) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            attempt: AtomicU32::new(0),
        }
    }

    pub(crate) fn can_retry(&self) -> bool {
        self.attempt.load(Ordering::Acquire) < self.max_retries
    }

    pub(crate) fn attempt(&self) -> u32 {
        self.attempt.load(Ordering::Acquire)
    }

    pub(crate) fn max_retries(&self) -> u32 {
        self.max_retries
    }

    pub(crate) fn next_delay(&self) -> Duration {
        let attempt = self.attempt.fetch_add(1, Ordering::AcqRel) + 1;
        let delay_ms = self.base_delay_ms * 2u64.pow(attempt.saturating_sub(1));
        Duration::from_millis(delay_ms)
    }

    pub(crate) fn next_delay_with_hint(&self, hint: Option<Duration>) -> Duration {
        let base = self.next_delay();
        match hint {
            Some(hint) if hint > base => hint,
            _ => base,
        }
    }

    pub async fn backoff(&self, delay: Duration) {
        tokio::time::sleep(delay).await;
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

    #[test]
    fn structured_status_classification() {
        use xylitol_plugin_sdk::error::XyProviderError;

        let retryable = |status: Option<u16>, code: Option<&str>| {
            XyError::provider(XyProviderError {
                message: "m".into(),
                status,
                code: code.map(str::to_string),
                ..Default::default()
            })
        };
        assert!(is_retryable_xy_error(&retryable(Some(429), None)));
        assert!(is_retryable_xy_error(&retryable(Some(529), None)));
        assert!(is_retryable_xy_error(&retryable(Some(503), None)));
        assert!(!is_retryable_xy_error(&retryable(Some(400), None)));
        assert!(!is_retryable_xy_error(&retryable(Some(401), None)));
        assert!(!is_retryable_xy_error(&retryable(
            None,
            Some("insufficient_quota")
        )));
        assert!(is_retryable_xy_error(&retryable(
            None,
            Some("overloaded_error")
        )));
        assert!(is_retryable_xy_error(&retryable(
            None,
            Some("rate_limit_error")
        )));
        assert!(is_retryable_xy_error(&XyError::Provider(anyhow::anyhow!(
            "connection refused"
        ))));
    }

    #[test]
    fn retry_after_hint_wins_when_larger() {
        let state = RetryState::new(3, 1000);
        assert_eq!(
            state.next_delay_with_hint(Some(Duration::from_secs(5))),
            Duration::from_secs(5)
        );
        let state = RetryState::new(3, 1000);
        assert_eq!(
            state.next_delay_with_hint(Some(Duration::from_millis(100))),
            Duration::from_millis(1000)
        );
    }

    #[tokio::test]
    async fn test_retry_state_exponential_backoff() {
        let state = RetryState::new(3, 1000);
        assert!(state.can_retry());

        let d1 = state.next_delay();
        assert_eq!(d1, Duration::from_millis(1000));

        let d2 = state.next_delay();
        assert_eq!(d2, Duration::from_millis(2000));

        let d3 = state.next_delay();
        assert_eq!(d3, Duration::from_millis(4000));

        let d4 = state.next_delay();
        assert_eq!(d4, Duration::from_millis(8000));
    }
}
