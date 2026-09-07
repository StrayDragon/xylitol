//! Shared `async-openai` Client construction for OpenAI-compatible dialects.

use std::sync::Arc;

use async_openai::Client;
use async_openai::config::OpenAIConfig;

use crate::hooks::HttpHooks;
use crate::provider::native::openai_hooks_mw::HooksHttpService;
use crate::provider::obs_session::ObsSessionContext;

/// Default User-Agent for OpenAI-compatible HTTP (Cloudflare / gateway friendly).
pub const DEFAULT_HTTP_USER_AGENT: &str =
    "Mozilla/5.0 (compatible; xylitol-ai-bridge/0.1; +https://github.com/straydragon/xylitol)";

fn openai_config(api_key: String, base_url: Option<String>) -> OpenAIConfig {
    OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()))
}

/// Factory that clones a per-generate HTTP service with an obs snapshot (c2590).
pub struct OpenAiClientFactory {
    config: OpenAIConfig,
    http: HooksHttpService,
    api_base: String,
}

impl OpenAiClientFactory {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        hooks: Option<Arc<dyn HttpHooks>>,
    ) -> Self {
        let api_base = base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".into());
        Self {
            config: openai_config(api_key, base_url),
            http: HooksHttpService::new(hooks),
            api_base,
        }
    }

    /// Request base used for gateway attribution / obs facts.
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// Per-call wrapping: bind this generate's obs snapshot onto hooks.
    pub fn bind(&self, obs: &ObsSessionContext) -> Client<OpenAIConfig> {
        Client::with_config(self.config.clone()).with_http_service(self.http.bind_obs(obs.clone()))
    }
}

/// Build an OpenAI-compatible [`Client`] with xylitol hook middleware.
///
/// Idle path (remote count): no generate snapshot — middleware may fall back
/// to the process obs slot.
pub fn build_openai_client(
    api_key: String,
    base_url: Option<String>,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> Client<OpenAIConfig> {
    Client::with_config(openai_config(api_key, base_url))
        .with_http_service(HooksHttpService::new(hooks))
}

/// Normalize a base that may be `https://host` or `https://host/v1`.
pub fn normalize_openai_v1_base(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}
