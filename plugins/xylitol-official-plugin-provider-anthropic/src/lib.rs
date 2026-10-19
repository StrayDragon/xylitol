//! Official Host plugin: Anthropic Messages HTTP (`impl XyModel`).

pub mod dto;
pub mod error;
pub mod hooks;
pub mod provider;
pub mod thinking;
pub mod wire_policy;

pub use error::XyError;
pub use provider::AnthropicMessagesAdapter;
pub use thinking::{
    ResolvedThinking, XyGenerateOptions, apply_thinking_anthropic, resolve_from_options,
};
pub use wire_policy::{Compat, WirePolicy};
