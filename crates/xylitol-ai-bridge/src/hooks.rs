use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::AiBridgeError;

pub type HeaderBag = serde_json::Map<String, serde_json::Value>;

#[async_trait]
pub trait HttpHooks: Send + Sync {
    async fn before_headers(&self, headers: &mut HeaderBag) -> Result<(), AiBridgeError>;
    async fn before_request(&self, model: &str, body: &mut Value) -> Result<(), AiBridgeError>;
    async fn after_response(&self, status: u16, headers: &HeaderBag);
}

pub async fn run_before_headers(
    hooks: &Option<Arc<dyn HttpHooks>>,
    headers: &mut HeaderBag,
) -> Result<(), AiBridgeError> {
    if let Some(h) = hooks {
        h.before_headers(headers).await?;
    }
    Ok(())
}

pub async fn run_before_request(
    hooks: &Option<Arc<dyn HttpHooks>>,
    model: &str,
    body: &mut Value,
) -> Result<(), AiBridgeError> {
    if let Some(h) = hooks {
        h.before_request(model, body).await?;
    }
    Ok(())
}

pub async fn run_after_response(
    hooks: &Option<Arc<dyn HttpHooks>>,
    status: u16,
    headers: &HeaderBag,
) {
    if let Some(h) = hooks {
        h.after_response(status, headers).await;
    }
}
