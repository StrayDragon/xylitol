//! Adapter factory — constructs an [`crate::infra::provider::adapter::LlmAdapter`] from a model config.

use std::sync::Arc;

use crate::domain::model::XyModelConfig;
use crate::infra::hooks::HookDispatcher;
use crate::infra::provider::adapter::{
    AdapterKind, AdapterRef, AnthropicMessagesAdapter, OpenAiCompletionsAdapter,
    OpenAiResponsesAdapter,
};

/// Resolve the adapter kind for a model config.
pub fn resolve_adapter_kind(config: &XyModelConfig) -> AdapterKind {
    config
        .api
        .as_deref()
        .and_then(AdapterKind::from_config_str)
        .unwrap_or_else(|| AdapterKind::default_for(config.kind))
}

/// Build an adapter instance from a model config.
pub fn build_adapter(
    config: &XyModelConfig,
    hooks: Option<Arc<HookDispatcher>>,
) -> Result<AdapterRef, String> {
    let kind = resolve_adapter_kind(config);
    match kind {
        AdapterKind::OpenAiResponses => Ok(Arc::new(OpenAiResponsesAdapter::new(
            config.api_key.clone(),
            config.model.clone(),
            config.base_url.clone(),
            hooks.clone(),
        ))),
        AdapterKind::OpenAiCompletions => Ok(Arc::new(OpenAiCompletionsAdapter::new(
            config.api_key.clone(),
            config.model.clone(),
            config.base_url.clone(),
            hooks,
        ))),
        AdapterKind::AnthropicMessages => Ok(Arc::new(AnthropicMessagesAdapter::new(
            config.api_key.clone(),
            config.model.clone(),
            config.base_url.clone(),
            hooks,
        ))),
    }
}
