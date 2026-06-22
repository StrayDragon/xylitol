//! HTTP dispatcher — proxy and timeout configuration for reqwest clients.
//!
//! Aligns with pi's http-dispatcher.ts. Provides:
//! - reqwest::Client configuration (builder-level proxy + timeouts)
//! - Idle timeout value parsing from config strings

use std::time::Duration;

// ── Constants ──────────────────────────────────────────────────────

/// Default HTTP idle timeout (5 minutes).
pub const DEFAULT_HTTP_IDLE_TIMEOUT_MS: u64 = 300_000;

/// Preset timeout choices matching pi's HTTP_IDLE_TIMEOUT_CHOICES.
pub const HTTP_IDLE_TIMEOUT_CHOICES: &[TimeoutChoice] = &[
    TimeoutChoice {
        label: "30 sec",
        timeout_ms: 30_000,
    },
    TimeoutChoice {
        label: "1 min",
        timeout_ms: 60_000,
    },
    TimeoutChoice {
        label: "2 min",
        timeout_ms: 120_000,
    },
    TimeoutChoice {
        label: "5 min",
        timeout_ms: 300_000,
    },
    TimeoutChoice {
        label: "disabled",
        timeout_ms: 0,
    },
];

/// A named timeout preset.
#[derive(Debug, Clone, Copy)]
pub struct TimeoutChoice {
    pub label: &'static str,
    pub timeout_ms: u64,
}

// ── Timeout Parsing ────────────────────────────────────────────────

/// Parse an HTTP idle timeout value from a config string or number.
///
/// Accepts:
/// - Numeric milliseconds: `"300000"` or `300000`
/// - Named presets: `"30 sec"`, `"1 min"`, `"5 min"`, `"disabled"`
/// - Empty string: returns `None`
///
/// Returns `None` for invalid/unparseable values.
pub fn parse_http_idle_timeout_ms(value: &str) -> Option<u64> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("disabled") {
        return Some(0);
    }

    // Check named presets
    for choice in HTTP_IDLE_TIMEOUT_CHOICES {
        if trimmed.eq_ignore_ascii_case(choice.label) {
            return Some(choice.timeout_ms);
        }
    }

    // Try parsing as a number
    if let Ok(ms) = trimmed.parse::<u64>() {
        return Some(ms);
    }

    None
}

/// Format a timeout value as a human-readable label.
pub fn format_http_idle_timeout_ms(timeout_ms: u64) -> String {
    for choice in HTTP_IDLE_TIMEOUT_CHOICES {
        if choice.timeout_ms == timeout_ms {
            return choice.label.to_string();
        }
    }
    format!("{} sec", timeout_ms / 1000)
}

// ── Client Configuration ───────────────────────────────────────────

/// Configure a `reqwest::ClientBuilder` with proxy and timeout settings.
///
/// Applies:
/// - `connect_timeout` if timeout > 0
/// - `read_timeout` if timeout > 0
/// - `pool_idle_timeout` (half of the idle timeout, min 30s)
/// - Proxy via `reqwest::Proxy::all` when `http_proxy` is non-empty
pub fn configure_http_client(
    builder: reqwest::ClientBuilder,
    timeout_ms: Option<u64>,
    http_proxy: Option<&str>,
) -> reqwest::ClientBuilder {
    let mut builder = builder;

    // Proxy
    if let Some(proxy_url) = http_proxy
        && !proxy_url.trim().is_empty()
    {
        let proxy_url = proxy_url.trim();
        if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
            builder = builder.proxy(proxy);
        }
    }

    // Timeouts
    if let Some(ms) = timeout_ms
        && ms > 0
    {
        let duration = Duration::from_millis(ms);
        builder = builder
            .connect_timeout(duration)
            .read_timeout(duration)
            .pool_idle_timeout(Duration::from_millis((ms / 2).max(30_000)));
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timeout_presets() {
        assert_eq!(parse_http_idle_timeout_ms("30 sec"), Some(30_000));
        assert_eq!(parse_http_idle_timeout_ms("1 min"), Some(60_000));
        assert_eq!(parse_http_idle_timeout_ms("2 min"), Some(120_000));
        assert_eq!(parse_http_idle_timeout_ms("5 min"), Some(300_000));
        assert_eq!(parse_http_idle_timeout_ms("disabled"), Some(0));
    }

    #[test]
    fn test_parse_timeout_numeric() {
        assert_eq!(parse_http_idle_timeout_ms("30000"), Some(30_000));
        assert_eq!(parse_http_idle_timeout_ms("0"), Some(0));
    }

    #[test]
    fn test_parse_timeout_empty() {
        assert_eq!(parse_http_idle_timeout_ms(""), None);
    }

    #[test]
    fn test_parse_timeout_invalid() {
        assert_eq!(parse_http_idle_timeout_ms("not-a-number"), None);
    }

    #[test]
    fn test_parse_timeout_whitespace() {
        assert_eq!(parse_http_idle_timeout_ms("  30 sec  "), Some(30_000));
    }

    #[test]
    fn test_format_timeout_preset() {
        assert_eq!(format_http_idle_timeout_ms(30_000), "30 sec");
        assert_eq!(format_http_idle_timeout_ms(300_000), "5 min");
    }

    #[test]
    fn test_format_timeout_custom() {
        assert_eq!(format_http_idle_timeout_ms(45_000), "45 sec");
    }

    #[test]
    fn test_format_timeout_disabled() {
        assert_eq!(format_http_idle_timeout_ms(0), "disabled");
    }

    #[test]
    fn test_configure_http_client_timeout() {
        let builder = reqwest::ClientBuilder::new();
        let configured = configure_http_client(builder, Some(60_000), None);
        // Can't easily inspect the configured builder, but ensure it doesn't panic
        let _client = configured.build().unwrap();
    }

    #[test]
    fn test_configure_http_client_proxy() {
        let builder = reqwest::ClientBuilder::new();
        let configured = configure_http_client(builder, None, Some("http://proxy.example:8080"));
        let _client = configured.build().unwrap();
    }

    #[test]
    fn test_configure_http_client_no_timeout() {
        let builder = reqwest::ClientBuilder::new();
        let configured = configure_http_client(builder, Some(0), None);
        let _client = configured.build().unwrap();
    }

    #[test]
    fn test_timeout_choices() {
        assert!(!HTTP_IDLE_TIMEOUT_CHOICES.is_empty());
        assert_eq!(HTTP_IDLE_TIMEOUT_CHOICES[0].label, "30 sec");
        assert_eq!(HTTP_IDLE_TIMEOUT_CHOICES[4].label, "disabled");
        assert_eq!(HTTP_IDLE_TIMEOUT_CHOICES[4].timeout_ms, 0);
    }
}
