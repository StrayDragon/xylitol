//! HTTP hook port consumed by dialect plugins (header bag + body seams).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::error::AiBridgeError;

/// Portable header map for provider HTTP hooks.
pub type HeaderBag = serde_json::Map<String, serde_json::Value>;

/// Three-seam HTTP hooks (before headers, before JSON body, after response).
#[async_trait]
pub trait HttpHooks: Send + Sync {
    async fn before_headers(&self, headers: &mut HeaderBag) -> Result<(), AiBridgeError>;
    async fn before_request(&self, model: &str, body: &mut Value) -> Result<(), AiBridgeError>;
    async fn after_response(&self, status: u16, headers: &HeaderBag);
}

/// Run [`HttpHooks::before_headers`] when hooks are present.
pub async fn run_before_headers(
    hooks: &Option<Arc<dyn HttpHooks>>,
    headers: &mut HeaderBag,
) -> Result<(), AiBridgeError> {
    if let Some(h) = hooks {
        h.before_headers(headers).await?;
    }
    Ok(())
}

/// Run [`HttpHooks::before_request`] when hooks are present.
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

/// Run [`HttpHooks::after_response`] when hooks are present.
pub async fn run_after_response(
    hooks: &Option<Arc<dyn HttpHooks>>,
    status: u16,
    headers: &HeaderBag,
) {
    if let Some(h) = hooks {
        h.after_response(status, headers).await;
    }
}
