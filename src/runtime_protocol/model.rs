//! Runtime boundary for LLM providers.

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;

use crate::domain::error::XyError;
use crate::domain::message::AgentMessage;
use crate::domain::model::XyModelConfig;
use crate::domain::types::{ThinkingBudgets, ThinkingLevel, ThinkingLevelMap, XyChunk};

/// Streaming response from an LLM provider.
pub type XyStream = Pin<Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>>;

/// Factory that builds an [`XyModel`] instance from a [`XyModelConfig`].
///
/// Supplied by the composition root (the agent layer must not construct
/// providers itself). Aliased because the closure signature is repeated across the
/// agent module, session, model manager, and composition roots.
pub type XyModelBuilder = std::sync::Arc<
    dyn Fn(&XyModelConfig) -> Result<std::sync::Arc<dyn XyModel>, String> + Send + Sync,
>;

/// Options for a single [`XyModel::generate_stream`] call.
///
/// Default (`Off` + empty map + no budgets) matches historical “no thinking fields”
/// request bodies for most adapters.
#[derive(Debug, Clone)]
pub struct XyGenerateOptions {
    pub thinking_level: ThinkingLevel,
    pub level_map: ThinkingLevelMap,
    pub thinking_budgets: Option<ThinkingBudgets>,
}

impl Default for XyGenerateOptions {
    fn default() -> Self {
        Self {
            thinking_level: ThinkingLevel::Off,
            level_map: HashMap::new(),
            thinking_budgets: None,
        }
    }
}

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
        options: XyGenerateOptions,
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

    #[test]
    fn generate_options_default_is_off() {
        let opts = XyGenerateOptions::default();
        assert_eq!(opts.thinking_level, ThinkingLevel::Off);
        assert!(opts.level_map.is_empty());
        assert!(opts.thinking_budgets.is_none());
    }
}
