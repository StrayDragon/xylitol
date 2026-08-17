//! Runtime boundary for LLM providers.

use std::pin::Pin;

use async_trait::async_trait;

use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyChunk;
use crate::protocol::model::XyModelConfig;

/// Streaming response from an LLM provider.
pub type XyStream = Pin<Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>>;

/// Factory that builds an [`XyModel`] instance from a [`XyModelConfig`].
///
/// Supplied by the composition root (the agent layer must not construct
/// providers itself). Aliased because the closure signature is repeated across the
/// agent module, session, model manager, and composition roots.
pub type XyModelBuilder =
    std::sync::Arc<dyn Fn(&XyModelConfig) -> std::sync::Arc<dyn XyModel> + Send + Sync>;

/// Options for a single [`XyModel::generate_stream`] call.
///
/// Bridge DTO alias (fields identical to
/// [`xylitol_ai_bridge::AiBridgeGenerateOptions`]). Default (`off` + empty
/// map + no budgets) matches historical “no thinking fields” request bodies
/// for most adapters.
pub type XyGenerateOptions = xylitol_ai_bridge::AiBridgeGenerateOptions;

/// LLM provider contract.
///
/// Implementations connect to a remote API (OpenAI, Anthropic, etc.) and
/// produce a streaming response from LLM-visible history.
///
/// Callers MUST project session [`crate::protocol::message::AgentMessage`] history
/// via [`crate::agent::llm_project::project_for_llm`] before invoking this trait.
/// [`LlmMessage`] is a type alias of bridge `AiBridgeMessage`.
#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    /// Generate a streaming response from LLM-only message history.
    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[crate::protocol::model::XyToolSchema],
        stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::THINKING_OFF;

    #[test]
    fn xy_stream_type_is_send() {
        // Compile-time check: XyStream must be Send
        fn assert_send<T: Send>() {}
        assert_send::<XyStream>();
    }

    #[test]
    fn generate_options_default_is_off() {
        let opts = XyGenerateOptions::default();
        assert_eq!(opts.thinking_level, THINKING_OFF);
        assert!(opts.level_map.is_empty());
        assert!(opts.thinking_budgets.is_none());
    }
}
