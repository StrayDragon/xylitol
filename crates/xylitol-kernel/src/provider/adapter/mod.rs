pub mod factory;

use xylitol_plugin_sdk::model::XyModelKind;

pub use xylitol_official_plugin_provider_openai::provider::AdapterKind;

pub fn default_for(kind: XyModelKind) -> AdapterKind {
    match kind.default_adapter_api() {
        "anthropic-messages" => AdapterKind::AnthropicMessages,
        _ => AdapterKind::OpenAiResponses,
    }
}
