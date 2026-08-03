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
///
/// Injects [`xylitol_ai_bridge::WirePolicy::default`] for Responses (c1880);
/// does not own a parallel defaults table and does not read YAML compat/extra_policy.
pub fn build_adapter(
    config: &XyModelConfig,
    hooks: Option<Arc<HookDispatcher>>,
) -> Result<AdapterRef, String> {
    let kind = resolve_adapter_kind(config);
    let http_hooks = to_http_hooks(hooks);
    // Assembly injects bridge defaults; AdapterKind selection ignores WirePolicy (pa4/pa21).
    let wire_policy = xylitol_ai_bridge::WirePolicy::default();
    let bridge = xylitol_ai_bridge::provider::factory::build_adapter_with_wire_policy(
        config.api_key.clone(),
        config.model.clone(),
        config.base_url.clone(),
        kind.to_bridge(),
        http_hooks,
        wire_policy,
    );
    Ok(Arc::new(MappedBridgeAdapter::new(bridge)) as AdapterRef)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model_config::{XyModelConfig, XyModelKind};

    #[test]
    fn resolve_adapter_kind_ignores_wire_policy_uses_api_only() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "gpt".into(),
            base_url: None,
            api: Some("openai-completions".into()),
        };
        assert_eq!(resolve_adapter_kind(&cfg), AdapterKind::OpenAiCompletions);
        let cfg2 = XyModelConfig { api: None, ..cfg };
        assert_eq!(resolve_adapter_kind(&cfg2), AdapterKind::OpenAiResponses);
    }

    #[test]
    fn model_entry_has_no_compat_or_extra_policy_yaml_fields() {
        // pa21: unexposed wire knobs are not ModelEntry YAML.
        let src = include_str!("../../config/types.rs");
        let entry = src
            .split("pub struct ModelEntry")
            .nth(1)
            .and_then(|s| s.split("pub struct ").next())
            .expect("ModelEntry block");
        assert!(
            !entry.lines().any(|l| {
                let t = l.trim();
                t.starts_with("pub compat") || t.starts_with("compat:")
            }),
            "ModelEntry must not grow a compat YAML field in c1880"
        );
        assert!(
            !entry.lines().any(|l| {
                let t = l.trim();
                t.starts_with("pub extra_policy") || t.starts_with("extra_policy:")
            }),
            "ModelEntry must not grow extra_policy YAML field in c1880"
        );
    }
}
