//! Runtime boundary for LLM providers.

use std::pin::Pin;

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::model::ModelConfig;
use crate::domain::types::XyChunk;

/// Streaming response from an LLM provider.
pub type XyStream = Pin<Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>>;

/// Factory that builds an [`XyModel`] instance from a [`ModelConfig`].
///
/// Supplied by the composition root (HC-1: agent/ must not construct providers
/// itself). Aliased because the closure signature is repeated across the
/// agent facade, session, model manager, and composition roots.
pub type ModelBuilder = std::sync::Arc<
    dyn Fn(&ModelConfig) -> Result<std::sync::Arc<dyn XyModel>, String> + Send + Sync,
>;

/// LLM provider contract.
///
/// Implementations connect to a remote API (OpenAI, Anthropic, etc.) and
/// produce a streaming response from a conversation history.
#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    /// Generate a streaming response from AgentMessage history.
    async fn generate_stream(
        &self,
        messages: Vec<AgentMessage>,
        tools: &[crate::domain::types::XyToolSchema],
        stream: bool,
    ) -> Result<XyStream, XyError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xy_stream_type_is_send() {
        // Compile-time check: XyStream must be Send
        fn assert_send<T: Send>() {}
        assert_send::<XyStream>();
    }
}
