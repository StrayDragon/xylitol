//! Adapter factory — constructs a [`AiBridgeLlmAdapter`] from connection parameters.

use std::sync::Arc;

use crate::hooks::HttpHooks;

use super::{
    AdapterKind, AdapterRef, AnthropicMessagesAdapter, OpenAiCompletionsAdapter,
    OpenAiResponsesAdapter,
};

pub fn build_adapter(
    api_key: String,
    model: String,
    base_url: Option<String>,
    kind: AdapterKind,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> AdapterRef {
    match kind {
        AdapterKind::OpenAiResponses => Arc::new(OpenAiResponsesAdapter::new(
            api_key,
            model,
            base_url,
            hooks.clone(),
        )),
        AdapterKind::OpenAiCompletions => Arc::new(OpenAiCompletionsAdapter::new(
            api_key, model, base_url, hooks,
        )),
        AdapterKind::AnthropicMessages => Arc::new(AnthropicMessagesAdapter::new(
            api_key, model, base_url, hooks,
        )),
    }
}
