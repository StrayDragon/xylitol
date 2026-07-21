//! Adapter factory — constructs a domain [`super::LlmAdapter`] via xylitol-ai-bridge.

use std::sync::Arc;

use crate::infra::hooks::HookDispatcher;
use crate::infra::provider::adapter::{AdapterKind, AdapterRef, MappedBridgeAdapter};
use crate::infra::provider::hooks_port::to_http_hooks;
use crate::protocol::model_config::XyModelConfig;

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
    let http_hooks = to_http_hooks(hooks);
    let bridge = xylitol_ai_bridge::provider::factory::build_adapter(
        config.api_key.clone(),
        config.model.clone(),
        config.base_url.clone(),
        kind.to_bridge(),
        http_hooks,
    );
    Ok(Arc::new(MappedBridgeAdapter::new(bridge)) as AdapterRef)
}
