use std::sync::Arc;

use crate::hooks::HttpHooks;
use crate::wire_policy::WirePolicy;

use super::{AdapterKind, AdapterRef, OpenAiCompletionsAdapter, OpenAiResponsesAdapter};

pub fn build_adapter(
    api_key: String,
    model: String,
    base_url: Option<String>,
    kind: AdapterKind,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> AdapterRef {
    build_adapter_with_wire_policy(api_key, model, base_url, kind, hooks, WirePolicy::default())
}

pub fn build_adapter_with_wire_policy(
    api_key: String,
    model: String,
    base_url: Option<String>,
    kind: AdapterKind,
    hooks: Option<Arc<dyn HttpHooks>>,
    wire_policy: WirePolicy,
) -> AdapterRef {
    match kind {
        AdapterKind::OpenAiResponses => Arc::new(OpenAiResponsesAdapter::with_wire_policy(
            api_key,
            model,
            base_url,
            hooks.clone(),
            wire_policy,
        )),
        AdapterKind::OpenAiCompletions => Arc::new(OpenAiCompletionsAdapter::with_wire_policy(
            api_key,
            model,
            base_url,
            hooks,
            wire_policy,
        )),
        AdapterKind::AnthropicMessages => unreachable!(
            "composition root builds Anthropic via xylitol-official-plugin-provider-anthropic"
        ),
    }
}
