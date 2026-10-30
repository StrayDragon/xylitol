use std::sync::Arc;

use crate::provider::adapter::{AdapterKind, default_for};
use crate::provider::hooks_port::to_http_hooks;
use xylitol_official_plugin_hooks::HookDispatcher;
use xylitol_official_plugin_provider_openai::provider::AdapterRef;
use xylitol_plugin_sdk::model::XyModelConfig;

pub fn resolve_adapter_kind(config: &XyModelConfig) -> AdapterKind {
    config
        .api
        .as_deref()
        .and_then(AdapterKind::from_config_str)
        .unwrap_or_else(|| default_for(config.kind))
}

pub fn resolve_wire_policy(
    config: &XyModelConfig,
) -> xylitol_official_plugin_provider_openai::WirePolicy {
    let compat = config
        .compat
        .as_deref()
        .and_then(xylitol_official_plugin_provider_openai::Compat::parse)
        .unwrap_or_default();
    let mut policy = xylitol_official_plugin_provider_openai::WirePolicy::for_compat(compat);
    apply_prompt_cache_policy(&mut policy, config.prompt_cache);
    policy
}

fn apply_prompt_cache_policy(
    policy: &mut xylitol_official_plugin_provider_openai::WirePolicy,
    cache: Option<xylitol_plugin_sdk::model::XyPromptCacheConfig>,
) {
    let Some(cache) = cache else {
        return;
    };
    if let Some(enabled) = cache.enabled {
        policy.extra_policy.prompt_cache_key = enabled;
    }
    policy.extra_policy.prompt_cache_retention = cache.retention;
}

pub fn build_adapter(config: &XyModelConfig, hooks: Option<Arc<HookDispatcher>>) -> AdapterRef {
    let kind = resolve_adapter_kind(config);
    let http_hooks = to_http_hooks(hooks);
    let wire_policy = resolve_wire_policy(config);
    match kind {
        AdapterKind::AnthropicMessages => {
            let anthropic_policy =
                xylitol_official_plugin_provider_anthropic::WirePolicy::for_compat(wire_policy.compat);
            xylitol_official_plugin_provider_anthropic::provider::factory::build_adapter_with_wire_policy(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
                http_hooks,
                anthropic_policy,
                config.prompt_cache,
            )
        }
        AdapterKind::OpenAiResponses | AdapterKind::OpenAiCompletions => {
            xylitol_official_plugin_provider_openai::provider::factory::build_adapter_with_wire_policy(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
                kind,
                http_hooks,
                wire_policy,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xylitol_plugin_sdk::model::{XyModelConfig, XyModelKind};

    #[test]
    fn resolve_adapter_kind_uses_explicit_completions() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "gpt".into(),
            base_url: None,
            api: Some("openai-completions".into()),
            compat: None,
            prompt_cache: None,
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
            prompt_cache: None,
        };
        let p = resolve_wire_policy(&cfg);
        assert_eq!(
            p.compat,
            xylitol_official_plugin_provider_openai::Compat::Deepseek
        );
        assert!(!p.allows_reasoning_encrypted_include());
    }

    #[test]
    fn resolve_wire_policy_applies_prompt_cache_config() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
            api: None,
            compat: Some("deepseek".into()),
            prompt_cache: Some(xylitol_plugin_sdk::model::XyPromptCacheConfig {
                enabled: Some(true),
                ttl: None,
                retention: Some(xylitol_plugin_sdk::model::XyPromptCacheRetention::TwentyFourHours),
            }),
        };
        let p = resolve_wire_policy(&cfg);
        assert!(
            p.allows_prompt_cache_key(),
            "explicit config must override the deepseek default"
        );
        assert_eq!(
            p.prompt_cache_retention(),
            Some(xylitol_plugin_sdk::model::XyPromptCacheRetention::TwentyFourHours)
        );
    }

    #[test]
    fn prompt_cache_none_keeps_compat_default() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
            api: None,
            compat: Some("deepseek".into()),
            prompt_cache: None,
        };
        let p = resolve_wire_policy(&cfg);
        assert!(!p.allows_prompt_cache_key());
        assert_eq!(p.prompt_cache_retention(), None);
    }

    #[test]
    fn prompt_cache_enabled_absent_keeps_default() {
        let cfg = XyModelConfig {
            kind: XyModelKind::OpenAi,
            api_key: "sk".into(),
            model: "m".into(),
            base_url: None,
            api: None,
            compat: Some("deepseek".into()),
            prompt_cache: Some(xylitol_plugin_sdk::model::XyPromptCacheConfig {
                enabled: None,
                ttl: None,
                retention: Some(xylitol_plugin_sdk::model::XyPromptCacheRetention::TwentyFourHours),
            }),
        };
        let p = resolve_wire_policy(&cfg);
        assert!(!p.allows_prompt_cache_key());
        assert_eq!(
            p.prompt_cache_retention(),
            Some(xylitol_plugin_sdk::model::XyPromptCacheRetention::TwentyFourHours)
        );
    }

    #[test]
    fn model_entry_allows_compat_forbids_extra_policy_yaml() {
        let src = include_str!("../../../../../crates/xylitol-plugin-sdk/src/model_entry.rs");
        let entry = src
            .split("pub struct XyModelEntryConfig")
            .nth(1)
            .and_then(|s| s.split("\n\nimpl ").next())
            .expect("XyModelEntryConfig block");
        assert!(
            entry.lines().any(|l| l.trim().starts_with("pub compat")),
            "XyModelEntryConfig must expose named compat (c1940)"
        );
        assert!(
            !entry.lines().any(|l| {
                let t = l.trim();
                t.starts_with("pub extra_policy") || t.starts_with("extra_policy:")
            }),
            "XyModelEntryConfig must not grow free-form extra_policy YAML"
        );
    }
}
