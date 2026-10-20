//! Provider adapter routing at the composition root.
//!
//! Dialect HTTP/SSE implementations live in the official provider plugins
//! and implement [`xylitol_plugin_sdk::XyModel`] directly.

pub mod factory;

use xylitol_plugin_sdk::model::XyModelKind;

/// Adapter kind (L1 protocol family). Composition-root routing only.
pub use xylitol_official_plugin_provider_openai::provider::AdapterKind;

/// Default adapter kind for a model kind. String SSOT:
/// `XyModelKind::default_adapter_api` (agent-safe).
pub fn default_for(kind: XyModelKind) -> AdapterKind {
    match kind.default_adapter_api() {
        "anthropic-messages" => AdapterKind::AnthropicMessages,
        _ => AdapterKind::OpenAiResponses,
    }
}
