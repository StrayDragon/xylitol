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
pub use native::AnthropicMessagesAdapter;
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

pub type AdapterRef = Arc<dyn XyModel>;
