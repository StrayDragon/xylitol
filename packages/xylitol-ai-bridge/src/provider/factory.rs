//! Adapter factory — constructs a [`AiBridgeLlmAdapter`] from connection parameters.

use std::sync::Arc;

use crate::hooks::HttpHooks;
use crate::wire_policy::WirePolicy;

use super::{
    AdapterKind, AdapterRef, AnthropicMessagesAdapter, OpenAiCompletionsAdapter,
    OpenAiResponsesAdapter,
};

/// Build an adapter using [`WirePolicy::default`] for Responses.
pub fn build_adapter(
    api_key: String,
    model: String,
    base_url: Option<String>,
    kind: AdapterKind,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> AdapterRef {
    build_adapter_with_wire_policy(api_key, model, base_url, kind, hooks, WirePolicy::default())
}

/// Build an adapter; `wire_policy` is applied on the Responses path only.
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
        AdapterKind::OpenAiCompletions => Arc::new(OpenAiCompletionsAdapter::new(
            api_key, model, base_url, hooks,
        )),
        AdapterKind::AnthropicMessages => Arc::new(AnthropicMessagesAdapter::new(
            api_key, model, base_url, hooks,
        )),
    }
}
