//! Provider adapter layer — L1 native APIs + L2 dialect overlays.
//!
//! ```text
//! YAML api × compat
//!        │
//!        ├─► native/     first-language HTTP (Responses / Completions / Anthropic)
//!        └─► dialect/    named quirks (deepseek, …) adjusting L1 bodies / WirePolicy
//! ```

use std::sync::Arc;

use async_trait::async_trait;

use crate::dto::{AiBridgeMessage, AiBridgeStream, AiBridgeToolSchema};
use crate::error::AiBridgeError;
use crate::thinking::AiBridgeGenerateOptions;

pub mod attribution;
pub mod dialect;
pub mod factory;
pub mod native;
pub mod obs_session;
pub mod remote_count;
pub mod reqwest_bridge;
pub mod tool_wire;
pub mod trace;

pub use attribution::{is_opencode_host, merge_opencode_attribution};
pub use dialect::{apply_anthropic_thinking, apply_completions_thinking};
pub use native::{
    AnthropicMessagesAdapter, OpenAiCompletionsAdapter, OpenAiResponsesAdapter, ResponsesAssembler,
    ResponsesStreamState, extract_embedded_provider_error_message, format_responses_error,
    map_responses_sse_event, messages_to_responses_input,
    messages_to_responses_input_with_diagnostics, messages_to_responses_input_with_options,
};
pub use obs_session::{
    ObsSessionContext, XYLITOL_OBS_LANE_ATTR, XYLITOL_OBS_LANE_INFRA, XYLITOL_OBS_LANE_LLM,
    clear_obs_session, langfuse_generation_properties, langfuse_observation_properties,
    langfuse_session_properties, obs_session_context, set_obs_session, set_obs_session_name,
    xylitol_obs_lane_properties,
};
pub use remote_count::{
    AnthropicRemoteCounter, OpenAiResponsesRemoteCounter, RemoteCounter, StubRemoteCounter,
};
pub use tool_wire::{
    canonical_tool_names, from_wire_tool_name, is_provider_safe_tool_name, to_wire_tool_name,
};

/// Supported adapter kinds (L1 protocol family).
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
