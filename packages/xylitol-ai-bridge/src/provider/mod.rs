//! Provider adapter layer — vendor-specific LLM API dialects.

use std::sync::Arc;

use async_trait::async_trait;

use crate::dto::{AiBridgeMessage, AiBridgeStream, AiBridgeToolSchema};
use crate::error::AiBridgeError;
use crate::thinking::AiBridgeGenerateOptions;

pub mod anthropic_messages;
pub mod factory;
pub mod obs_session;
pub mod obs_span_parent;
pub mod openai;
pub mod openai_client;
pub mod openai_completions;
pub mod openai_hooks_mw;
pub mod openai_responses;
pub mod remote_count;
pub mod reqwest_bridge;
pub mod trace;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use obs_session::{
    ObsSessionContext, XYLITOL_OBS_LANE_ATTR, XYLITOL_OBS_LANE_INFRA, XYLITOL_OBS_LANE_LLM,
    clear_obs_session, langfuse_generation_properties, langfuse_observation_properties,
    langfuse_session_properties, obs_session_context, set_obs_session, set_obs_session_name,
    xylitol_obs_lane_properties,
};
pub use obs_span_parent::{
    clear_obs_span_parents, obs_llm_parent, obs_turn_parent, set_obs_compaction_parent,
    set_obs_iteration_parent, set_obs_turn_parent,
};
pub use openai_completions::OpenAiCompletionsAdapter;
pub use openai_responses::{
    OpenAiResponsesAdapter, ResponsesStreamState, assemble_responses_body,
    extract_embedded_provider_error_message, format_responses_error, map_responses_sse_event,
    messages_to_responses_input, messages_to_responses_input_with_options,
};
pub use remote_count::{
    AnthropicRemoteCounter, OpenAiResponsesRemoteCounter, RemoteCounter, StubRemoteCounter,
};

/// Supported adapter kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    OpenAiResponses,
    OpenAiCompletions,
    AnthropicMessages,
}

impl AdapterKind {
    pub fn from_config_str(s: &str) -> Option<Self> {
        match s {
            "openai-responses" => Some(Self::OpenAiResponses),
            "openai-completions" => Some(Self::OpenAiCompletions),
            "anthropic-messages" => Some(Self::AnthropicMessages),
            _ => None,
        }
    }
}

impl std::fmt::Display for AdapterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterKind::OpenAiResponses => write!(f, "openai-responses"),
            AdapterKind::OpenAiCompletions => write!(f, "openai-completions"),
            AdapterKind::AnthropicMessages => write!(f, "anthropic-messages"),
        }
    }
}

#[async_trait]
pub trait AiBridgeLlmAdapter: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_stream(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError>;

    async fn generate(
        &self,
        messages: Vec<AiBridgeMessage>,
        tools: &[AiBridgeToolSchema],
        options: AiBridgeGenerateOptions,
    ) -> Result<AiBridgeStream, AiBridgeError>;
}

pub type AdapterRef = Arc<dyn AiBridgeLlmAdapter>;
