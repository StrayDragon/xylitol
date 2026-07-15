//! Adapt [`HookDispatcher`] to bridge [`HttpHooks`] (c1030).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use xylitol_ai_bridge::error::AiBridgeError;
use xylitol_ai_bridge::hooks::{HeaderBag, HttpHooks};

use crate::infra::hooks::HookDispatcher;
use crate::infra::hooks::http::{run_after_response, run_before_headers, run_before_request};
use crate::infra::provider::map::to_bridge_error;

/// Wrap a [`HookDispatcher`] as the package's HTTP hooks port.
pub struct DispatcherHttpHooks(pub Arc<HookDispatcher>);

#[async_trait]
impl HttpHooks for DispatcherHttpHooks {
    async fn before_headers(&self, headers: &mut HeaderBag) -> Result<(), AiBridgeError> {
        let hooks = Some(self.0.clone());
        run_before_headers(&hooks, headers)
            .await
            .map_err(to_bridge_error)
    }

    async fn before_request(&self, model: &str, body: &mut Value) -> Result<(), AiBridgeError> {
        let hooks = Some(self.0.clone());
        run_before_request(&hooks, model, body)
            .await
            .map_err(to_bridge_error)
    }

    async fn after_response(&self, status: u16, headers: &HeaderBag) {
        let hooks = Some(self.0.clone());
        run_after_response(&hooks, status, headers).await;
    }
}

pub fn to_http_hooks(hooks: Option<Arc<HookDispatcher>>) -> Option<Arc<dyn HttpHooks>> {
    hooks.map(|h| Arc::new(DispatcherHttpHooks(h)) as Arc<dyn HttpHooks>)
}
