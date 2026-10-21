use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use xylitol_official_plugin_provider_openai::hooks::{HeaderBag, HttpHooks};
use xylitol_plugin_sdk::XyError;

use xylitol_official_plugin_hooks::HookDispatcher;
use xylitol_official_plugin_hooks::http::{
    run_after_response, run_before_headers, run_before_request,
};

pub struct DispatcherHttpHooks(pub Arc<HookDispatcher>);

#[async_trait]
impl HttpHooks for DispatcherHttpHooks {
    async fn before_headers(&self, headers: &mut HeaderBag) -> Result<(), XyError> {
        let hooks = Some(self.0.clone());
        run_before_headers(&hooks, headers).await
    }

    async fn before_request(&self, model: &str, body: &mut Value) -> Result<(), XyError> {
        let hooks = Some(self.0.clone());
        run_before_request(&hooks, model, body).await
    }

    async fn after_response(&self, status: u16, headers: &HeaderBag) {
        let hooks = Some(self.0.clone());
        run_after_response(&hooks, status, headers).await;
    }
}

pub fn to_http_hooks(hooks: Option<Arc<HookDispatcher>>) -> Option<Arc<dyn HttpHooks>> {
    hooks.map(|h| Arc::new(DispatcherHttpHooks(h)) as Arc<dyn HttpHooks>)
}
