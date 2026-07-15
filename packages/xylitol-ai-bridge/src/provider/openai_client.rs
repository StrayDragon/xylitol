//! Shared `async-openai` Client construction for OpenAI-compatible dialects.

use std::sync::Arc;

use async_openai::Client;
use async_openai::config::OpenAIConfig;

use crate::hooks::HttpHooks;
use crate::provider::openai_hooks_mw::HooksHttpService;

/// Build an OpenAI-compatible [`Client`] with xylitol hook middleware.
pub fn build_openai_client(
    api_key: String,
    base_url: Option<String>,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> Client<OpenAIConfig> {
    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()));
    Client::with_config(config).with_http_service(HooksHttpService::new(hooks))
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
