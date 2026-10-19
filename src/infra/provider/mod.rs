//! LLM providers for xylitol.
//!
//! Dialect HTTP/SSE implementations live in the official provider plugins
//! (`impl XyModel`). This module owns Fake assembly and the composition-root factory.

pub mod adapter;
pub mod factory;
pub mod hooks_port;
pub mod map;

mod fake;

pub use fake::*;

/// Re-export provider-trace gate so CLI logging keeps a stable path.
pub mod trace {
    pub use xylitol_official_plugin_provider_openai::provider::trace::{
        ObservationIoTier, set_observation_io_tier, set_provider_trace_active,
        set_tool_observation_io_tier,
    };
}
