pub mod anthropic_messages;

pub use anthropic_messages::AnthropicMessagesAdapter;

pub fn default_user_agent() -> String {
    format!(
        "xylitol/{} ({}; rust; {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

pub mod wait_bounds {
    use std::time::Duration;
    pub const CONNECT: Duration = Duration::from_secs(10);
    pub const SSE_IDLE: Duration = Duration::from_secs(90);
    pub const NON_STREAM_TOTAL: Duration = Duration::from_secs(300);
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_agent_identifies_xylitol() {
        let ua = super::default_user_agent();
        assert!(ua.starts_with("xylitol/"));
        assert!(ua.contains(std::env::consts::OS));
        assert!(ua.contains(std::env::consts::ARCH));
    }
}
