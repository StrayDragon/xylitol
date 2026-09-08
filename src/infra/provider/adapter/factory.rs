//! Adapter factory — constructs a bridge adapter (pa1: the domain
//! [`super::AdapterXyModel`] shell wraps it directly).

use std::sync::Arc;

use crate::infra::hooks::HookDispatcher;
use crate::infra::provider::adapter::{AdapterKind, default_for};
use crate::infra::provider::hooks_port::to_http_hooks;
use crate::protocol::model::XyModelConfig;
use xylitol_ai_bridge::provider::AdapterRef as AiBridgeAdapterRef;

/// Resolve the adapter kind for a model config.
pub fn resolve_adapter_kind(config: &XyModelConfig) -> AdapterKind {
    config
        .api
        .as_deref()
        .and_then(AdapterKind::from_config_str)
        .unwrap_or_else(|| default_for(config.kind))
}

/// Resolve named YAML `compat` → bridge [`xylitol_ai_bridge::WirePolicy`].
///
/// Unknown / omitted → generic default. AdapterKind selection ignores WirePolicy (pa4).
pub fn resolve_wire_policy(config: &XyModelConfig) -> xylitol_ai_bridge::WirePolicy {
    let compat = config
        .compat
        .as_deref()
        .and_then(xylitol_ai_bridge::Compat::parse)
        .unwrap_or_default();
    xylitol_ai_bridge::WirePolicy::for_compat(compat)
}

/// Build an adapter instance from a model config.
///
/// WirePolicy comes from named `compat` profiles in the bridge (c1940); free-form
/// `extra_policy` YAML is not accepted. AdapterKind still ignores WirePolicy.
pub fn build_adapter(
    config: &XyModelConfig,
    hooks: Option<Arc<HookDispatcher>>,
) -> AiBridgeAdapterRef {
    let kind = resolve_adapter_kind(config);
    let http_hooks = to_http_hooks(hooks);
    let wire_policy = resolve_wire_policy(config);
    xylitol_ai_bridge::provider::factory::build_adapter_with_wire_policy(
        config.api_key.clone(),
        config.model.clone(),
        config.base_url.clone(),
        kind,
        http_hooks,
        wire_policy,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::model::{XyModelConfig, XyModelKind};

    #[test]
    fn resolve_adapter_kind_uses_explicit_completions() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "gpt".into(),
            base_url: None,
            api: Some("openai-completions".into()),
            compat: None,
        };
        assert_eq!(resolve_adapter_kind(&cfg), AdapterKind::OpenAiCompletions);
        let cfg2 = XyModelConfig { api: None, ..cfg };
        assert_eq!(resolve_adapter_kind(&cfg2), AdapterKind::OpenAiResponses);
    }

    #[test]
    fn resolve_wire_policy_named_deepseek() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
            api: Some("openai-responses".into()),
            compat: Some("deepseek".into()),
        };
        let p = resolve_wire_policy(&cfg);
        assert_eq!(p.compat, xylitol_ai_bridge::Compat::Deepseek);
        assert!(!p.allows_reasoning_encrypted_include());
    }

    #[test]
    fn model_entry_allows_compat_forbids_extra_policy_yaml() {
        let src = include_str!("../../config/types.rs");
        let entry = src
            .split("pub struct ModelEntry")
            .nth(1)
            .and_then(|s| s.split("pub struct ").next())
            .expect("ModelEntry block");
        assert!(
            entry.lines().any(|l| l.trim().starts_with("pub compat")),
            "ModelEntry must expose named compat (c1940)"
        );
        assert!(
            !entry.lines().any(|l| {
                let t = l.trim();
                t.starts_with("pub extra_policy") || t.starts_with("extra_policy:")
            }),
            "ModelEntry must not grow free-form extra_policy YAML"
        );
    }
}
