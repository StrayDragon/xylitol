//! Provider adapter layer — maps xylitol-ai-bridge adapters into domain streams.
//!
//! Vendor HTTP/SSE implementations live in `packages/xylitol-ai-bridge`. The
//! domain-facing surface is the single-layer [`AdapterXyModel`] shell (pa1):
//! it holds a bridge `AiBridgeLlmAdapter` directly and runs the DTO→`XyChunk`
//! mapping inline via [`crate::infra::provider::map`].

pub mod factory;
pub mod xy_model;

pub use xy_model::AdapterXyModel;

use crate::protocol::model::XyModelKind;

/// Adapter kind (bridge enum, re-exported so domain code never mirrors it).
pub use xylitol_ai_bridge::provider::AdapterKind;

/// Default adapter kind for a model kind. String SSOT:
/// `XyModelKind::default_adapter_api` (agent-safe).
pub fn default_for(kind: XyModelKind) -> AdapterKind {
    match kind.default_adapter_api() {
        "anthropic-messages" => AdapterKind::AnthropicMessages,
        _ => AdapterKind::OpenAiResponses,
    }
}
