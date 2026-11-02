use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DEFAULT_IDLE_GRACE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ServeLifecycle {
    #[default]
    Persistent,
    LastClient,
}

impl ServeLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Persistent => "persistent",
            Self::LastClient => "last-client",
        }
    }

    pub fn parse_name(raw: &str) -> Option<Self> {
        match raw.trim() {
            "persistent" => Some(Self::Persistent),
            "last-client" => Some(Self::LastClient),
            _ => None,
        }
    }

    pub fn is_persistent(self) -> bool {
        matches!(self, Self::Persistent)
    }
}

pub fn parse_idle_duration(raw: &str) -> Option<Duration> {
    let raw = raw.trim();
    if let Some(n) = raw.strip_suffix("ms") {
        return n.parse().ok().map(Duration::from_millis);
    }
    if let Some(n) = raw.strip_suffix('s') {
        return n.parse().ok().map(Duration::from_secs);
    }
    if let Some(n) = raw.strip_suffix('m') {
        return n
            .parse::<u64>()
            .ok()
            .map(|m| Duration::from_secs(m.saturating_mul(60)));
    }
    None
}

pub fn format_idle_timeout(duration: Duration) -> String {
    let millis = duration.as_millis();
    if millis > 0 && millis.is_multiple_of(1000) {
        format!("{}s", millis / 1000)
    } else {
        format!("{millis}ms")
    }
}
