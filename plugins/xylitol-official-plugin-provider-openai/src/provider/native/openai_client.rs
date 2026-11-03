use std::sync::Arc;

use async_openai::Client;
use async_openai::config::OpenAIConfig;

use crate::hooks::HttpHooks;
use crate::provider::native::openai_hooks_mw::HooksHttpService;
use crate::provider::obs_session::ObsSessionContext;

pub fn default_user_agent() -> String {
    format!(
        "XYlitol/{} ({}; rust; {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn openai_config(api_key: String, base_url: Option<String>) -> OpenAIConfig {
    OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()))
}

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

    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    pub fn bind(&self, obs: &ObsSessionContext) -> Client<OpenAIConfig> {
        Client::with_config(self.config.clone()).with_http_service(self.http.bind_obs(obs.clone()))
    }
}

pub fn build_openai_client(
    api_key: String,
    base_url: Option<String>,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> Client<OpenAIConfig> {
    Client::with_config(openai_config(api_key, base_url))
        .with_http_service(HooksHttpService::new(hooks))
}

pub fn normalize_openai_v1_base(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_agent_identifies_xylitol() {
        let ua = super::default_user_agent();
        assert!(ua.starts_with("XYlitol/"));
        assert!(ua.contains(std::env::consts::OS));
        assert!(ua.contains(std::env::consts::ARCH));
    }
}
