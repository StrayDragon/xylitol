//! LLM providers for xylitol.
//!
//! Dialect HTTP/SSE implementations live in [`xylitol_ai_bridge`]. This module
//! owns domain mapping, Fake/`XyModel` assembly, and the composition-root factory.
//! - OpenAI Responses / Completions / Anthropic Messages → `adapter::AdapterXyModel`
//! - [`FakeProvider`]: scenario-based mock for offline testing

pub mod adapter;
pub mod factory;
pub mod hooks_port;
pub mod map;

mod fake;

pub use fake::*;

pub use crate::protocol::model::{ContextTokenEstimate, TokenProvenance};

/// Re-export provider-trace gate so CLI logging keeps a stable path.
pub mod trace {
    pub use xylitol_ai_bridge::provider::trace::{
        ObsGateScope, ObsGateState, ObservationIoTier, PROVIDER_TRACE_TEXT_MAX,
        ProviderRequestTrace, SpanCollectScope, observation_io_tier, provider_trace_active,
        set_observation_io_tier, set_provider_trace_active, set_tool_observation_io_tier,
        tool_observation_io_tier,
    };
}
