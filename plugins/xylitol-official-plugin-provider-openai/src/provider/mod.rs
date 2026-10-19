//! Provider adapter layer — L1 native APIs + L2 dialect overlays.
//!
//! Dialects implement [`xylitol_plugin_sdk::XyModel`] directly.

use std::sync::Arc;

use xylitol_plugin_sdk::XyModel;

pub mod attribution;
pub mod dialect;
pub mod factory;
pub mod native;
pub mod obs_session;
pub mod reqwest_headers;
pub mod tool_wire;
pub mod trace;

pub use attribution::{
    is_opencode_host, merge_opencode_attribution, merge_opencode_attribution_from,
    obs_for_llm_request, presented_llm_gateway_session_id,
};
pub use dialect::{apply_anthropic_thinking, apply_completions_thinking};
pub use native::{
    OpenAiCompletionsAdapter, OpenAiResponsesAdapter, ResponsesAssembler, ResponsesStreamState,
    extract_embedded_provider_error_message, format_responses_error, map_responses_sse_event,
    messages_to_responses_input, messages_to_responses_input_with_diagnostics,
    messages_to_responses_input_with_options,
};
pub use obs_session::{
    ObsSessionContext, ObsSessionScope, XYLITOL_OBS_LANE_ATTR, XYLITOL_OBS_LANE_LLM,
    clear_obs_session, langfuse_generation_properties, langfuse_generation_properties_from,
    langfuse_observation_properties, langfuse_observation_properties_from,
    langfuse_session_properties, langfuse_session_properties_from, obs_session_context,
    set_obs_session, set_obs_session_name, xylitol_obs_lane_properties,
};
pub use tool_wire::{
    canonical_tool_names, from_wire_tool_name, is_provider_safe_tool_name, to_wire_tool_name,
};
pub use trace::{ObsGateScope, ObsGateState, SpanCollectScope};

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

pub type AdapterRef = Arc<dyn XyModel>;
