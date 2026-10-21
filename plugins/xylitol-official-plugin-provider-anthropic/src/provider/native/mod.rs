pub mod anthropic_messages;

pub use anthropic_messages::AnthropicMessagesAdapter;

pub const DEFAULT_HTTP_USER_AGENT: &str = "Mozilla/5.0 (compatible; xylitol-official-plugin-provider-anthropic/0.1; +https://github.com/straydragon/xylitol)";

pub mod wait_bounds {
    use std::time::Duration;
    pub const CONNECT: Duration = Duration::from_secs(10);
    pub const SSE_IDLE: Duration = Duration::from_secs(90);
    pub const NON_STREAM_TOTAL: Duration = Duration::from_secs(300);
}
